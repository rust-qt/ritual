use crate::cpp_code_generator;
use crate::database::CppCheckerAddResult;
use crate::database::CppCheckerEnv;
use crate::database::CppFfiItem;
use crate::database::CppFfiItemKind;
use crate::processor::ProcessingStep;
use crate::processor::ProcessorData;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use log::debug;
use ritual_common::cpp_lib_builder::{
    c2r_cmake_vars, BuildType, CppLibBuilder, CppLibBuilderOutput,
};
use ritual_common::errors::{bail, Result};
use ritual_common::file_utils::{create_dir_all, create_file, path_to_str, remove_dir_all};
use ritual_common::target::current_target;
use ritual_common::utils::MapIfOk;
use std::path::Path;
use std::path::PathBuf;

fn check_snippet(
    main_cpp_path: &Path,
    builder: &CppLibBuilder,
    snippet: &Snippet,
) -> Result<CppLibBuilderOutput> {
    {
        let mut file = create_file(main_cpp_path)?;
        file.write("#include \"utils.h\"\n\n")?;
        match snippet.context {
            SnippetContext::Main => {
                file.write(format!("int main() {{\n{}\n}}\n", snippet.code))?;
            }
            SnippetContext::Global => {
                file.write(format!("{}\n\nint main() {{}}\n", snippet.code))?;
            }
        }
    }
    builder.run()
}

#[allow(unused_variables)]
fn snippet_for_item(item: &CppFfiItem) -> Result<Snippet> {
    match item.kind {
        CppFfiItemKind::Function(ref cpp_ffi_function) => Ok(Snippet::new_global(
            cpp_code_generator::function_implementation(cpp_ffi_function)?,
        )),
        CppFfiItemKind::QtSlotWrapper(ref qt_slot_wrapper) => {
            bail!("qt slot wrappers are not supported yet");
        }
    }
}

struct CppChecker<'b, 'a: 'b> {
    data: &'b mut ProcessorData<'a>,
    env: CppCheckerEnv,
    main_cpp_path: PathBuf,
    builder: CppLibBuilder,
}

enum SnippetContext {
    Main,
    Global,
}
struct Snippet {
    code: String,
    context: SnippetContext,
}

impl Snippet {
    fn new_in_main<S: Into<String>>(code: S) -> Snippet {
        Snippet {
            code: code.into(),
            context: SnippetContext::Main,
        }
    }
    fn new_global<S: Into<String>>(code: S) -> Snippet {
        Snippet {
            code: code.into(),
            context: SnippetContext::Global,
        }
    }
}

