use crate::config::Config;
use crate::cpp_data::{CppItem, CppPath};
use crate::cpp_ffi_data::{CppFfiFunctionKind, CppFfiItem};
use crate::cpp_type::CppType;
use crate::database::CppFfiDatabaseItem;
use crate::processor::ProcessorData;
use crate::{cluster_api, cpp_code_generator};
use itertools::Itertools;
use log::{debug, trace};
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSliceMut;
use ritual_common::cpp_build_config::{CppBuildConfigData, CppBuildPaths};
use ritual_common::cpp_lib_builder::{
    BuildType, CMakeConfigData, CppLibBuilder, CppLibBuilderOutput,
};
use ritual_common::errors::{bail, err_msg, Result};
use ritual_common::file_utils::{
    create_dir_all, create_file, os_str_to_str, path_to_str, remove_dir_all,
};
use ritual_common::target::{current_target, LibraryTarget};
use ritual_common::utils::{MapIfOk, ProgressBar};
use serde_derive::{Deserialize, Serialize};
use std::collections::{hash_map::Entry, HashMap};
use std::io::Write;
use std::iter::{once, IntoIterator};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;
use std::time::Instant;
use std::{iter, thread};

pub const CHUNK_SIZE: usize = 64;

fn snippet_for_item(
    item: &CppFfiDatabaseItem,
    all_items: &[CppFfiDatabaseItem],
) -> Result<Snippet> {
    match &item.item {
        CppFfiItem::Function(cpp_ffi_function) => {
            let item_code = cpp_code_generator::function_implementation(cpp_ffi_function)?;
            let mut needs_moc = false;
            let full_code = if let Some(index) = item.source_ffi_item {
                let source_item = all_items
                    .get(index)
                    .ok_or_else(|| err_msg("ffi item references invalid index"))?;
                match &source_item.item {
                    CppFfiItem::Function(_) => {}
                    CppFfiItem::QtSlotWrapper(_) => needs_moc = true,
                }
                let source_item_code = source_item.source_item_cpp_code()?;
                format!("{}\n{}", source_item_code, item_code)
            } else {
                item_code
            };
            Ok(Snippet::new_global(full_code, needs_moc))
        }
        CppFfiItem::QtSlotWrapper(_) => Ok(Snippet::new_global(item.source_item_cpp_code()?, true)),
    }
}

pub struct CppCheckerInstance {
    main_cpp_path: PathBuf,
    crate_name: String,
    builder: CppLibBuilder,
    tests: Vec<PreliminaryTest>,
}

