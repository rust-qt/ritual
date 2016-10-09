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
    SourceDirDoesntExist(path: String) {
      display("source dir doesn't exist: {:?}", path)
    }
    JoinPathsFailed
    AddEnvFailed
  }
}
