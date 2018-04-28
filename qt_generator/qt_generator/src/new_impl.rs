use cpp_to_rust_generator::common::errors::{ChainErr, Result};
use cpp_to_rust_generator::common::log;
use cpp_to_rust_generator::common::file_utils::canonicalize;
use cpp_to_rust_generator::new_impl::workspace::Workspace;
use std::path::PathBuf;
use qt_generator_common::all_sublib_names;
use lib_configs::make_config;

fn run(matches: ::clap::ArgMatches) -> Result<()> {
  let workspace_path = canonicalize(&PathBuf::from(matches
    .value_of("workspace")
    .chain_err(|| "clap arg missing")?))?;

  log::status(format!("Workspace: {}", workspace_path.display()));
  let mut workspace = Workspace::new(workspace_path)?;
  workspace.set_disable_logging(matches.is_present("disable-logging"))?;
  let mut was_any_action = false;

  if matches.is_present("process") {
    let libs: Vec<_> = matches
      .values_of("process")
      .chain_err(|| "clap arg missing")?
      .map(|s| s.to_lowercase())
      .collect();

    let final_libs = if libs.iter().any(|x| x == "all") {
      all_sublib_names().iter().map(|s| s.to_string()).collect()
    } else {
      libs
    };
    for sublib_name in final_libs {
      let config = make_config(&sublib_name)?;
      was_any_action = true;
      workspace.process_crate(&config)?;
    }
  }

  if matches.is_present("generate") {

    /*
      if exec_config.write_dependencies_local_paths {
        log::status(
          "Output Cargo.toml file will contain local paths of used dependencies \
                   (use --no-local-paths to disable).",
        );
      } else {
        log::status(
          "Local paths will not be written to the output crate. Make sure all dependencies \
                   are published before trying to compile the crate.",
        );
      }

    */
  }

  //...

  workspace.save_data()?;
  if was_any_action {
    log::status("qt_generator finished");
  } else {
    log::error("No action requested. Run \"qt_generator --help\".");
  }
  Ok(())
}

pub fn new_main() {
  let result = {
    use clap::{App, Arg};
    const ABOUT: &'static str = "Generates rust_qt crates using cpp_to_rust";
    const AFTER_HELP: &'static str =
      "\
       Example:\n    qt_generator -w /path/to/workspace -p all -g\n\n\
       See https://github.com/rust-qt/cpp_to_rust for more details.";
    const WORKSPACE_DIR_HELP: &'static str = "Directory for output and temporary files";
    const DISABLE_LOGGING_HELP: &'static str = "Disable creating log files";
    const GENERATE_HELP: &'static str = "Generate new crates";
    const CLEAR_ALL_HELP: &'static str = "Clear all data in the workspace.";
    const CLEAR_CURRENT_HELP: &'static str =
      "\
       Clear data corresponding to the current platform in the workspace.";

    let libs_help = format!(
      "Process libraries (Qt modules). Specify \"all\" \
       to process all supported modules or specify one or multiple of the following: {}.",
      all_sublib_names().join(", ")
    );

    run(
      App::new("cpp_to_rust")
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .arg(
          Arg::with_name("workspace")
            .short("w")
            .long("workspace")
            .value_name("WORKSPACE")
            .help(WORKSPACE_DIR_HELP)
            .takes_value(true)
            .required(true),
        )
        .arg(
          Arg::with_name("process")
            .short("p")
            .long("process")
            .value_name("LIB1 LIB2 LIB3")
            .help(&libs_help)
            .takes_value(true)
            .multiple(true)
            .use_delimiter(false),
        )
        .arg(
          Arg::with_name("generate")
            .short("g")
            .long("generate")
            .help(&GENERATE_HELP),
        )
        .arg(
          Arg::with_name("disable-logging")
            .long("disable-logging")
            .help(DISABLE_LOGGING_HELP),
        )
        .arg(
          Arg::with_name("clear-current")
            .long("clear-current")
            .help(CLEAR_CURRENT_HELP),
        )
        .arg(
          Arg::with_name("clear-all")
            .long("clear-all")
            .help(CLEAR_ALL_HELP),
        )
        .get_matches(),
    )
  };
  if let Err(err) = result {
    err.display_report();
    ::std::process::exit(1);
  }
}
