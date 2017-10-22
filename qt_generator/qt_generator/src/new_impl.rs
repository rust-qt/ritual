use cpp_to_rust_generator::common::string_utils::JoinWithSeparator;
use cpp_to_rust_generator::common::errors::Result;

fn run(matches: ::clap::ArgMatches) -> Result<()> {
  unimplemented!()

}


pub fn new_main() {
  let result = {
    use clap::{Arg, App};
    const ABOUT: &'static str = "Generates rust_qt crates using cpp_to_rust";
    const AFTER_HELP: &'static str = "\
      Example:\n    qt_generator -w /path/to/workspace -p all -g\n\n\
      See https://github.com/rust-qt/cpp_to_rust for more details.";
    const WORKSPACE_DIR_HELP: &'static str = "Directory for output and temporary files";
    const DISABLE_LOGGING_HELP: &'static str = "Disable creating log files";
    const GENERATE_HELP: &'static str = "Generate new crates";

    let libs_help = format!(
      "Process libraries (Qt modules). Specify \"all\" \
      to process all supported modules or specify one or multiple of the following: {}.",
      ::executor::all_sublib_names().join(", ")
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
        .get_matches(),
    )
  };
  if let Err(err) = result {
    err.display_report();
    ::std::process::exit(1);
  }
}
