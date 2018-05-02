use common::errors::{ChainErr, Result};
use config::Config;
use new_impl::workspace::Workspace;
use common::file_utils::{create_dir_all, create_file, path_to_str, remove_dir_all};
use common::file_utils::PathBufWithAdded;
use common::utils::MapIfOk;
use common::cpp_lib_builder::{BuildType, CppLibBuilder, c2r_cmake_vars};
use common::target::current_target;

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
  create_file(src_path.with_added("main.cpp"))?
    .write("#include \"utils.h\"\n\nint main() { std::cout << \"hello?\\n\"; return 0; }\n")?;

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
  };

  builder.run()?;

  Ok(())
}
