// inspired by https://github.com/Xion/cargo-download/

use crate::database::CRATE_DB_FILE_NAME;
use log::{info, trace};
use reqwest::header::CONTENT_LENGTH;
use ritual_common::errors::{bail, Result};
use std::io::Read;
use std::path::Path;

const CRATES_API_ROOT: &str = "https://crates.io/api/v1/crates";

pub fn download_db(crate_name: &str, crate_version: &str, path: impl AsRef<Path>) -> Result<()> {
    let download_url = format!(
        "{}/{}/{}/download",
        CRATES_API_ROOT, crate_name, crate_version
    );
    info!(
        "Downloading crate {} v{} from {}",
        crate_name, crate_version, download_url
    );
    let mut response = reqwest::get(&download_url)?;

    let content_length: Option<usize> = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|content_length| content_length.to_str().ok())
        .and_then(|content_length| content_length.parse().ok());
    trace!(
        "download size: {}",
        content_length.map_or("<unknown>".into(), |cl| format!("{} bytes", cl))
    );
    let mut bytes = match content_length {
        Some(cl) => Vec::with_capacity(cl),
        None => Vec::new(),
    };
    response.read_to_end(&mut bytes)?;

    info!("Crate {} v{} downloaded", crate_name, crate_version);

    let gzip = flate2::read::GzDecoder::new(&bytes[..]);
    let mut archive = tar::Archive::new(gzip);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_path = entry.path()?;
        if entry_path.components().count() == 2
            && entry_path.components().nth(1).unwrap().as_os_str() == CRATE_DB_FILE_NAME
        {
            info!("Unpacking database file");
            entry.unpack(path)?;
            info!("Database file unpacked");
            return Ok(());
        }
    }

    bail!(
        "database file ({:?}) not found in crate tarball",
        CRATE_DB_FILE_NAME
    );
}
