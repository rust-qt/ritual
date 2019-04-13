use crate::config::ClusterConfig;
use crate::cpp_checker::SnippetTask;
use ritual_common::errors::{bail, Result};

const PROTOCOL_VERSION: u32 = 1;

pub fn run_checks(config: &ClusterConfig, tasks: &mut [SnippetTask]) -> Result<()> {
    if config.protocol_version != PROTOCOL_VERSION {
        bail!("unsupported cluster protocol version");
    }
    unimplemented!()
}
