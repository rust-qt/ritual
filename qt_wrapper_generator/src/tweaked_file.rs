use std::fs::File;
use std::path::{Path, PathBuf};
use std::io;
use std::io::Write;
use utils::move_one_file;

#[allow(dead_code)]
pub struct TweakedFile {
  file: Option<File>,
  original_path: PathBuf,
  tmp_path: PathBuf,
}

impl TweakedFile {
  #[allow(dead_code)]
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
    self.file.as_ref().unwrap().flush().unwrap();
    self.file = None;
    move_one_file(&self.original_path, &self.tmp_path).unwrap();
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
