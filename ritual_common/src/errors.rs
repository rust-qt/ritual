//! Error handling types based on `failure` crate.

pub type Result<T> = std::result::Result<T, failure::Error>;
pub use failure::{bail, ensure, err_msg, format_err, Error, ResultExt};
use itertools::Itertools;
use log::log;
use std::env;

pub trait FancyUnwrap {
    type Output;
    fn fancy_unwrap(self) -> Self::Output;
}

pub fn print_trace(err: &failure::Error, log_level: log::Level) {
    log!(log_level, "");
    log!(log_level, "Error:");
    for cause in err.iter_chain() {
        log!(log_level, "   {}", cause);
    }
    let backtrace = err.backtrace().to_string();
    if !backtrace.is_empty() {
        if env::var("RUST_BACKTRACE").as_ref().map(|v| v.as_str()) == Ok("full") {
            log!(log_level, "{}", backtrace);
        } else {
            log!(log_level, "Short backtrace:");
            let mut lines = backtrace.split('\n').collect_vec();
            if let Some(position) = lines
                .iter()
                .position(|line| line.contains("std::rt::lang_start::"))
            {
                lines.truncate(position);
            }
            if let Some(position) = lines
                .iter()
                .position(|line| line.contains("failure::backtrace::Backtrace::new::"))
            {
                lines.drain(0..position + 2);
            }
            log!(log_level, "{}", lines.join("\n"));
        }
    }
}

impl<T> FancyUnwrap for Result<T> {
    type Output = T;

    fn fancy_unwrap(self) -> T {
        match self {
            Ok(value) => value,
            Err(err) => {
                print_trace(&err, log::Level::Error);
                std::process::exit(1);
            }
        }
    }
}
