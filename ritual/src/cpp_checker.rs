use crate::cpp_code_generator;
use crate::cpp_code_generator::apply_moc;
use crate::database::CppCheckerEnv;
use crate::database::CppFfiItem;
use crate::database::CppFfiItemKind;
use crate::processor::ProcessorData;
use log::{debug, trace};
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSliceMut;
use ritual_common::cpp_build_config::{CppBuildConfigData, CppBuildPaths};
use ritual_common::cpp_lib_builder::{
    c2r_cmake_vars, BuildType, CppLibBuilder, CppLibBuilderOutput,
};
use ritual_common::errors::{bail, err_msg, Result};
use ritual_common::file_utils::{create_dir_all, create_file, path_to_str, remove_dir_all};
use ritual_common::target::current_target;
use ritual_common::utils::MapIfOk;
use ritual_common::utils::ProgressBar;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;
use std::time::Instant;
use std::{iter, thread};

const PARALLEL_CHUNKS: usize = 64;

fn check_snippets<'a>(
    data: &mut CppCheckerData,
    snippets: impl Iterator<Item = &'a Snippet>,
) -> Result<CppLibBuilderOutput> {
    let mut any_needs_moc = false;
    {
        let mut file = create_file(&data.main_cpp_path)?;
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
    }
    if any_needs_moc && !data.crate_name.starts_with("moqt_") {
        apply_moc(&data.main_cpp_path)?;
    }

    let instant = Instant::now();
    let result = data.builder.run();
    trace!("cpp builder time: {:?}", instant.elapsed());
    result
}

fn snippet_for_item(item: &CppFfiItem, all_items: &[CppFfiItem]) -> Result<Snippet> {
    match &item.kind {
        CppFfiItemKind::Function(cpp_ffi_function) => {
            let item_code = cpp_code_generator::function_implementation(cpp_ffi_function)?;
            let mut needs_moc = false;
            let full_code = if let Some(index) = item.source_ffi_item {
                let source_item = all_items
                    .get(index)
                    .ok_or_else(|| err_msg("ffi item references invalid index"))?;
                match &item.kind {
                    CppFfiItemKind::Function(_) => {}
                    CppFfiItemKind::QtSlotWrapper(_) => needs_moc = true,
                }
                let source_item_code = source_item.source_item_cpp_code()?;
                format!("{}\n{}", source_item_code, item_code)
            } else {
                item_code
            };
            Ok(Snippet::new_global(full_code, needs_moc))
        }
        CppFfiItemKind::QtSlotWrapper(_) => {
            Ok(Snippet::new_global(item.source_item_cpp_code()?, true))
        }
    }
}

struct CppCheckerData {
    main_cpp_path: PathBuf,
    crate_name: String,
    builder: CppLibBuilder,
}

struct CppChecker<'b, 'a: 'b> {
    data: &'b mut ProcessorData<'a>,
    instance_provider: InstanceProvider,
}

#[derive(Debug, Clone, Copy)]
enum SnippetContext {
    Main,
    Global,
}

#[derive(Debug, Clone)]
struct Snippet {
    code: String,
    context: SnippetContext,
    needs_moc: bool,
}

#[derive(Debug, Clone)]
struct SnippetWithIndexes {
    snippet: Snippet,
    ffi_item_index: usize,
    output: Option<CppLibBuilderOutput>,
}

impl Snippet {
    fn new_in_main<S: Into<String>>(code: S, needs_moc: bool) -> Self {
        Snippet {
            code: code.into(),
            context: SnippetContext::Main,
            needs_moc,
        }
    }
    fn new_global<S: Into<String>>(code: S, needs_moc: bool) -> Self {
        Snippet {
            code: code.into(),
            context: SnippetContext::Global,
            needs_moc,
        }
    }
}

struct PreliminaryTest {
    name: String,
    snippet: Snippet,
    expected: bool,
}

impl PreliminaryTest {
    fn new(name: &str, expected: bool, snippet: Snippet) -> Self {
        Self {
            name: name.into(),
            expected,
            snippet,
        }
    }
}

fn binary_check(
    snippets: &mut [SnippetWithIndexes],
    data: &mut CppCheckerData,
    progress_bar: &ProgressBar,
) -> Result<()> {
    if snippets.len() < 3 {
        for snippet in &mut *snippets {
            let output = check_snippets(data, iter::once(&snippet.snippet))?;
            snippet.output = Some(output);
            progress_bar.add(1);
        }
        return Ok(());
    }

    let output = check_snippets(data, snippets.iter().map(|s| &s.snippet))?;
    if let CppLibBuilderOutput::Success = output {
        for snippet in &mut *snippets {
            snippet.output = Some(output.clone());
        }
        progress_bar.add(snippets.len() as u64);
    } else {
        let split_point = snippets.len() / 2;
        let (left, right) = snippets.split_at_mut(split_point);
        binary_check(left, data, progress_bar)?;
        binary_check(right, data, progress_bar)?;
    }
    Ok(())
}

