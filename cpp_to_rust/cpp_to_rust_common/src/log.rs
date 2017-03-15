use std::fs::File;
use std::io::Write;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::borrow::Borrow;
// use ::term_painter::{Color, ToStyle};
use std;

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

#[derive(Debug)]
pub struct LoggerSettings {
  pub file_path: Option<PathBuf>,
  pub write_to_stderr: bool, // stderr_color: Option<()>,
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
  fn is_on(&self) -> bool {
    self.write_to_stderr || self.file_path.is_some()
  }
}

#[derive(Default)]
pub struct Logger {
  pub default_settings: LoggerSettings,
  pub category_settings: HashMap<LoggerCategory, LoggerSettings>,
  files: HashMap<LoggerCategory, File>,
}

impl Logger {
  pub fn new() -> Logger {
    Logger::default()
  }

  pub fn is_on(&self, category: LoggerCategory) -> bool {
    self.settings(category).is_on()
  }

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
      std::io::stderr().write(text.borrow().as_bytes()).unwrap();
      std::io::stderr().write(b"\n").unwrap();
    }
    if let Some(ref path) = settings.file_path {
      if !self.files.contains_key(&category) {
        let file = OpenOptions::new()
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

  pub fn log<T: Borrow<str>>(&mut self, category: LoggerCategory, text: T) {
    self.llog(category, move || text);
  }

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

pub fn default_logger() -> MutexGuard<'static, Logger> {
  DEFAULT_LOGGER.lock().unwrap()
}

pub fn status<T: Borrow<str>>(text: T) {
  default_logger().log(LoggerCategory::Status, text);
}

pub fn error<T: Borrow<str>>(text: T) {
  default_logger().log(LoggerCategory::Error, text);
}

pub fn log<T: Borrow<str>>(category: LoggerCategory, text: T) {
  default_logger().log(category, text);
}

pub fn llog<T: Borrow<str>, F: FnOnce() -> T>(category: LoggerCategory, f: F) {
  default_logger().llog(category, f);
}
