
use qt_generator_common::run_qmake_query;
use cpp_to_rust_common::errors::{Result, ChainErr};
use cpp_to_rust_common::file_utils::PathBufWithAdded;
use cpp_to_rust_common::log;
use html_parser::document::Document;
use rusqlite;

trait ConvertError<T> {
  fn convert_err(self) -> Result<T>;
}

impl<T> ConvertError<T> for ::std::result::Result<T, rusqlite::Error> {
  fn convert_err(self) -> Result<T> {
    self.map_err(|err| format!("sqlite error: {}", err).into())
  }
}

#[derive(Debug)]
pub struct DocData {
  pub index: Vec<DocIndexItem>,
  connection: rusqlite::Connection,
}

#[derive(Debug, Clone)]
pub struct DocIndexItem {
  pub name: String,
  pub document_id: i32,
  pub anchor: Option<String>,
}

use std::io::Read;

use compress;

impl DocData {
  pub fn document(&self, id: i32) -> Result<Document> {
    let file_data_query = "select Data from FileDataTable where Id==?";
    let mut file_data_query = self.connection.prepare(file_data_query).convert_err()?;
    let mut file_data = file_data_query.query(&[&id]).convert_err()?;
    let file_data = file_data.next().chain_err(|| "invalid file id")?;
    let file_data = file_data.convert_err()?;
    let file_data: Vec<u8> = file_data.get_checked(0).convert_err()?;
    let mut file_html = Vec::new();
    compress::zlib::Decoder::new(&file_data[4..]).read_to_end(&mut file_html)
      .map_err(|err| format!("zlib decoder failed: {}", err))?;
    let file_html = String::from_utf8_lossy(&file_html);
    Ok(Document::from(file_html.as_ref()))
  }

  pub fn file_name(&self, id: i32) -> Result<String> {
    let query = "select Name from FileNameTable where FileId==?";
    let mut query = self.connection.prepare(query).convert_err()?;
    let mut result = query.query(&[&id]).convert_err()?;
    let row = result.next().chain_err(|| "invalid file id")?;
    let row = row.convert_err()?;
    row.get_checked(0).convert_err()
  }
}

pub fn decode_doc(qt_sub_lib_name: &str) -> Result<DocData> {
  let doc_path = run_qmake_query("QT_INSTALL_DOCS")?;
  if !doc_path.exists() {
    return Err(format!("Documentation directory does not exist: {}",
                       doc_path.display())
      .into());
  }
  let doc_file_path = doc_path.with_added(format!("qt{}.qch", qt_sub_lib_name));
  if !doc_file_path.exists() {
    return Err(format!("Documentation file does not exist: {}",
                       doc_file_path.display())
      .into());
  }
  log::info(format!("Loading Qt documentation from {}", doc_file_path.display()));
  let connection = rusqlite::Connection::open_with_flags(&doc_file_path,
                                                         rusqlite::SQLITE_OPEN_READ_ONLY)
      .convert_err()?;

  let mut index_data = Vec::new();
  {
    let index_query = "select IndexTable.Identifier, IndexTable.FileId, IndexTable.Anchor \
                       from IndexTable";
    let mut index = connection.prepare(index_query).convert_err()?;
    let mut index_rows = index.query(&[]).convert_err()?;
    while let Some(index_row) = index_rows.next() {
      let index_row = index_row.convert_err()?;
      let name: String = index_row.get_checked(0).convert_err()?;
      let file_id: i32 = index_row.get_checked(1).convert_err()?;
      let anchor: Option<String> = index_row.get_checked(2).convert_err()?;
      index_data.push(DocIndexItem {
        name: name,
        document_id: file_id,
        anchor: anchor,
      });
    }
  }
  Ok(DocData {
    index: index_data,
    connection: connection,
  })
}
