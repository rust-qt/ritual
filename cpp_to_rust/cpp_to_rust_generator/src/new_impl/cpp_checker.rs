use common::cpp_lib_builder::{BuildType, CppLibBuilder, CppLibBuilderOutput, c2r_cmake_vars};
use common::errors::{ChainErr, Result};
use common::file_utils::PathBufWithAdded;
use common::file_utils::{create_dir_all, create_file, path_to_str, remove_dir_all};
use common::log;
use common::target::current_target;
use common::utils::MapIfOk;
use config::Config;
use cpp_data::CppTypeData;
use cpp_ffi_data::CppFfiMethod;
use cpp_type::CppTypeClassBase;
use new_impl::database::CppCheckerAddResult;
use new_impl::database::CppCheckerInfo;
use new_impl::database::CppItemData;
use new_impl::database::{CppCheckerEnv, Database};
use new_impl::html_logger::HtmlLogger;
use new_impl::processor::ProcessorData;
use new_impl::processor::ProcessorItem;
use new_impl::workspace::Workspace;
use std::fmt::Display;
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

fn snippet_for_method(method: &CppFfiMethod) -> Result<Snippet> {
  unimplemented!()
}

struct CppChecker<'a> {
  data: ProcessorData<'a>,
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

impl<'a> CppChecker<'a> {
  fn run(&mut self) -> Result<()> {
    if !self
      .data
      .current_database
      .environments
      .iter()
      .any(|e| e == &self.env)
    {
      self
        .data
        .current_database
        .environments
        .push(self.env.clone());
    }

    self.data.html_logger.add_header(&["Item", "Status"])?;
    self.run_tests()?;

    let total_count = self.data.current_database.items.len();
    for (index, item) in self.data.current_database.items.iter_mut().enumerate() {
      for ffi_method in &mut item.cpp_ffi_methods {
        if let Ok(snippet) = snippet_for_method(ffi_method) {
          log::status(format!("Checking item {} / {}", index + 1, total_count));

          let error_data = match check_snippet(&self.main_cpp_path, &self.builder, &snippet)? {
            CppLibBuilderOutput::Success => None, // no error
            CppLibBuilderOutput::Fail(output) => Some(format!("build failed: {}", output.stderr)),
          };
          let error_data_text = CppCheckerInfo::error_to_log(&error_data);
          let r = ffi_method.checks.add(&self.env, error_data);
          let change_text = match r {
            CppCheckerAddResult::Added => "Added".to_string(),
            CppCheckerAddResult::Unchanged => "Unchanged".to_string(),
            CppCheckerAddResult::Changed { ref old } => format!(
              "Changed! Old data for the same env: {}",
              CppCheckerInfo::error_to_log(old)
            ),
          };

          self.data.html_logger.add(
            &[
              ffi_method.short_text(),
              format!("{}<br>{}", error_data_text, change_text),
            ],
            "cpp_checker_update",
          )?;
        }
      }
    }

    Ok(())
  }

  fn run_tests(&mut self) -> Result<()> {
    self.check_preliminary_test(
      "hello world",
      &Snippet::new_in_main("std::cout << \"Hello world\\n\";"),
      true,
    )?;
    self.builder.skip_cmake = true;
    self.check_preliminary_test(
      "correct assertion",
      &Snippet::new_in_main("assert(2 + 2 == 4);"),
      true,
    )?;
    self.check_preliminary_test(
      "type traits",
      &Snippet::new_in_main(
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
      true,
    )?;
    self.check_preliminary_test(
      "incorrect assertion in fn",
      &Snippet::new_global("int f1() { assert(2 + 2 == 5); return 1; }"),
      true,
    )?;

    self.check_preliminary_test("syntax error", &Snippet::new_in_main("}"), false)?;
    self.check_preliminary_test(
      "incorrect assertion",
      &Snippet::new_in_main("assert(2 + 2 == 5)"),
      false,
    )?;
    self.check_preliminary_test("status code 1", &Snippet::new_in_main("return 1;"), false)?;
    Ok(())
  }

  fn check_preliminary_test(&self, name: &str, snippet: &Snippet, expected: bool) -> Result<()> {
    log::status(format!("Running preliminary test: {}", name));
    match self.check_snippet(snippet)? {
      CppLibBuilderOutput::Success => {
        if !expected {
          return Err(format!("Nevative test ({}) succeeded", name).into());
        }
      }
      CppLibBuilderOutput::Fail(output) => {
        if expected {
          return Err(format!("Positive test ({}) failed: {}", name, output.stderr).into());
        }
      }
    }
    Ok(())
  }

  fn check_snippet(&self, snippet: &Snippet) -> Result<CppLibBuilderOutput> {
    check_snippet(&self.main_cpp_path, &self.builder, snippet)
  }
}

fn run(data: ProcessorData) -> Result<()> {
  let root_path = data.workspace.tmp_path()?.with_added("cpp_checker");
  if root_path.exists() {
    remove_dir_all(&root_path)?;
  }
  let src_path = root_path.with_added("src");
  create_dir_all(&src_path)?;
  create_file(src_path.with_added("CMakeLists.txt"))?
    .write(include_str!("../../templates/cpp_checker/CMakeLists.txt"))?;
  create_file(src_path.with_added("utils.h"))?.write(format!(
    include_str!("../../templates/cpp_checker/utils.h"),
    include_directives_code = data
      .config
      .include_directives()
      .map_if_ok(|d| -> Result<_> { Ok(format!("#include \"{}\"", path_to_str(d)?)) })?
      .join("\n")
  ))?;

  let builder = CppLibBuilder {
    cmake_source_dir: src_path.clone(),
    build_dir: root_path.with_added("build"),
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
    main_cpp_path: src_path.with_added("main.cpp"),
    env,
  };

  checker.run()?;

  Ok(())
}

pub fn cpp_checker() -> ProcessorItem {
  ProcessorItem {
    name: "cpp_checker".to_string(),
    is_main: true,
    run_after: vec!["cpp_parser".to_string(), "cpp_ffi_generator".to_string()],
    function: run,
  }
}
