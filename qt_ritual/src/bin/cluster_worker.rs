use flexi_logger::{LogSpecification, Logger};
use itertools::Itertools;
use log::LevelFilter;
use log::{info, warn};
use qt_ritual::lib_configs::{create_config, MOQT_INSTALL_DIR_ENV_VAR_NAME};
use qt_ritual_common::all_crate_names;
use ritual::cluster_api::{Client, GroupKey, TaskOutput};
use ritual::config::CrateProperties;
use ritual::cpp_checker::{LocalCppChecker, SnippetTask};
use ritual_common::errors::{format_err, FancyUnwrap, Result};
use ritual_common::file_utils::create_dir;
use ritual_common::target::current_target;
use std::collections::HashMap;
use std::env;
use tempdir::TempDir;

const QUEUE_ADDRESS_VAR: &str = "QT_RITUAL_WORKER_QUEUE_ADDRESS";
const RUN_TESTS_VAR: &str = "QT_RITUAL_WORKER_RUN_TESTS";
const QMAKE_PATH_VAR_PREFIX: &str = "QT_RITUAL_QMAKE_";

struct RemoteSnippetTaskData {
    id: u64,
}

fn run() -> Result<()> {
    Logger::with(LogSpecification::default(LevelFilter::Info).build())
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed: {}", e));

    let temp_dir = TempDir::new("qt_ritual_cluster_worker")?;
    let moqt_present = env::var(MOQT_INSTALL_DIR_ENV_VAR_NAME).is_ok();
    let supported_moqt_libs = ["moqt_core", "moqt_gui"]
        .iter()
        .filter(|_| moqt_present)
        .map(|&crate_name| {
            let lib = GroupKey {
                crate_name: crate_name.to_string(),
                cpp_library_version: None,
            };
            (lib, None)
        });

    let supported_qt_libs = env::vars()
        .filter(|(key, _value)| key.starts_with(QMAKE_PATH_VAR_PREFIX))
        .flat_map(|(key, value)| {
            let version = key[QMAKE_PATH_VAR_PREFIX.len()..].replace("_", ".");
            all_crate_names().iter().map(move |&crate_name| {
                let lib = GroupKey {
                    crate_name: crate_name.to_string(),
                    cpp_library_version: Some(version.clone()),
                };
                (lib, Some(value.clone()))
            })
        });

    let supported_libs = supported_moqt_libs.chain(supported_qt_libs);

    let mut checkers = HashMap::new();
    let run_tests = env::var(RUN_TESTS_VAR).ok().map_or(false, |s| s == "1");
    if run_tests {
        info!("running tests");
    }

    for (lib, qmake_path) in supported_libs {
        info!("lib: {:?}", lib);
        let dir = temp_dir.path().join(format!(
            "{}_{}",
            lib.crate_name,
            lib.cpp_library_version
                .as_ref()
                .map(String::as_str)
                .unwrap_or("noversion")
        ));
        create_dir(&dir)?;

        let qmake_path = qmake_path.as_ref().map(String::as_str);
        let config = create_config(CrateProperties::new(&lib.crate_name, ""), qmake_path)?;
        let checker = LocalCppChecker::new(dir, &config)?;
        let mut checker = checker.get("0")?;
        if run_tests {
            checker.check_preliminary_tests()?;
        }
        checkers.insert(lib, checker);
    }
    if run_tests {
        info!("all tests passed");
        return Ok(());
    }

    let queue_address = env::var(QUEUE_ADDRESS_VAR)
        .map_err(|err| format_err!("failed to get env var \"{}\": {}", QUEUE_ADDRESS_VAR, err))?;
    info!("connecting to queue");
    let mut client = Client::new(&queue_address, &current_target())?;
    info!("ready");
    client.run(|task| {
        info!("received task: {:?}", task);
        if let Some(checker) = checkers.get_mut(&task.group_key) {
            let mut snippets = task
                .snippets
                .into_iter()
                .map(|item| SnippetTask {
                    snippet: item.snippet,
                    data: RemoteSnippetTaskData { id: item.id },
                    output: None,
                })
                .collect_vec();
            checker.binary_check(&mut snippets, None)?;
            let outputs = snippets
                .into_iter()
                .map(|snippet| TaskOutput {
                    id: snippet.data.id,
                    output: snippet.output.unwrap(),
                })
                .collect_vec();
            Ok(outputs)
        } else {
            warn!("unknown group key: {:?}", task);
            Ok(Vec::new())
        }
    })?;
    Ok(())
}

fn main() {
    run().fancy_unwrap();
}