impl CppCheckerInstance {
    pub fn check_snippets<'a>(
        &mut self,
        snippets: impl Iterator<Item = &'a Snippet>,
    ) -> Result<CppLibBuilderOutput> {
        let mut any_needs_moc = false;

        let mut file = create_file(&self.main_cpp_path)?;
        writeln!(file, "#include \"utils.h\"")?;
        writeln!(file)?;
        let mut main_content = Vec::new();
        for snippet in snippets {
            if snippet.needs_moc {
                any_needs_moc = true;
            }
            match snippet.context {
                SnippetContext::Main => {
                    main_content.push(&snippet.code);
                }
                SnippetContext::Global => {
                    writeln!(file, "{}", snippet.code)?;
                    writeln!(file)?;
                }
            }
        }

        writeln!(file, "int main() {{")?;
        for item in main_content {
            writeln!(file, "{{")?;
            writeln!(file, "{}", item)?;
            writeln!(file, "}}")?;
        }
        writeln!(file, "}}")?;

        if any_needs_moc && !self.crate_name.starts_with("moqt_") {
            let stem = self
                .main_cpp_path
                .file_stem()
                .ok_or_else(|| err_msg("failed to get file stem"))?;
            writeln!(file, "#include \"{}.moc\"", os_str_to_str(stem)?)?;
        }

        drop(file);

        let instant = Instant::now();
        let result = self.builder.run();
        trace!("cpp builder time: {:?}", instant.elapsed());
        result
    }

    fn check_preliminary_test(&mut self, test: &PreliminaryTest) -> Result<()> {
        match self.check_snippets(iter::once(&test.snippet))? {
            CppLibBuilderOutput::Success => {
                if !test.expected {
                    bail!("Nevative test ({}) succeeded", test.name);
                }
            }
            CppLibBuilderOutput::Fail(output) => {
                if test.expected {
                    bail!("Positive test ({}) failed: {}", test.name, output.stderr);
                }
            }
        }
        Ok(())
    }

    pub fn check_preliminary_tests(&mut self) -> Result<()> {
        let positive_tests = self
            .tests
            .iter()
            .filter(|test| test.expected)
            .cloned()
            .collect_vec();

        let all_positive_output = self.check_snippets(positive_tests.iter().map(|t| &t.snippet))?;
        if all_positive_output != CppLibBuilderOutput::Success {
            for test in &positive_tests {
                self.check_preliminary_test(test)?;
            }
        }

        let negative_tests = self
            .tests
            .iter()
            .filter(|test| !test.expected)
            .cloned()
            .collect_vec();

        for test in &negative_tests {
            self.check_preliminary_test(test)?;
        }
        Ok(())
    }

    pub fn binary_check<T>(
        &mut self,
        snippets: &mut [SnippetTask<T>],
        progress_bar: Option<&ProgressBar>,
    ) -> Result<()> {
        if snippets.len() < 3 {
            for snippet in &mut *snippets {
                let output = self.check_snippets(iter::once(&snippet.snippet))?;
                snippet.output = Some(output);
                if let Some(progress_bar) = progress_bar {
                    progress_bar.add(1);
                }
            }
            return Ok(());
        }

        let output = self.check_snippets(snippets.iter().map(|s| &s.snippet))?;
        if let CppLibBuilderOutput::Success = output {
            for snippet in &mut *snippets {
                snippet.output = Some(output.clone());
            }
            if let Some(progress_bar) = progress_bar {
                progress_bar.add(snippets.len() as u64);
            }
        } else {
            let split_point = snippets.len() / 2;
            let (left, right) = snippets.split_at_mut(split_point);
            self.binary_check(left, progress_bar)?;
            self.binary_check(right, progress_bar)?;
        }
        Ok(())
    }
}

struct CppChecker<'b, 'a: 'b> {
    data: &'b mut ProcessorData<'a>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
enum SnippetContext {
    Main,
    Global,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Snippet {
    code: String,
    context: SnippetContext,
    needs_moc: bool,
}

#[derive(Debug, Clone)]
pub struct SnippetTask<T> {
    pub snippet: Snippet,
    pub output: Option<CppLibBuilderOutput>,
    pub data: T,
}

pub struct SnippetTaskLocalData {
    pub ffi_item_index: usize,
    pub crate_name: String,
    pub library_target: LibraryTarget,
}

pub type LocalSnippetTask = SnippetTask<SnippetTaskLocalData>;

impl Snippet {
    pub fn new_in_main<S: Into<String>>(code: S, needs_moc: bool) -> Self {
        Snippet {
            code: code.into(),
            context: SnippetContext::Main,
            needs_moc,
        }
    }

