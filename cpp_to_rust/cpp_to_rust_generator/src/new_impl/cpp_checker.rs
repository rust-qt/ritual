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
use new_impl::processor::ProcessorData;
use new_impl::database::DataEnvInfo;
use std::path::Path;

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

fn check_one_item(main_cpp_path: &Path, builder: &CppLibBuilder, item: &CppItemData) -> Result<()> {
  let snippet = snippet_for_item(item).map_err(|e| format!("can't generate snippet: {}", e))?;
  match check_snippet(main_cpp_path, builder, &snippet)? {
    CppLibBuilderOutput::Success => Ok(()),
    CppLibBuilderOutput::Fail(output) => Err(format!("build failed: {}", output.stderr).into()),
  }
}

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
          "assert(std::is_class<{0}>::value);\nassert(sizeof({0}) > 0);",
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
  data: ProcessorData<'a>,
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
    self.data.html_logger.add_header(&["Item", "Status"])?;
    self.run_tests()?;

    let total_count = self.data.current_database.items.len();
    for (index, item) in self.data.current_database.items.iter_mut().enumerate() {
      log::status(format!("Checking item {} / {}", index + 1, total_count));
      let result = check_one_item(&self.main_cpp_path, &self.builder, &item.cpp_data);
      let data = match result {
        Ok(_) => DataEnvInfo {
          is_success: true,
          error: None,
          include_file: None,
          origin_location: None,
          is_invalidated: false,
        },
        Err(err) => DataEnvInfo {
          is_success: false,
          error: Some(err.to_string()),
          include_file: None,
          origin_location: None,
          is_invalidated: false,
        },
      };
      let r = item.add_cpp_data(&self.data.env, data);
      self.data.html_logger.log_database_update_result(&r);
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

pub fn run(data: ProcessorData) -> Result<()> {
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

  let mut checker = CppChecker {
    data,
    builder,
    main_cpp_path: src_path.with_added("main.cpp"),
  };

  checker.run()?;

  Ok(())
}
