use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use log;
use std::io;
use std::io::{Read, Write};

pub struct TweakedFile {
  file: Option<File>,
  original_path: PathBuf,
  tmp_path: PathBuf,
}

impl TweakedFile {
  pub fn create<P: AsRef<Path>>(path: P) -> io::Result<Self> {
    let original_path = path.as_ref().to_path_buf();
    let mut tmp_path = original_path.clone();
    let file_name = format!("{}.tmp", tmp_path.file_name().unwrap().to_str().unwrap());
    tmp_path.set_file_name(file_name);
    Ok(TweakedFile {
      file: Some(try!(File::create(&tmp_path))),
      original_path: original_path,
      tmp_path: tmp_path,
    })
  }
}

impl Drop for TweakedFile {
  fn drop(&mut self) {
    self.file.as_ref().unwrap().flush();
    self.file = None;
    let is_changed = if self.original_path.as_path().is_file() {
      let mut string1 = String::new();
      let mut string2 = String::new();
      File::open(&self.tmp_path).unwrap().read_to_string(&mut string1);
      File::open(&self.original_path).unwrap().read_to_string(&mut string2);
      string1 != string2
    } else {
      true
    };

    if is_changed {
      if self.original_path.as_path().exists() {
        fs::remove_file(&self.original_path).unwrap();
      }
      fs::rename(&self.tmp_path, &self.original_path).unwrap();
      log::info(format!("File changed: {}", &self.original_path.to_str().unwrap()));
    } else {
      fs::remove_file(&self.tmp_path).unwrap();
      log::info(format!("File not changed: {}",
                        &self.original_path.to_str().unwrap()));
    }
  }
}

impl io::Write for TweakedFile {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    self.file.as_ref().unwrap().write(buf)
  }
  fn flush(&mut self) -> io::Result<()> {
    self.file.as_ref().unwrap().flush()
  }
}
