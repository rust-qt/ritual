use common::file_utils::{create_file, FileWrapper};
use common::errors::{ChainErr, Result};
use std::path::Path;
use common::file_utils::PathBufWithAdded;

pub struct HtmlLogger {
  file: FileWrapper,
}

impl HtmlLogger {
  pub fn new<P: AsRef<Path>>(path: P, title: &str) -> Result<HtmlLogger> {
    let mut file = create_file(path)?;
    file.write(format!(
      include_str!("../../templates/html_logger/header.html"),
      title = title
    ))?;
    Ok(HtmlLogger { file: file })
  }

  pub fn add_header(&mut self, titles: &[&str]) -> Result<()> {
    self.file.write("<tr>")?;
    for title in titles {
      self.file.write(format!("<th>{}</th>", title))?;
    }
    self.file.write("</tr>")?;
    Ok(())
  }
  pub fn add(&mut self, texts: &[&str], classes: &str) -> Result<()> {
    self.file.write(format!("<tr class='{}'>", classes))?;
    for text in texts {
      self.file.write(format!("<td>{}</td>", text))?;
    }
    self.file.write("</tr>")?;
    Ok(())
  }

  fn finalize(&mut self) -> Result<()> {
    self
      .file
      .write(include_str!("../../templates/html_logger/footer.html"))?;
    let parent_path = self
      .file
      .path()
      .parent()
      .chain_err(|| "path parent failed")?;
    let style_path = parent_path.with_added("style.css");
    if !style_path.exists() {
      create_file(style_path)?.write(include_str!("../../templates/html_logger/style.css"))?;
    }
    let script_path = parent_path.with_added("script.js");
    if !script_path.exists() {
      create_file(script_path)?.write(include_str!("../../templates/html_logger/script.js"))?;
    }
    Ok(())
  }
}

impl Drop for HtmlLogger {
  fn drop(&mut self) {
    self.finalize().unwrap();
  }
}
