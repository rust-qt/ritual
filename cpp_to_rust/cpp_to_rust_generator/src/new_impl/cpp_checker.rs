use common::errors::{ChainErr, Result};
use config::Config;
use new_impl::workspace::Workspace;
use common::file_utils::{create_dir_all, create_file, path_to_str, remove_dir_all};
use common::file_utils::PathBufWithAdded;
use common::utils::MapIfOk;
use common::cpp_lib_builder::{BuildType, CppLibBuilder, CppLibBuilderOutput, c2r_cmake_vars};
use common::target::current_target;
use std::path::PathBuf;
use std::fmt::Display;
use common::log;

struct CppChecker<'a> {
  workspace: &'a mut Workspace,
  config: &'a Config,
  main_cpp_path: PathBuf,
  builder: CppLibBuilder,
}

enum CheckContext {
  Main,
  Global,
}

impl<'a> CppChecker<'a> {
  fn run(&mut self) -> Result<()> {
    self.check_preliminary_test(
      "hello world",
      "std::cout << \"Hello world\\n\";",
      CheckContext::Main,
      true,
    )?;
    self.check_preliminary_test(
      "correct assertion",
      "assert(2 + 2 == 4);",
      CheckContext::Main,
      true,
    )?;
    self.check_preliminary_test(
      "incorrect assertion in fn",
      "int f1() { assert(2 + 2 == 5); return 1; }",
      CheckContext::Global,
      true,
    )?;

    self.check_preliminary_test("syntax error", "}", CheckContext::Main, false)?;
    self.check_preliminary_test(
      "incorrect assertion",
      "assert(2 + 2 == 5)",
      CheckContext::Main,
      false,
    )?;
    self.check_preliminary_test("status code 1", "return 1;", CheckContext::Main, false)?;

    Ok(())
  }

  fn check_preliminary_test<S: Display>(
    &self,
    name: &str,
    code: S,
    context: CheckContext,
    expected: bool,
  ) -> Result<()> {
    log::status(format!("Running preliminary test: {}", name));
    match self.check_code(code, context)? {
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

  fn check_code<S: Display>(&self, code: S, context: CheckContext) -> Result<CppLibBuilderOutput> {
    {
      let mut file = create_file(&self.main_cpp_path)?;
      file.write("#include \"utils.h\"\n\n")?;
      match context {
        CheckContext::Main => {
          file.write(format!("int main() {{\n{}\n}}\n", code))?;
        }
        CheckContext::Global => {
          file.write(format!("{}\n\nint main() {{}}\n", code))?;
        }
      }
    }
    self.builder.run()
  }
}

pub fn run(workspace: &mut Workspace, config: &Config) -> Result<()> {
  let root_path = workspace.tmp_path()?.with_added("cpp_checker");
  if root_path.exists() {
    remove_dir_all(&root_path)?;
  }
  let src_path = root_path.with_added("src");
  create_dir_all(&src_path)?;
  create_file(src_path.with_added("CMakeLists.txt"))?
    .write(include_str!("../../templates/cpp_checker/CMakeLists.txt"))?;
  create_file(src_path.with_added("utils.h"))?.write(format!(
    include_str!("../../templates/cpp_checker/utils.h"),
    include_directives_code = config
      .include_directives()
      .map_if_ok(|d| -> Result<_> { Ok(format!("#include \"{}\"", path_to_str(d)?)) })?
      .join("\n")
  ))?;

  let builder = CppLibBuilder {
    cmake_source_dir: src_path.clone(),
    build_dir: root_path.with_added("build"),
    install_dir: None,
    num_jobs: None,
    build_type: BuildType::Debug,
    cmake_vars: c2r_cmake_vars(
      &config.cpp_build_config().eval(&current_target())?,
      config.cpp_build_paths(),
      None,
    )?,
    capture_output: true,
  };

  let mut checker = CppChecker {
    workspace,
    config,
    builder,
    main_cpp_path: src_path.with_added("main.cpp"),
  };

  checker.run()?;

  Ok(())
}
