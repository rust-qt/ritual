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
use new_impl::database::CppItemData;
use cpp_data::CppTypeKind;
use cpp_type::CppTypeClassBase;
use new_impl::database::DataEnv;
use new_impl::database::DataSource;
use new_impl::html_logger::HtmlLogger;
use new_impl::database::Database;

fn snippet_for_item(item: &CppItemData) -> Result<Snippet> {
  match *item {
    CppItemData::Type(ref type1) => match type1.kind {
      CppTypeKind::Enum => Ok(Snippet::new_in_main(format!(
        "assert(std::is_enum<{}>::value);",
        type1.name
      ))),
      CppTypeKind::Class {
        ref template_arguments,
      } => {
        let type_code = CppTypeClassBase {
          name: type1.name.clone(),
          template_arguments: template_arguments.clone(),
        }.to_cpp_code()?;
        Ok(Snippet::new_in_main(format!(
          "assert(std::is_class<{}>::value);",
          type_code
        )))
      }
    },
    CppItemData::Method(ref method) => Err("snippet not implemented yet".into()),
    CppItemData::EnumValue(ref enum_value) => Err("snippet not implemented yet".into()),
    CppItemData::ClassField(ref field) => Err("snippet not implemented yet".into()),
    CppItemData::ClassBase(ref data) => Err("snippet not implemented yet".into()),
  }
}

struct CppChecker<'a> {
  database: &'a mut Database,
  config: &'a Config,
  main_cpp_path: PathBuf,
  builder: CppLibBuilder,
  data_env: DataEnv,
  html_logger: HtmlLogger,
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
    self.database.environments.push(self.data_env.clone());
    self.database.invalidate_env(&self.data_env);
    self.html_logger.add_header(&["Item", "Status"])?;
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
         assert(std::is_enum<E1>::value); \n\
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
    {
      let mut file = create_file(&self.main_cpp_path)?;
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
    self.builder.run()
  }
}

pub fn run(workspace: &mut Workspace, database: &mut Database, config: &Config) -> Result<()> {
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
    skip_cmake: false,
  };

  let html_logger = HtmlLogger::new(
    workspace.log_path()?.with_added("cpp_checker_log.html"),
    "C++ checker log",
  )?;
  let mut checker = CppChecker {
    database,
    config,
    builder,
    main_cpp_path: src_path.with_added("main.cpp"),
    data_env: DataEnv {
      target: current_target(),
      data_source: DataSource::CppChecker,
      cpp_library_version: config.cpp_lib_version().map(|s| s.to_string()),
    },
    html_logger,
  };

  checker.run()?;

  Ok(())
}