fn new_progress_bar(count: u64, message: &str) -> ProgressBar {
    let progress_bar = ProgressBar::new(count);
    progress_bar.set_style(ProgressStyle::default_bar().template("{pos}/{len} {msg} {wide_bar}"));
    progress_bar.set_message(message);
    progress_bar
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

impl CppChecker<'_, '_> {
    fn run(&mut self) -> Result<()> {
        if !self
            .data
            .current_database
            .environments
            .iter()
            .any(|e| e == &self.env)
        {
            self.data
                .current_database
                .environments
                .push(self.env.clone());
        }

        self.run_tests()?;

        let total_count = self.data.current_database.cpp_items.len();
        let progress_bar = new_progress_bar(total_count as u64, "Checking items");

        for item in &mut self.data.current_database.cpp_items {
            progress_bar.inc(1);
            for ffi_item in &mut item.ffi_items {
                if let Ok(snippet) = snippet_for_item(ffi_item) {
                    //info!("Checking item {} / {}", index + 1, total_count);

                    let error_data =
                        match check_snippet(&self.main_cpp_path, &self.builder, &snippet)? {
                            CppLibBuilderOutput::Success => None, // no error
                            CppLibBuilderOutput::Fail(output) => {
                                Some(format!("build failed: {}", output.stderr))
                            }
                        };
                    let r = ffi_item.checks.add(&self.env, error_data.clone());
                    let change_text = match r {
                        CppCheckerAddResult::Added => "Added".to_string(),
                        CppCheckerAddResult::Unchanged => "Unchanged".to_string(),
                        CppCheckerAddResult::Changed { ref old } => {
                            format!("Changed! Old data for the same env: {:?}", old)
                        }
                    };

                    debug!(
                        "[cpp_checker_update] ffi_item = {:?}; snippet = {:?}; error = {:?}; {}",
                        ffi_item, snippet.code, error_data, change_text
                    );
                }
            }
        }

        Ok(())
    }

    fn run_tests(&mut self) -> Result<()> {
        let tests = &[
            PreliminaryTest::new(
                "hello world",
                true,
                Snippet::new_in_main("std::cout << \"Hello world\\n\";"),
            ),
            PreliminaryTest::new(
                "correct assertion",
                true,
                Snippet::new_in_main("assert(2 + 2 == 4);"),
            ),
            PreliminaryTest::new(
                "type traits",
                true,
                Snippet::new_in_main(
                    "\
                     class C1 {}; \n\
                     enum E1 {};  \n\
                     assert(std::is_class<C1>::value); \n\
                     assert(!std::is_class<E1>::value); \n\
                     assert(!std::is_enum<C1>::value); \n\
                     assert(std::is_enum<E1>::value); \
                     assert(sizeof(C1) > 0);\
                     assert(sizeof(E1) > 0);\n\
                     ",
                ),
            ),
            PreliminaryTest::new(
                "incorrect assertion in fn",
                true,
                Snippet::new_global("int f1() { assert(2 + 2 == 5); return 1; }"),
            ),
            PreliminaryTest::new("syntax error", false, Snippet::new_in_main("}")),
            PreliminaryTest::new(
                "incorrect assertion",
                false,
                Snippet::new_in_main("assert(2 + 2 == 5)"),
            ),
            PreliminaryTest::new("status code 1", false, Snippet::new_in_main("return 1;")),
        ];

        let progress_bar = new_progress_bar(tests.len() as u64, "Running preliminary tests");
        for test in tests {
            progress_bar.inc(1);
            self.check_preliminary_test(test)?;
            self.builder.skip_cmake = true;
        }

        Ok(())
    }

    fn check_preliminary_test(&self, test: &PreliminaryTest) -> Result<()> {
        match self.check_snippet(&test.snippet)? {
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

    fn check_snippet(&self, snippet: &Snippet) -> Result<CppLibBuilderOutput> {
        check_snippet(&self.main_cpp_path, &self.builder, snippet)
    }
}

fn run(data: &mut ProcessorData) -> Result<()> {
    let root_path = data.workspace.tmp_path()?.join("cpp_checker");
    if root_path.exists() {
        remove_dir_all(&root_path)?;
    }
    let src_path = root_path.join("src");
    create_dir_all(&src_path)?;
    create_file(src_path.join("CMakeLists.txt"))?
        .write(include_str!("../templates/cpp_checker/CMakeLists.txt"))?;
    create_file(src_path.join("utils.h"))?.write(format!(
        include_str!("../templates/cpp_checker/utils.h"),
        include_directives_code = data
            .config
            .include_directives()
            .map_if_ok(|d| -> Result<_> { Ok(format!("#include \"{}\"", path_to_str(d)?)) })?
            .join("\n")
    ))?;

    let builder = CppLibBuilder {
        cmake_source_dir: src_path.clone(),
        build_dir: root_path.join("build"),
        install_dir: None,
        num_jobs: Some(1),
        build_type: BuildType::Debug,
        cmake_vars: c2r_cmake_vars(
            &data.config.cpp_build_config().eval(&current_target())?,
            data.config.cpp_build_paths(),
            None,
        )?,
        capture_output: true,
        skip_cmake: false,
    };

    let env = CppCheckerEnv {
        target: current_target(),
        cpp_library_version: data.config.cpp_lib_version().map(|s| s.to_string()),
    };
    let mut checker = CppChecker {
        data,
        builder,
        main_cpp_path: src_path.join("main.cpp"),
        env,
    };

    checker.run()?;

    Ok(())
}

pub fn cpp_checker_step() -> ProcessingStep {
    ProcessingStep::new("cpp_checker", run)
}
