//! Error handling types based on `failure` crate.

pub type Result<T> = std::result::Result<T, failure::Error>;
pub use crate::unexpected;
pub use failure::{bail, err_msg, Error, ResultExt};
use log::log;
use std::env;

pub trait FancyUnwrap {
    type Output;
    fn fancy_unwrap(self) -> Self::Output;
}

pub fn print_trace(err: failure::Error, log_level: log::Level) {
    log!(log_level, "\nError:");
    for cause in err.iter_chain() {
        log!(log_level, "   {}", cause);
    }
    let backtrace = err.backtrace().to_string();
    if !backtrace.is_empty() {
        if env::var("RUST_BACKTRACE").as_ref().map(|v| v.as_str()) == Ok("full") {
            log!(log_level, "{}", backtrace);
        } else {
            log!(log_level, "Short backtrace:");
            let mut lines: Vec<_> = backtrace.split('\n').collect();
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
                print_trace(err, log::Level::Error);
                std::process::exit(1);
            }
        }
    }
}

// TODO: replace with a proper mechanism
pub fn should_panic_on_unexpected() -> bool {
    true
}

#[macro_export]
macro_rules! unexpected {
    ($e:expr) => {
        if $crate::errors::should_panic_on_unexpected() {
            panic!($e);
        } else {
            bail!($e);
        }
    };
    ($fmt:expr, $($arg:tt)*) => {
        if $crate::errors::should_panic_on_unexpected() {
            panic!($fmt, $($arg)*);
        } else {
            bail!($fmt, $($arg)*);
        }
    };
}
