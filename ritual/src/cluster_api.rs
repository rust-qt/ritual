use crate::config::ClusterConfig;
use crate::cpp_checker::{LocalSnippetTask, Snippet, CHUNK_SIZE};
use amqp::{protocol::basic::BasicProperties, Basic, Channel, Session, Table};
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
pub struct GroupKey {
    pub crate_name: String,
    pub cpp_library_version: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct GroupItem {
    pub snippet: Snippet,
    pub id: u64,
}

pub fn run_checks(config: &ClusterConfig, tasks: &mut [LocalSnippetTask]) -> Result<()> {
    if config.protocol_version != PROTOCOL_VERSION {
        bail!("unsupported cluster protocol version");
    }

    let mut session = Session::open_url(&config.queue_address)
        .with_context(|_| format!("can't connect to queue at {}", config.queue_address))?;
    let mut channel = session.open_channel(1)?;

    let mut grouped = HashMap::<Target, HashMap<GroupKey, Vec<GroupItem>>>::new();
    for (index, task) in tasks.iter().enumerate() {
        let group = grouped
            .entry(task.data.library_target.target.clone())
            .or_default();
        let key = GroupKey {
            crate_name: task.data.crate_name.clone(),
            cpp_library_version: task.data.library_target.cpp_library_version.clone(),
        };
        let group2 = group.entry(key).or_default();
        group2.push(GroupItem {
            snippet: task.snippet.clone(),
            id: index as u64,
        });
    }

    let launch_id = Uuid::new_v4().to_simple().to_string();

    let output_queue_name = task_output_queue_name(&launch_id);
    channel.queue_declare(
        output_queue_name.clone(),
        false,
        false,
        false,
        false,
        false,
        Table::new(),
    )?;

    info!("sending tasks to queue");
    for (target, group) in grouped {
        let queue_name = task_queue_name(&target);
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
                    group_key: key.clone(),
                    snippets: chunk.to_vec(),
                };

                let json = serde_json::to_vec(&task)?;

                channel.basic_publish(
                    "",
                    &queue_name,
                    true,
                    false,
                    BasicProperties::default(),
                    json,
                )?;
            }
        }
    }

    let progress_bar = ProgressBar::new(tasks.len() as u64, "Waiting for results");

    let mut received_count = 0;

    loop {
        for message in channel.basic_get(&output_queue_name, false) {
            //println!("Headers: {:?}", get_result.headers);
            //println!("Reply: {:?}", get_result.reply);
            //println!("Body: {:?}", String::from_utf8_lossy(&get_result.body));

            let outputs: Vec<TaskOutput> = serde_json::from_slice(&message.body)?;
            for output in outputs {
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

pub struct Client {
    _session: Session,
    channel: Channel,
    output_channel: Channel,
    queue_name: String,
}

impl Client {
    pub fn new(queue_address: &str, target: &Target) -> Result<Client> {
        let mut session = Session::open_url(queue_address)
            .with_context(|_| format!("can't connect to queue at {}", queue_address))?;
        let mut channel = session.open_channel(1)?;

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

        let output_channel = session.open_channel(2)?;

        Ok(Client {
            _session: session,
            channel,
            output_channel,
            queue_name,
        })
    }

    pub fn run(&mut self, work: impl FnMut(Task) -> Result<Vec<TaskOutput>>) -> Result<Vec<Task>> {
        let mut work = work;
        loop {
            for message in self.channel.basic_get(&self.queue_name, false) {
                let task: Task = serde_json::from_slice(&message.body)?;
                let launch_id = task.launch_id.clone();
                let output = work(task)?;

                let queue_name = task_output_queue_name(&launch_id);
                let json = serde_json::to_vec(&output)?;
                info!("sending output: {:?}", output);
                self.output_channel.basic_publish(
                    "",
                    &queue_name,
                    true,
                    false,
                    BasicProperties::default(),
                    json,
                )?;

                message.ack();
            }
            sleep(Duration::from_millis(100));
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub launch_id: String,
    pub group_key: GroupKey,
    pub snippets: Vec<GroupItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskOutput {
    pub id: u64,
    pub output: CppLibBuilderOutput,
}

fn task_queue_name(target: &Target) -> String {
    format!("ritual-{}-tasks-{}", PROTOCOL_VERSION, target.short_text())
}

fn task_output_queue_name(launch_id: &str) -> String {
    format!("ritual-{}-task-output-{}", PROTOCOL_VERSION, launch_id)
}
