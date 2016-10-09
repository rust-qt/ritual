#![cfg_attr(feature="clippy", allow(redundant_closure))]

use std;

error_chain! {
  foreign_links {
    std::io::Error, IO;
  }

  errors {
    CreateDirFailed(path: String) {
      display("failed: create_dir({:?})", path)
    }
    ReadDirFailed(path: String) {
      display("failed: read_dir({:?})", path)
    }
    ReadDirItemFailed(path: String) {
      display("failed: item of read_dir({:?})", path)
    }
    RemoveDirAllFailed(path: String) {
      display("failed: remove_dir_all({:?})", path)
    }
    RemoveFileFailed(path: String) {
      display("failed: remove_file({:?})", path)
    }
    MoveOneFileFailed { from: String, to: String } {
      display("failed: move_one_file({:?}, {:?})", from, to)
    }
    MoveFilesFailed { from: String, to: String } {
      display("failed: move_files({:?}, {:?})", from, to)
    }
    CopyRecursivelyFailed { from: String, to: String } {
      display("failed: copy_recursively({:?}, {:?})", from, to)
    }
    CopyFileFailed { from: String, to: String } {
      display("failed: copy_file({:?}, {:?})", from, to)
    }
    ReadFileFailed(path: String) {
      display("failed: read_file({:?})", path)
    }
    RenameFileFailed { from: String, to: String } {
      display("failed: rename_file({:?}, {:?})", from, to)
    }
    CommandFailed(cmd: String) {
      display("command execution failed: {}", cmd)
    }
    CommandStatusFailed { cmd: String, status: std::process::ExitStatus } {
      display("command failed with status {:?}: {}", status, cmd)
    }
    QMakeQueryFailed
    CMakeFailed
    MakeFailed
    CargoFailed
    CWrapperBuildFailed
    SourceDirDoesntExist(path: String)
  }
}

use log;
use utils::manifest_dir;

extern crate backtrace;
use self::backtrace::Symbol;

impl Error {
  pub fn display_report(&self) {
    if let Some(backtrace) = self.backtrace() {
      log::error(format!("{:?}", backtrace));
      log::error("");
      let dir = manifest_dir();
      let mut next_frame_num = 0;
      for frame in backtrace.frames() {
        for symbol in frame.symbols() {
          if let Some(path) = symbol.filename() {
            if let Ok(relative_path) = path.strip_prefix(&dir) {
              let path_items: Vec<_> = relative_path.iter().collect();
              if path_items.len() == 2 && path_items[0] == "src" && path_items[1] == "errors.rs" {
                continue;
              }
              let name = if let Some(name) = symbol.name() {
                name.to_string()
              } else {
                "<no name>".to_string()
              };
              if next_frame_num == 0 {
                log::error("Best of stack backtrace:");
              }
              log::error(format!("{:>w$}: {}", next_frame_num, name, w = 4));
              log::info(format!("      at {}:{}",
                                relative_path.display(),
                                if let Some(n) = symbol.lineno() {
                                  n.to_string()
                                } else {
                                  "<no lineno>".to_string()
                                }));
              log::info("");
              next_frame_num += 1;
            }
          }
        }
      }
      if next_frame_num > 0 {
        log::error("");
      }
    }
    log::error("Error:");
    let items: Vec<_> = self.iter().collect();
    for (i, err) in items.iter().rev().enumerate() {
      log::error(format!("{:>w$}: {}", i, err, w = 4));
    }
  }
}
