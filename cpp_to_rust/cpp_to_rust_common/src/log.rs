use std::fs::File;
use std::io::Write;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LoggerCategory {
  Status,
  FatalError,
  DebugGeneral,
  DebugCppParser,
  DebugCppFfiGenerator,
}

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

#[derive(Default)]
pub struct Logger {
  pub default_settings: LoggerSettings,
  pub category_settings: HashMap<LoggerCategory, LoggerSettings>,
  files: HashMap<LoggerCategory, File>,
}

pub struct SubLogger<'a> {
  settings: &'a LoggerSettings,
  file: Option<&'a mut File>,
}

impl Logger {
  pub fn new() -> Logger {
    Logger::default()
  }

  pub fn get(&mut self, category: LoggerCategory) -> Option<SubLogger> {
    let settings = if let Some(data) = self.category_settings.get(&category) {
      data
    } else {
      &self.default_settings
    };
    if settings.file_path.is_none() && !settings.write_to_stderr {
      return None;
    }
    let file = if let Some(ref path) = settings.file_path {
      if !self.files.contains_key(&category) {
        let file = OpenOptions::new().write(true).append(true).open(path)
            .unwrap_or_else(|err| panic!("failed to open log file '{}': {}", path.display(), err));
        self.files.insert(category, file);
      }
      self.files.get_mut(&category)
    } else {
      None
    };
    Some(SubLogger {
      settings: settings,
      file: file,
    })
  }
}

impl<'a> SubLogger<'a> {
  pub fn log<T: Borrow<str>>(&mut self, text: T) {
    if self.settings.write_to_stderr {
      std::io::stderr().write(text.borrow().as_bytes()).unwrap();
    }
    if let Some(ref mut file) = self.file {
      file.write(text.borrow().as_bytes()).unwrap();
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
  if let Some(mut log) = default_logger().get(LoggerCategory::Status) {
    log.log(text);
  }
}


//use ::term_painter::{Color, ToStyle};
use std::borrow::Borrow;
use std;

pub fn error<T: Borrow<str>>(text: T) {
  // println!("{}", Color::Red.paint(text.borrow()));
  println!("{}", text.borrow());
}
pub fn warning<T: Borrow<str>>(text: T) {
  if std::env::var("CPP_TO_RUST_QUIET").is_err() {
    // println!("{}", Color::Magenta.paint(text.borrow()));
    println!("{}", text.borrow());
  }
}

pub fn debug<T: Borrow<str>>(text: T) {
  if std::env::var("CPP_TO_RUST_QUIET").is_err() {
    println!("{}", text.borrow());
  }
}

#[allow(unused_variables)]
pub fn noisy<T: Borrow<str>>(text: T) {}