    pub fn new_global<S: Into<String>>(code: S, needs_moc: bool) -> Self {
        Snippet {
            code: code.into(),
            context: SnippetContext::Global,
            needs_moc,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreliminaryTest {
    name: String,
    snippet: Snippet,
    expected: bool,
}

impl PreliminaryTest {
    pub fn new(name: &str, expected: bool, snippet: Snippet) -> Self {
        Self {
            name: name.into(),
            expected,
            snippet,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalCppChecker {
    parent_path: PathBuf,
    include_directives: Vec<PathBuf>,
    crate_name: String,
    cpp_build_config: CppBuildConfigData,
    cpp_build_paths: CppBuildPaths,
    tests: Vec<PreliminaryTest>,
}

impl LocalCppChecker {
    pub fn new(parent_path: impl Into<PathBuf>, config: &Config) -> Result<LocalCppChecker> {
        let mut tests = builtin_tests();
        tests.extend(config.cpp_checker_tests().iter().cloned());

        Ok(LocalCppChecker {
            parent_path: parent_path.into(),
            include_directives: config.include_directives().to_vec(),
            crate_name: config.crate_properties().name().to_string(),
            cpp_build_paths: {
                let mut data = config.cpp_build_paths().clone();
                data.apply_env();
                data
            },
            cpp_build_config: config.cpp_build_config().eval(&current_target())?,
            tests,
        })
    }

    pub fn get(&self, id: &str) -> Result<CppCheckerInstance> {
        let root_path = self.parent_path.join(id);
        if root_path.exists() {
            remove_dir_all(&root_path)?;
        }
        let src_path = root_path.join("src");
        create_dir_all(&src_path)?;

        let mut cmake_file = create_file(src_path.join("CMakeLists.txt"))?;
        write!(
            cmake_file,
            "{}",
            include_str!("../templates/cpp_checker/CMakeLists.txt")
        )?;

        let mut utils_file = create_file(src_path.join("utils.h"))?;
        write!(
            utils_file,
            include_str!("../templates/cpp_checker/utils.h"),
            include_directives_code = self
                .include_directives
                .iter()
                .map_if_ok(|d| -> Result<_> { Ok(format!("#include \"{}\"", path_to_str(d)?)) })?
                .join("\n")
        )?;

        let cmake_config = CMakeConfigData {
            cpp_build_config_data: &self.cpp_build_config,
            cpp_build_paths: &self.cpp_build_paths,
            library_type: None,
            cpp_library_version: None,
        };

        let builder = CppLibBuilder {
            cmake_source_dir: src_path.clone(),
            build_dir: root_path.join("build"),
            install_dir: None,
            num_jobs: Some(1),
            build_type: BuildType::Debug,
            cmake_vars: cmake_config.cmake_vars()?,
            capture_output: true,
            skip_cmake: false,
            skip_cmake_after_first_run: true,
        };

        Ok(CppCheckerInstance {
            builder,
            main_cpp_path: src_path.join("main.cpp"),
            crate_name: self.crate_name.clone(),
            tests: self.tests.clone(),
        })
    }
}

struct InstanceStorage {
    instances: Arc<Mutex<HashMap<ThreadId, Arc<Mutex<CppCheckerInstance>>>>>,
    provider: LocalCppChecker,
}

impl InstanceStorage {
    fn new(provider: LocalCppChecker) -> Self {
        Self {
            provider,
            instances: Default::default(),
        }
    }
    fn current(&self) -> Result<Arc<Mutex<CppCheckerInstance>>> {
        let mut instances = self.instances.lock().unwrap();
        let instances_len = instances.len();
        let instance = match instances.entry(thread::current().id()) {
            Entry::Vacant(entry) => {
                let instance = self.provider.get(&format!("main_{}", instances_len))?;
                entry.insert(Arc::new(Mutex::new(instance)))
            }
            Entry::Occupied(entry) => entry.into_mut(),
        };
        Ok(Arc::clone(instance))
    }
}

fn builtin_tests() -> Vec<PreliminaryTest> {
    vec![
        PreliminaryTest::new(
            "hello world",
            true,
            Snippet::new_in_main("std::cout << \"Hello world\\n\";", false),
        ),
        PreliminaryTest::new(
            "correct assertion",
            true,
            Snippet::new_in_main("ritual_assert(2 + 2 == 4);", false),
        ),
        PreliminaryTest::new(
            "type traits",
            true,
            Snippet::new_in_main(
                "\
                 class C1 {}; \n\
                 enum E1 {};  \n\
                 ritual_assert(std::is_class<C1>::value); \n\
                 ritual_assert(!std::is_class<E1>::value); \n\
                 ritual_assert(!std::is_enum<C1>::value); \n\
                 ritual_assert(std::is_enum<E1>::value); \
                 ritual_assert(sizeof(C1) > 0);\
                 ritual_assert(sizeof(E1) > 0);\n\
                 ",
                false,
            ),
        ),
        PreliminaryTest::new(
            "incorrect assertion in fn",
            true,
            Snippet::new_global("int f1() { ritual_assert(2 + 2 == 5); return 1; }", false),
        ),
        PreliminaryTest::new("syntax error", false, Snippet::new_in_main("}", false)),
        PreliminaryTest::new(
            "incorrect assertion",
            false,
            Snippet::new_in_main("ritual_assert(2 + 2 == 5);", false),
        ),
        PreliminaryTest::new(
            "status code 1",
            false,
            Snippet::new_in_main("return 1;", false),
        ),
    ]
}

impl CppChecker<'_, '_> {
    fn env(&self) -> LibraryTarget {
        LibraryTarget {
            target: current_target(),
            cpp_library_version: self.data.config.cpp_lib_version().map(ToString::to_string),
        }
    }

    fn run(&mut self) -> Result<()> {
        if self.data.config.cluster_config().is_some() {
            self.run_cluster()
        } else {
            self.run_local()
        }
    }

    fn run_cluster(&mut self) -> Result<()> {
        let cluster_config = self.data.config.cluster_config().unwrap();
        let crate_name = self.data.current_database.crate_name().to_string();

        let environments = cluster_config
            .workers
            .iter()
            .flat_map(|worker| {
                worker
                    .libraries
                    .iter()
                    .filter(|lib| lib.crate_name == crate_name)
                    .map(move |lib| LibraryTarget {
                        target: worker.target.clone(),
                        cpp_library_version: lib.lib_version.clone(),
                    })
            })
            .collect_vec();

        for env in &environments {
            self.data.current_database.add_environment(env.clone());
        }

        let mut snippets = self.create_tasks(&environments);
        if snippets.is_empty() {
            return Ok(());
        }

        cluster_api::run_checks(cluster_config, &mut snippets)?;

        self.save_results(snippets);

        Ok(())
    }

    fn run_local(&mut self) -> Result<()> {
        let instance_provider = LocalCppChecker::new(
            self.data.workspace.tmp_path().join("cpp_checker"),
            &self.data.config,
        )?;

        let env = self.env();

        self.data.current_database.add_environment(env.clone());

        let mut snippets = self.create_tasks(&[env]);
        if snippets.is_empty() {
            return Ok(());
        }

        let mut instance = instance_provider.get("tests")?;
        instance.check_preliminary_tests()?;

        let progress_bar = ProgressBar::new(snippets.len() as u64, "Checking items");

        let instances = InstanceStorage::new(instance_provider.clone());

        snippets
            .par_chunks_mut(CHUNK_SIZE)
            .map(|chunk| {
                let progress_bar = progress_bar.clone();
                let instance = instances.current()?;
                let mut instance = instance.lock().unwrap();
                instance.binary_check(chunk, Some(&progress_bar))
            })
            .collect::<Result<_>>()?;
        self.save_results(snippets);

        Ok(())
    }

    fn create_tasks(&self, library_targets: &[LibraryTarget]) -> Vec<LocalSnippetTask> {
        let crate_name = self.data.current_database.crate_name().to_string();

        let mut snippets = Vec::new();
        for (ffi_item_index, ffi_item) in self.data.current_database.ffi_items().iter().enumerate()
        {
            if ffi_item.checks.has_all_envs(library_targets) {
                continue;
            }

            match snippet_for_item(ffi_item, self.data.current_database.ffi_items()) {
                Ok(snippet) => {
                    for library_target in library_targets {
                        if ffi_item.checks.has_env(library_target) {
                            continue;
                        }
                        snippets.push(SnippetTask {
                            data: SnippetTaskLocalData {
                                ffi_item_index,
                                crate_name: crate_name.clone(),
                                library_target: library_target.clone(),
                            },
                            snippet: snippet.clone(),
                            output: None,
                        });
                    }
                }
                Err(err) => {
                    debug!(
                        "can't create snippet: {}: {:?}",
                        ffi_item.item.short_text(),
                        err
                    );
                }
            }
        }
        snippets
    }

    fn save_results(&mut self, snippets: Vec<LocalSnippetTask>) {
        for snippet in snippets {
            let ffi_item =
                &mut self.data.current_database.ffi_items_mut()[snippet.data.ffi_item_index];
            if let Some(output) = snippet.output {
                if output.is_success() {
                    debug!("success: {}", ffi_item.item.short_text());
                } else {
                    debug!("error: {}: {:?}", ffi_item.item.short_text(), output);
                }
                ffi_item
                    .checks
                    .add(snippet.data.library_target, output.is_success());
            } else {
                debug!("no output for item: {}", ffi_item.item.short_text());
            }
            trace!("snippet: {:?}", snippet.snippet);
        }
    }
}

pub fn run(data: &mut ProcessorData<'_>) -> Result<()> {
    let mut checker = CppChecker { data };
    checker.run()?;
    Ok(())
}

fn type_paths(type1: &CppType) -> Vec<&CppPath> {
    match type1 {
        CppType::Void
        | CppType::BuiltInNumeric(_)
        | CppType::SpecificNumeric(_)
        | CppType::PointerSizedInteger { .. }
        | CppType::TemplateParameter { .. } => Vec::new(),
        CppType::Enum { path } | CppType::Class(path) => vec![path],
        CppType::FunctionPointer(function) => function
            .arguments
            .iter()
            .chain(once(&*function.return_type))
            .flat_map(|type1| type_paths(type1))
            .collect(),
        CppType::PointerLike { target, .. } => type_paths(target),
    }
}

pub fn apply_blacklist_to_checks(data: &mut ProcessorData<'_>) -> Result<()> {
    if let Some(hook) = data.config.ffi_generator_hook() {
        for item in data.current_database.ffi_items_mut() {
            if !item.checks.any_success() {
                continue;
            }
            let allowed = if let CppFfiItem::Function(function) = &item.item {
                match &function.kind {
                    CppFfiFunctionKind::Function { cpp_function, .. } => {
                        hook(&CppItem::Function(cpp_function.clone()))?
                    }
                    CppFfiFunctionKind::FieldAccessor { field, .. } => {
                        hook(&CppItem::ClassField(field.clone()))?
                    }
                }
            } else {
                true
            };
            if !allowed {
                log::info!("Checks are cleared for {}", item.item.short_text());
                item.checks.clear();
            }
        }
    }

    if let Some(hook) = data.config.cpp_parser_path_hook() {
        for item in data.current_database.ffi_items_mut() {
            if !item.checks.any_success() {
                continue;
            }
            let (types, path) = match &item.item {
                CppFfiItem::Function(function) => match &function.kind {
                    CppFfiFunctionKind::Function { cpp_function, .. } => (
                        cpp_function.all_involved_types(),
                        Some(cpp_function.path.clone()),
                    ),
                    CppFfiFunctionKind::FieldAccessor { field, .. } => {
                        (vec![field.field_type.clone()], Some(field.path.clone()))
                    }
                },
                CppFfiItem::QtSlotWrapper(wrapper) => (wrapper.signal_arguments.clone(), None),
            };

            let any_blocked = types
                .iter()
                .flat_map(|type1| type_paths(type1))
                .chain(path.as_ref())
                .map_if_ok(|path| hook(path))?
                .into_iter()
                .any(|x| !x);

            if any_blocked {
                log::info!("Checks are cleared for {}", item.item.short_text());
                item.checks.clear();
            }
        }
    }
    Ok(())
}
