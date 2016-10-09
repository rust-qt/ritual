use log;
use errors::{ErrorKind, Result, ChainErr};

use std::path::{Path, PathBuf};
use std::fs;
use std::io::Read;

pub fn move_files(src: &PathBuf, dst: &PathBuf) -> Result<()> {
  if src.as_path().is_dir() {
    if !dst.as_path().is_dir() {
      log::noisy(format!("New dir created: {}", dst.display()));
      try!(fs::create_dir(dst).chain_err(|| ErrorKind::CreateDirFailed(dst.display().to_string())));
    }

    for item in try!(fs::read_dir(dst)
      .chain_err(|| ErrorKind::ReadDirFailed(dst.display().to_string()))) {
      let item = try!(item.chain_err(|| ErrorKind::ReadDirItemFailed(dst.display().to_string())));
      if !src.with_added(item.file_name()).as_path().exists() {
        let path = item.path();
        if path.as_path().is_dir() {
          log::noisy(format!("Old dir removed: {}", path.display()));
          try!(fs::remove_dir_all(&path)
            .chain_err(|| ErrorKind::RemoveDirAllFailed(path.display().to_string())));
        } else {
          log::noisy(format!("Old file removed: {}", path.display()));
          try!(fs::remove_file(&path)
            .chain_err(|| ErrorKind::RemoveFileFailed(path.display().to_string())));
        }
      }
    }

    for item in try!(fs::read_dir(src)
      .chain_err(|| ErrorKind::ReadDirFailed(src.display().to_string()))) {
      let item = try!(item.chain_err(|| ErrorKind::ReadDirItemFailed(src.display().to_string())));
      let from = item.path().to_path_buf();
      let to = dst.with_added(item.file_name());
      try!(move_files(&from, &to).chain_err(|| {
        ErrorKind::MoveFilesFailed {
          from: from.display().to_string(),
          to: to.display().to_string(),
        }
      }));
    }
    try!(fs::remove_dir_all(src)
      .chain_err(|| ErrorKind::RemoveDirAllFailed(src.display().to_string())));
  } else {
    try!(move_one_file(src, dst).chain_err(|| {
      ErrorKind::MoveOneFileFailed {
        from: src.display().to_string(),
        to: dst.display().to_string(),
      }
    }));
  }
  Ok(())
}

pub fn copy_recursively(src: &PathBuf, dst: &PathBuf) -> Result<()> {
  if src.as_path().is_dir() {
    try!(fs::create_dir(&dst).chain_err(|| ErrorKind::CreateDirFailed(dst.display().to_string())));
    for item in try!(fs::read_dir(src)
      .chain_err(|| ErrorKind::ReadDirFailed(src.display().to_string()))) {
      let item = try!(item.chain_err(|| ErrorKind::ReadDirItemFailed(src.display().to_string())));
      let from = item.path().to_path_buf();
      let to = dst.with_added(item.file_name());
      try!(copy_recursively(&from, &to).chain_err(|| {
        ErrorKind::CopyRecursivelyFailed {
          from: from.display().to_string(),
          to: to.display().to_string(),
        }
      }));
    }
  } else {
    try!(fs::copy(src, dst).chain_err(|| {
      ErrorKind::CopyFileFailed {
        from: src.display().to_string(),
        to: dst.display().to_string(),
      }
    }));
  }
  Ok(())
}

pub fn move_one_file(old_path: &PathBuf, new_path: &PathBuf) -> Result<()> {
  let is_changed = if new_path.as_path().is_file() {
    let mut string1 = String::new();
    let mut string2 = String::new();
    try!(try!(fs::File::open(&old_path)
        .chain_err(|| ErrorKind::ReadFileFailed(old_path.display().to_string())))
      .read_to_string(&mut string1)
      .chain_err(|| ErrorKind::ReadFileFailed(old_path.display().to_string())));
    try!(try!(fs::File::open(&new_path)
        .chain_err(|| ErrorKind::ReadFileFailed(new_path.display().to_string())))
      .read_to_string(&mut string2)
      .chain_err(|| ErrorKind::ReadFileFailed(new_path.display().to_string())));
    string1 != string2
  } else {
    true
  };

  if is_changed {
    if new_path.as_path().exists() {
      try!(fs::remove_file(&new_path)
        .chain_err(|| ErrorKind::RemoveFileFailed(new_path.display().to_string())));
    }
    try!(fs::rename(&old_path, &new_path).chain_err(|| {
      ErrorKind::RenameFileFailed {
        from: old_path.display().to_string(),
        to: new_path.display().to_string(),
      }
    }));
    log::noisy(format!("File changed: {}", new_path.display()));
  } else {
    try!(fs::remove_file(&old_path)
      .chain_err(|| ErrorKind::RemoveFileFailed(old_path.display().to_string())));
    log::noisy(format!("File not changed: {}", new_path.display()));
  }
  Ok(())
}


pub trait PathBufWithAdded {
  fn with_added<P: AsRef<Path>>(&self, path: P) -> PathBuf;
}

impl PathBufWithAdded for PathBuf {
  fn with_added<P: AsRef<Path>>(&self, path: P) -> PathBuf {
    let mut p = self.clone();
    p.push(path);
    p
  }
}

impl PathBufWithAdded for Path {
  fn with_added<P: AsRef<Path>>(&self, path: P) -> PathBuf {
    let mut p = self.to_path_buf();
    p.push(path);
    p
  }
}
