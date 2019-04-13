use crate::config::ClusterConfig;
use crate::cpp_checker::{Snippet, SnippetTask, CHUNK_SIZE};
use amqp::{protocol::basic::BasicProperties, Basic, Session, Table};
use log::{info, warn};
use ritual_common::cpp_lib_builder::CppLibBuilderOutput;
use ritual_common::errors::{bail, Result, ResultExt};
use ritual_common::target::Target;
use ritual_common::utils::ProgressBar;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use uuid::Uuid;

const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
struct GroupKey {
    crate_name: String,
    cpp_library_version: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct GroupItem {
    snippet: Snippet,
    id: u64,
}

pub fn run_checks(config: &ClusterConfig, tasks: &mut [SnippetTask]) -> Result<()> {
    if config.protocol_version != PROTOCOL_VERSION {
        bail!("unsupported cluster protocol version");
    }

    let mut session = Session::open_url(&config.queue_address)
        .with_context(|_| format!("can't connect to queue at {}", config.queue_address))?;
    let mut channel = session.open_channel(1)?;

    let mut grouped = HashMap::<Target, HashMap<GroupKey, Vec<GroupItem>>>::new();
    for (index, task) in tasks.iter().enumerate() {
        let group = grouped
            .entry(task.library_target.target.clone())
            .or_default();
        let key = GroupKey {
            crate_name: task.crate_name.clone(),
            cpp_library_version: task.library_target.cpp_library_version.clone(),
        };
        let group2 = group.entry(key).or_default();
        group2.push(GroupItem {
            snippet: task.snippet.clone(),
            id: index as u64,
        });
    }

    let launch_id = Uuid::new_v4().to_simple().to_string();

    info!("sending tasks to queue");
    for (target, group) in grouped {
        let queue_name = task_queue_name(target);
        channel.queue_declare(
            queue_name.clone(),
            false,
            false,
            false,
            false,
            false,
            Table::new(),
        )?;

        for (key, items) in group {
            for chunk in items.chunks(CHUNK_SIZE) {
                let task = Task {
                    launch_id: launch_id.clone(),
                    crate_name: key.crate_name.clone(),
                    cpp_library_version: key.cpp_library_version.clone(),
                    snippets: chunk.to_vec(),
                };

                let json = serde_json::to_vec(&task)?;

                channel
                    .basic_publish(
                        "",
                        &queue_name,
                        true,
                        false,
                        BasicProperties::default(),
                        json,
                    )
                    .unwrap();
            }
        }
    }

    let progress_bar = ProgressBar::new(tasks.len() as u64, "Waiting for results");
    let queue_name = task_output_queue_name(&launch_id);
    let mut received_count = 0;

    loop {
        for message in channel.basic_get(&queue_name, false) {
            //println!("Headers: {:?}", get_result.headers);
            //println!("Reply: {:?}", get_result.reply);
            //println!("Body: {:?}", String::from_utf8_lossy(&get_result.body));

            let output: TaskOutput = serde_json::from_slice(&message.body)?;
            let index = output.id as usize;
            if index >= tasks.len() {
                bail!("invalid id in received TaskOutput");
            }
            let task = &mut tasks[index];
            if task.output.is_some() {
                warn!("received duplicate TaskOutput");
            } else {
                task.output = Some(output.output);
                progress_bar.add(1);
                received_count += 1;
            }
            message.ack();
        }
        if received_count >= tasks.len() {
            break;
        }
        sleep(Duration::from_millis(100));
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    launch_id: String,
    crate_name: String,
    cpp_library_version: Option<String>,
    snippets: Vec<GroupItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskOutput {
    id: u64,
    output: CppLibBuilderOutput,
}

fn task_queue_name(target: Target) -> String {
    format!("ritual-{}-tasks-{}", PROTOCOL_VERSION, target.short_text())
}

fn task_output_queue_name(launch_id: &str) -> String {
    format!("ritual-{}-task-output-{}", PROTOCOL_VERSION, launch_id)
}
