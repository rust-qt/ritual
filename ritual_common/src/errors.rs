//! Error handling types based on `failure` crate.

use itertools::Itertools;
use log::{log, log_enabled, Level};
use std::env;

pub use failure::{bail, ensure, err_msg, format_err, Error, ResultExt};

pub type Result<T> = std::result::Result<T, failure::Error>;

pub trait FancyUnwrap {
    type Output;
    fn fancy_unwrap(self) -> Self::Output;
}

macro_rules! log_or_print {
    ($lvl:expr, $($arg:tt)+) => {
        if let Some(level) = $lvl {
            log!(level, $($arg)+);
        } else if log_enabled!(Level::Error) {
            log!(Level::Error, $($arg)+);
        } else {
            eprintln!($($arg)+);
        }
    };
}

pub fn print_trace(err: &failure::Error, log_level: Option<log::Level>) {
    log_or_print!(log_level, "Error:");
    for cause in err.iter_chain() {
        log_or_print!(log_level, "   {}", cause);
    }
    let backtrace = err.backtrace().to_string();
    if !backtrace.is_empty() {
        if env::var("RUST_BACKTRACE").as_ref().map(String::as_str) == Ok("full") {
            log_or_print!(log_level, "{}", backtrace);
        } else {
            log_or_print!(log_level, "Short backtrace:");
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
            for line in lines {
                log_or_print!(log_level, "{}", line);
            }
        }
    }
}

impl<T> FancyUnwrap for Result<T> {
    type Output = T;

    fn fancy_unwrap(self) -> T {
        match self {
            Ok(value) => value,
            Err(err) => {
                print_trace(&err, None);
                std::process::exit(1);
            }
        }
    }
}
