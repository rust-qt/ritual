use common::errors::{ChainErr, Result};
use common::file_utils::PathBufWithAdded;
use common::file_utils::{create_file, FileWrapper};
use common::log;

use std::fmt::Display;
use std::path::Path;

pub struct HtmlLogger {
    file: FileWrapper,
}

pub fn escape_html(text: &str) -> String {
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
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
    pub fn add<S: Display>(&mut self, texts: &[S], classes: &str) -> Result<()> {
        self.file.write(format!("<tr class='{}'>", classes))?;
        for text in texts {
            self.file.write(format!("<td>{}</td>", text))?;
        }
        self.file.write("</tr>")?;
        Ok(())
    }

    /*
    pub fn log_database_update_result(&mut self, result: &DatabaseUpdateResult) {
      let log_class = match result.result_type {
        DatabaseUpdateResultType::ItemAdded => "database_item_added",
        DatabaseUpdateResultType::EnvAdded => "database_env_added",
        DatabaseUpdateResultType::EnvUpdated => "database_env_updated",
        DatabaseUpdateResultType::Unchanged => "database_unchanged",
      };
      let mut text = match result.result_type {
        DatabaseUpdateResultType::ItemAdded => "New item".to_string(),
        DatabaseUpdateResultType::EnvAdded => "New env for existing item".to_string(),
        DatabaseUpdateResultType::EnvUpdated => format!(
          "Env data changed! Old data: {}",
          result
            .old_data
            .as_ref()
            .map(|r| r.to_html_log())
            .unwrap_or("None".to_string())
        ),
        DatabaseUpdateResultType::Unchanged => "Unchanged".to_string(),
      };

      let item_texts = result.new_data.iter().map(|item| {
        format!(
          "<li>{}: {}</li>",
          item.env.short_text(),
          item.info.to_html_log()
        )
      });
      text += &format!("<ul>{}</ul>", item_texts.join(""));
      self
        .add(&[&escape_html(&result.item), &text], log_class)
        .unwrap();
    }*/

    fn finalize(&mut self) -> Result<()> {
        self.file
            .write(include_str!("../../templates/html_logger/footer.html"))?;
        let parent_path = self
            .file
            .path()
            .parent()
            .chain_err(|| "path parent failed")?;
        let style_path = parent_path.with_added("style.css");
        if !style_path.exists() {
            create_file(style_path)?
                .write(include_str!("../../templates/html_logger/style.css"))?;
        }
        let script_path = parent_path.with_added("script.js");
        if !script_path.exists() {
            create_file(script_path)?
                .write(include_str!("../../templates/html_logger/script.js"))?;
        }
        log::status(format!("Log saved as {}", self.file.path().display()));
        Ok(())
    }
}

impl Drop for HtmlLogger {
    fn drop(&mut self) {
        self.finalize().unwrap();
    }
}
