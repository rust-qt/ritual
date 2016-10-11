use log;
use errors::{Result, ChainErr};

use std::path::{Path, PathBuf};
use std::fs;
use std::io::Read;

pub fn move_files(src: &PathBuf, dst: &PathBuf) -> Result<()> {
  let err = || format!("failed: move_files({:?}, {:?})", src, dst);
  if src.as_path().is_dir() {
    if !dst.as_path().is_dir() {
      log::noisy(format!("New dir created: {}", dst.display()));
      try!(create_dir(dst).chain_err(&err));
    }

    for item in try!(read_dir(dst).chain_err(&err)) {
      let item = try!(item.chain_err(&err));
      if !src.with_added(item.file_name()).as_path().exists() {
        let path = item.path();
        if path.as_path().is_dir() {
          log::noisy(format!("Old dir removed: {}", path.display()));
          try!(remove_dir_all(&path).chain_err(&err));
        } else {
          log::noisy(format!("Old file removed: {}", path.display()));
          try!(remove_file(&path).chain_err(&err));
        }
      }
    }

    for item in try!(fs::read_dir(src).chain_err(&err)) {
      let item = try!(item.chain_err(&err));
      let from = item.path().to_path_buf();
      let to = dst.with_added(item.file_name());
      try!(move_files(&from, &to).chain_err(&err));
    }
    try!(fs::remove_dir_all(src).chain_err(&err));
  } else {
    try!(move_one_file(src, dst).chain_err(&err));
  }
  Ok(())
}

pub fn copy_recursively(src: &PathBuf, dst: &PathBuf) -> Result<()> {
  let err = || format!("failed: copy_recursively({:?}, {:?})", src, dst);
  if src.as_path().is_dir() {
    try!(create_dir(&dst).chain_err(&err));
    for item in try!(fs::read_dir(src).chain_err(&err)) {
      let item = try!(item.chain_err(&err));
      let from = item.path().to_path_buf();
      let to = dst.with_added(item.file_name());
      try!(copy_recursively(&from, &to).chain_err(&err));
    }
  } else {
    try!(copy_file(src, dst).chain_err(&err));
  }
  Ok(())
}

pub fn move_one_file(old_path: &PathBuf, new_path: &PathBuf) -> Result<()> {
  let err = || format!("failed: move_one_file({:?}, {:?})", old_path, new_path);
  let is_changed = if new_path.as_path().is_file() {
    let string1 = try!(file_to_string(old_path).chain_err(&err));
    let string2 = try!(file_to_string(new_path).chain_err(&err));
    string1 != string2
  } else {
    true
  };

  if is_changed {
    if new_path.as_path().exists() {
      try!(remove_file(&new_path).chain_err(&err));
    }
    try!(rename_file(&old_path, &new_path).chain_err(&err));
    log::noisy(format!("File changed: {}", new_path.display()));
  } else {
    try!(remove_file(&old_path).chain_err(&err));
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

pub struct FileWrapper {
  file: fs::File,
  path: PathBuf,
}

pub fn open_file<P: AsRef<Path>>(path: P) -> Result<FileWrapper> {
  Ok(FileWrapper {
    file: try!(fs::File::open(path.as_ref())
      .chain_err(|| format!("Failed to open file for reading: {:?}", path.as_ref()))),
    path: path.as_ref().to_path_buf(),
  })
}

pub fn file_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
  let mut f = try!(open_file(path));
  f.read_all()
}

pub fn create_file<P: AsRef<Path>>(path: P) -> Result<FileWrapper> {
  Ok(FileWrapper {
    file: try!(fs::File::create(path.as_ref())
      .chain_err(|| format!("Failed to create file: {:?}", path.as_ref()))),
    path: path.as_ref().to_path_buf(),
  })
}

impl FileWrapper {
  pub fn read_all(&mut self) -> Result<String> {
    let mut r = String::new();
    try!(self.file
      .read_to_string(&mut r)
      .chain_err(|| format!("Failed to read from file: {:?}", self.path)));
    Ok(r)
  }

  pub fn write<S: AsRef<str>>(&mut self, text: S) -> Result<()> {
    use std::io::Write;
    self.file
      .write(text.as_ref().as_bytes())
      .map(|_| ())
      .chain_err(|| format!("Failed to write to file: {:?}", self.path))
  }
  pub fn into_file(self) -> fs::File {
    self.file
  }
}

pub fn create_dir<P: AsRef<Path>>(path: P) -> Result<()> {
  fs::create_dir(path.as_ref()).chain_err(|| format!("Failed to create dir: {:?}", path.as_ref()))
}

pub fn create_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
  fs::create_dir_all(path.as_ref()).chain_err(|| {
    format!("Failed to create dirs (with parent components): {:?}",
            path.as_ref())
  })
}

pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
  fs::remove_dir_all(path.as_ref())
    .chain_err(|| format!("Failed to remove dir (recursively): {:?}", path.as_ref()))
}

pub fn remove_file<P: AsRef<Path>>(path: P) -> Result<()> {
  fs::remove_file(path.as_ref()).chain_err(|| format!("Failed to remove file: {:?}", path.as_ref()))
}

pub fn rename_file<P: AsRef<Path>, P2: AsRef<Path>>(path1: P, path2: P2) -> Result<()> {
  fs::rename(path1.as_ref(), path2.as_ref()).chain_err(|| {
    format!("Failed to rename file from {:?} to {:?}",
            path1.as_ref(),
            path2.as_ref())
  })
}

pub fn copy_file<P: AsRef<Path>, P2: AsRef<Path>>(path1: P, path2: P2) -> Result<()> {
  fs::copy(path1.as_ref(), path2.as_ref()).map(|_| ()).chain_err(|| {
    format!("Failed to copy file from {:?} to {:?}",
            path1.as_ref(),
            path2.as_ref())
  })
}

pub struct ReadDirWrapper {
  read_dir: fs::ReadDir,
  path: PathBuf,
}

pub fn read_dir<P: AsRef<Path>>(path: P) -> Result<ReadDirWrapper> {
  Ok(ReadDirWrapper {
    read_dir: try!(fs::read_dir(path.as_ref())
      .chain_err(|| format!("Failed to read dir: {:?}", path.as_ref()))),
    path: path.as_ref().to_path_buf(),
  })
}

impl Iterator for ReadDirWrapper {
  type Item = Result<fs::DirEntry>;
  fn next(&mut self) -> Option<Result<fs::DirEntry>> {
    self.read_dir
      .next()
      .map(|value| value.chain_err(|| format!("Failed to read dir (in item): {:?}", self.path)))
  }
}
