//! Error handling types based on `failure` crate.

use std::io::{stderr, Write};

pub type Result<T> = std::result::Result<T, failure::Error>;
pub use crate::unexpected;
pub use failure::{bail, err_msg, Error, ResultExt};
use std::env;

pub trait FancyUnwrap {
    type Output;
    fn fancy_unwrap(self) -> Self::Output;
}

impl<T> FancyUnwrap for Result<T> {
    type Output = T;
    fn fancy_unwrap(self) -> T {
        match self {
            Ok(value) => value,
            Err(err) => {
                let mut stderr = stderr();
                writeln!(stderr, "\nError:").unwrap();
                for cause in err.iter_chain() {
                    writeln!(stderr, "   {}", cause).unwrap();
                }
                let backtrace = err.backtrace().to_string();
                if !backtrace.is_empty() {
                    if env::var("RUST_BACKTRACE").as_ref().map(|v| v.as_str()) == Ok("full") {
                        writeln!(stderr, "{}", backtrace).unwrap();
                    } else {
                        writeln!(stderr, "Short backtrace:").unwrap();
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
                        writeln!(stderr, "{}", lines.join("\n")).unwrap();
                    }
                }
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
        if should_panic_on_unexpected() {
            panic!($e);
        } else {
            bail!($e);
        }
    };
    ($fmt:expr, $($arg:tt)*) => {
        if should_panic_on_unexpected() {
            panic!($fmt, $($arg)*);
        } else {
            bail!($fmt, $($arg)*);
        }
    };
}