fn check_preliminary_test(data: &mut CppCheckerData, test: &PreliminaryTest) -> Result<()> {
    match check_snippets(data, iter::once(&test.snippet))? {
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

#[derive(Debug, Clone)]
struct InstanceProvider {
    parent_path: PathBuf,
    include_directives: Vec<PathBuf>,
    crate_name: String,
    cpp_build_config: CppBuildConfigData,
    cpp_build_paths: CppBuildPaths,
}

impl InstanceProvider {
    fn get(&self, id: &str) -> Result<CppCheckerData> {
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

        let builder = CppLibBuilder {
            cmake_source_dir: src_path.clone(),
            build_dir: root_path.join("build"),
            install_dir: None,
            num_jobs: Some(1),
            build_type: BuildType::Debug,
            cmake_vars: c2r_cmake_vars(&self.cpp_build_config, &self.cpp_build_paths, None)?,
            capture_output: true,
            skip_cmake: false,
            skip_cmake_after_first_run: true,
        };

        Ok(CppCheckerData {
            builder,
            main_cpp_path: src_path.join("main.cpp"),
            crate_name: self.crate_name.clone(),
        })
    }
}

struct InstanceStorage {
    instances: Arc<Mutex<HashMap<ThreadId, Arc<Mutex<CppCheckerData>>>>>,
    provider: InstanceProvider,
}

impl InstanceStorage {
    fn new(provider: InstanceProvider) -> Self {
        Self {
            provider,
            instances: Default::default(),
        }
    }
    fn current(&self) -> Result<Arc<Mutex<CppCheckerData>>> {
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

impl CppChecker<'_, '_> {
    fn env(&self) -> CppCheckerEnv {
        CppCheckerEnv {
            target: current_target(),
            cpp_library_version: self.data.config.cpp_lib_version().map(|s| s.to_string()),
        }
    }

    fn run(&mut self) -> Result<()> {
        let env = self.env();
        self.data.current_database.add_environment(env.clone());

        let mut snippets = Vec::new();
        for (ffi_item_index, ffi_item) in self.data.current_database.ffi_items().iter().enumerate()
        {
            if ffi_item.checks.has_env(&env) {
                continue;
            }

            match snippet_for_item(ffi_item, self.data.current_database.ffi_items()) {
                Ok(snippet) => {
                    snippets.push(SnippetWithIndexes {
                        ffi_item_index,
                        snippet,
                        output: None,
                    });
                }
                Err(err) => {
                    debug!(
                        "can't create snippet: {}: {:?}",
                        ffi_item.kind.short_text(),
                        err
                    );
                }
            }
        }

        if snippets.is_empty() {
            return Ok(());
        }

        self.run_tests()?;

        let progress_bar = ProgressBar::new(snippets.len() as u64, "Checking items");

        let instances = InstanceStorage::new(self.instance_provider.clone());

        snippets
            .par_chunks_mut(PARALLEL_CHUNKS)
            .map(|chunk| {
                let progress_bar = progress_bar.clone();
                let instance = instances.current()?;
                let mut instance = instance.lock().unwrap();
                binary_check(chunk, &mut instance, &progress_bar)
            })
            .collect::<Result<_>>()?;

        for snippet in snippets {
            let ffi_item = &mut self.data.current_database.ffi_items_mut()[snippet.ffi_item_index];
            let output = snippet.output.unwrap();
            if output.is_success() {
                debug!("success: {}", ffi_item.kind.short_text());
            } else {
                debug!("error: {}: {:?}", ffi_item.kind.short_text(), output);
                trace!("snippet: {:?}", snippet.snippet);
            }
            ffi_item.checks.add(env.clone(), output.is_success());
        }
        Ok(())
    }

    fn run_tests(&mut self) -> Result<()> {
        let positive_tests = &[
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
        ];
        assert!(positive_tests.iter().all(|t| t.expected));

        let mut instance = self.instance_provider.get("tests")?;
        let all_positive_output =
            check_snippets(&mut instance, positive_tests.iter().map(|t| &t.snippet))?;
        if all_positive_output != CppLibBuilderOutput::Success {
            for test in positive_tests {
                check_preliminary_test(&mut instance, test)?;
            }
        }

        let negative_tests = &[
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
        ];
        assert!(negative_tests.iter().all(|t| !t.expected));

        for test in negative_tests {
            check_preliminary_test(&mut instance, test)?;
        }

        Ok(())
    }
}

pub fn run(data: &mut ProcessorData<'_>) -> Result<()> {
    let instance_provider = InstanceProvider {
        parent_path: data.workspace.tmp_path().join("cpp_checker"),
        include_directives: data.config.include_directives().to_vec(),
        crate_name: data.current_database.crate_name().to_string(),
        cpp_build_paths: {
            let mut data = data.config.cpp_build_paths().clone();
            data.apply_env();
            data
        },
        cpp_build_config: data.config.cpp_build_config().eval(&current_target())?,
    };
    let mut checker = CppChecker {
        data,
        instance_provider,
    };

    checker.run()?;

    Ok(())
}
