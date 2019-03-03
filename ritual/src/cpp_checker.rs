use crate::cpp_code_generator;
use crate::database::CppCheckerEnv;
use crate::database::CppFfiItem;
use crate::database::CppFfiItemKind;
use crate::processor::ProcessingStep;
use crate::processor::ProcessorData;
use log::{debug, trace};
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSliceMut;
use ritual_common::cpp_lib_builder::{
    c2r_cmake_vars, BuildType, CppLibBuilder, CppLibBuilderOutput,
};
use ritual_common::errors::{bail, Result};
use ritual_common::file_utils::{create_dir_all, create_file, path_to_str, remove_dir_all};
use ritual_common::target::current_target;
use ritual_common::utils::MapIfOk;
use ritual_common::utils::ProgressBar;
use std::io::Write;
use std::iter;
use std::path::PathBuf;
use std::time::Instant;

fn check_snippets<'a>(
    data: &mut CppCheckerData,
    snippets: impl Iterator<Item = &'a Snippet>,
) -> Result<CppLibBuilderOutput> {
    {
        let mut file = create_file(&data.main_cpp_path)?;
        writeln!(file, "#include \"utils.h\"")?;
        writeln!(file)?;
        let mut main_content = Vec::new();
        for snippet in snippets {
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
    let instant = Instant::now();
    let result = data.builder.run();
    trace!("cpp builder time: {:?}", instant.elapsed());
    result
}

fn snippet_for_item(item: &CppFfiItem) -> Result<Snippet> {
    match &item.kind {
        CppFfiItemKind::Function(cpp_ffi_function) => Ok(Snippet::new_global(
            cpp_code_generator::function_implementation(cpp_ffi_function)?,
        )),
        CppFfiItemKind::QtSlotWrapper(_qt_slot_wrapper) => {
            bail!("qt slot wrappers are not supported yet");
        }
    }
}

struct CppCheckerData {
    main_cpp_path: PathBuf,
    builder: CppLibBuilder,
}

struct CppChecker<'b, 'a: 'b> {
    data: &'b mut ProcessorData<'a>,
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
}

#[derive(Debug, Clone)]
struct SnippetWithIndexes {
    snippet: Snippet,
    ffi_item_index: usize,
    output: Option<CppLibBuilderOutput>,
}

impl Snippet {
    fn new_in_main<S: Into<String>>(code: S) -> Self {
        Snippet {
            code: code.into(),
            context: SnippetContext::Main,
        }
    }
    fn new_global<S: Into<String>>(code: S) -> Self {
        Snippet {
            code: code.into(),
            context: SnippetContext::Global,
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

impl CppChecker<'_, '_> {
    fn env(&self) -> CppCheckerEnv {
        CppCheckerEnv {
            target: current_target(),
            cpp_library_version: self.data.config.cpp_lib_version().map(|s| s.to_string()),
        }
    }
    fn create_instance(&self, id: &str) -> Result<CppCheckerData> {
        let root_path = self.data.workspace.tmp_path().join("cpp_checker").join(id);
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
                .data
                .config
                .include_directives()
                .map_if_ok(|d| -> Result<_> { Ok(format!("#include \"{}\"", path_to_str(d)?)) })?
                .join("\n")
        )?;

        let mut build_paths = self.data.config.cpp_build_paths().clone();
        build_paths.apply_env();
        let builder = CppLibBuilder {
            cmake_source_dir: src_path.clone(),
            build_dir: root_path.join("build"),
            install_dir: None,
            num_jobs: Some(1),
            build_type: BuildType::Debug,
            cmake_vars: c2r_cmake_vars(
                &self
                    .data
                    .config
                    .cpp_build_config()
                    .eval(&current_target())?,
                &build_paths,
                None,
            )?,
            capture_output: true,
            skip_cmake: false,
            skip_cmake_after_first_run: true,
        };

        Ok(CppCheckerData {
            builder,
            main_cpp_path: src_path.join("main.cpp"),
        })
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

            if let Ok(snippet) = snippet_for_item(ffi_item) {
                snippets.push(SnippetWithIndexes {
                    ffi_item_index,
                    snippet,
                    output: None,
                });
            }
        }

        if snippets.is_empty() {
            return Ok(());
        }

        self.run_tests()?;

        let progress_bar = ProgressBar::new(snippets.len() as u64, "Checking items");
        let num_threads = num_cpus::get();
        let div_ceil = |x, y| (x + y - 1) / y;
        let chunk_size = div_ceil(snippets.len(), num_threads);
        let num_chunks = div_ceil(snippets.len(), chunk_size);

        let instances =
            (0..num_chunks).map_if_ok(|index| self.create_instance(&format!("main_{}", index)))?;

        snippets
            .par_chunks_mut(chunk_size)
            .zip_eq(instances.into_par_iter())
            .map(|(chunk, mut instance)| {
                let progress_bar = progress_bar.clone();
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
                Snippet::new_in_main("std::cout << \"Hello world\\n\";"),
            ),
            PreliminaryTest::new(
                "correct assertion",
                true,
                Snippet::new_in_main("ritual_assert(2 + 2 == 4);"),
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
                ),
            ),
            PreliminaryTest::new(
                "incorrect assertion in fn",
                true,
                Snippet::new_global("int f1() { ritual_assert(2 + 2 == 5); return 1; }"),
            ),
        ];
        assert!(positive_tests.iter().all(|t| t.expected));

        let mut instance = self.create_instance("tests")?;
        let all_positive_output =
            check_snippets(&mut instance, positive_tests.iter().map(|t| &t.snippet))?;
        if all_positive_output != CppLibBuilderOutput::Success {
            for test in positive_tests {
                check_preliminary_test(&mut instance, test)?;
            }
        }

        let negative_tests = &[
            PreliminaryTest::new("syntax error", false, Snippet::new_in_main("}")),
            PreliminaryTest::new(
                "incorrect assertion",
                false,
                Snippet::new_in_main("ritual_assert(2 + 2 == 5);"),
            ),
            PreliminaryTest::new("status code 1", false, Snippet::new_in_main("return 1;")),
        ];
        assert!(negative_tests.iter().all(|t| !t.expected));

        for test in negative_tests {
            check_preliminary_test(&mut instance, test)?;
        }

        Ok(())
    }
}

fn run(data: &mut ProcessorData<'_>) -> Result<()> {
    let mut checker = CppChecker { data };

    checker.run()?;

    Ok(())
}

pub fn cpp_checker_step() -> ProcessingStep {
    ProcessingStep::new("cpp_checker", run)
}
