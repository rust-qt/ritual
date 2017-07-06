//! Logger implementation

use std::fs::File;
use std::io::Write;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::borrow::Borrow;
use std;

/// Logger category. Logger can be configured to save
/// messages of each category to a separate file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LoggerCategory {
  Status,
  Error,
  DebugGeneral,
  DebugMoveFiles,
  DebugTemplateInstantiation,
  DebugInheritance,
  DebugParserSkips,
  DebugParser,
  DebugFfiSkips,
  DebugSignals,
  DebugAllocationPlace,
  DebugRustSkips,
  DebugQtDoc,
  DebugQtDocDeclarations,
  DebugQtHeaderNames,
}
pub use self::LoggerCategory::*;

/// Specifies where the logging messages should be sent.
#[derive(Debug)]
pub struct LoggerSettings {
  /// Write messages to specified file path. If `None`,
  /// logging to file is disabled.
  pub file_path: Option<PathBuf>,
  /// Write messages to stderr.
  pub write_to_stderr: bool,
}
impl Default for LoggerSettings {
  fn default() -> LoggerSettings {
    LoggerSettings {
      file_path: None,
      write_to_stderr: true,
    }
  }
}

impl LoggerSettings {
  /// Returns false if messages are ignored. This function
  /// can be used to skip expensive construction of messages.
  fn is_on(&self) -> bool {
    self.write_to_stderr || self.file_path.is_some()
  }
}

/// Logger object. One logger manages messages of all categories.
/// It's possible to use multiple loggers independently.
/// Use `default_logger()` to get global `Logger` instance.
/// Note that the instance is mutex-guarded.
#[derive(Default)]
pub struct Logger {
  default_settings: LoggerSettings,
  category_settings: HashMap<LoggerCategory, LoggerSettings>,
  files: HashMap<LoggerCategory, File>,
}

impl Logger {
  /// Creates a new logger.
  pub fn new() -> Logger {
    Logger::default()
  }

  /// Set settings for all categories that don't have specific category settings.
  pub fn set_default_settings(&mut self, value: LoggerSettings) {
    self.default_settings = value;
    self.files.clear();
  }
  /// Set settings for `category`.
  pub fn set_category_settings(&mut self, category: LoggerCategory, value: LoggerSettings) {
    self.category_settings.insert(category, value);
    self.files.remove(&category);
  }

  /// Set all specific category settings. Old category settings are removed.
  pub fn set_all_category_settings(&mut self, value: HashMap<LoggerCategory, LoggerSettings>) {
    self.category_settings = value;
    self.files.clear();
  }

  /// Returns false if messages of `category` are ignored. This function
  /// can be used to skip expensive construction of messages.
  pub fn is_on(&self, category: LoggerCategory) -> bool {
    self.settings(category).is_on()
  }

  /// Lazy-log. If messages of `category` are not ignored, calls the passed closure
  /// and uses its output value as a message in that category.
  pub fn llog<T: Borrow<str>, F: FnOnce() -> T>(&mut self, category: LoggerCategory, f: F) {
    let settings = if let Some(data) = self.category_settings.get(&category) {
      data
    } else {
      &self.default_settings
    };
    if !settings.is_on() {
      return;
    }
    let text = f();
    if settings.write_to_stderr {
      std::io::stderr()
        .write(text.borrow().as_bytes())
        .unwrap();
      std::io::stderr().write(b"\n").unwrap();
    }
    if let Some(ref path) = settings.file_path {
      if !self.files.contains_key(&category) {
        let file =
          OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(path)
            .unwrap_or_else(|err| panic!("failed to open log file '{}': {}", path.display(), err));
        self.files.insert(category, file);
      }
      let mut file = self.files.get_mut(&category).unwrap();
      file.write(text.borrow().as_bytes()).unwrap();
      file.write(b"\n").unwrap();
    }
  }

  /// Log a message `text` to `category`.
  pub fn log<T: Borrow<str>>(&mut self, category: LoggerCategory, text: T) {
    self.llog(category, move || text);
  }

  /// Returns settings for `category`.
  fn settings(&self, category: LoggerCategory) -> &LoggerSettings {
    if let Some(data) = self.category_settings.get(&category) {
      data
    } else {
      &self.default_settings
    }
  }
}

lazy_static! {
  pub static ref DEFAULT_LOGGER: Mutex<Logger> = Mutex::new(Logger::new());

}

/// Returns global instance of `Logger`.
pub fn default_logger() -> MutexGuard<'static, Logger> {
  DEFAULT_LOGGER.lock().unwrap()
}

/// Convenience method to log status messages to the default logger.
pub fn status<T: Borrow<str>>(text: T) {
  default_logger().log(LoggerCategory::Status, text);
}

/// Convenience method to log error messages to the default logger.
pub fn error<T: Borrow<str>>(text: T) {
  default_logger().log(LoggerCategory::Error, text);
}

/// Convenience method to log messages to the default logger and specified `category`.
pub fn log<T: Borrow<str>>(category: LoggerCategory, text: T) {
  default_logger().log(category, text);
}

/// Convenience method to lazy-log messages to the default logger and specified `category`.
/// If messages of `category` are not ignored, calls the passed closure
/// and uses its output value as a message in that category.
pub fn llog<T: Borrow<str>, F: FnOnce() -> T>(category: LoggerCategory, f: F) {
  default_logger().llog(category, f);
}

/// Convenience method to check if `category` is enabled in the default logger.
pub fn is_on(category: LoggerCategory) -> bool {
  default_logger().is_on(category)
}
