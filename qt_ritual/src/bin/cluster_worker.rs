use flexi_logger::{LogSpecification, Logger};
use itertools::Itertools;
use log::LevelFilter;
use log::{info, warn};
use qt_ritual::lib_configs::create_config;
use ritual::cluster_api::{Client, GroupKey, TaskOutput};
use ritual::cpp_checker::{LocalCppChecker, SnippetTask};
use ritual_common::errors::{format_err, FancyUnwrap, Result};
use ritual_common::file_utils::create_dir;
use ritual_common::target::current_target;
use std::collections::HashMap;
use std::env;
use tempdir::TempDir;

const QUEUE_ADDRESS_VAR: &str = "QT_RITUAL_WORKER_QUEUE_ADDRESS";

struct RemoteSnippetTaskData {
    id: u64,
}

fn run() -> Result<()> {
    Logger::with(LogSpecification::default(LevelFilter::Trace).build())
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed: {}", e));

    let temp_dir = TempDir::new("qt_ritual_cluster_worker")?;
    let supported_libs = vec![GroupKey {
        crate_name: "moqt_core".into(),
        cpp_library_version: None,
    }];

    let mut checkers = HashMap::new();

    for lib in supported_libs {
        let dir = temp_dir.path().join(format!(
            "{}_{}",
            lib.crate_name,
            lib.cpp_library_version
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("noversion")
        ));
        create_dir(&dir)?;
        // TODO: select proper Qt version for Qt libs
        let config = create_config(&lib.crate_name)?;
        let checker = LocalCppChecker::new(dir, &config)?;
        let mut checker = checker.get("0")?;
        checker.check_preliminary_tests()?;
        checkers.insert(lib, checker);
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
