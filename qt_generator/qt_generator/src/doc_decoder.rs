//! Converts Qt docs from the internal format.

use cpp_to_rust_generator::common::errors::{ChainErr, Result};
use cpp_to_rust_generator::common::file_utils::PathBufWithAdded;
use cpp_to_rust_generator::common::log;
use html_parser::document::Document;
use rusqlite;

use std::path::Path;

/// Convenience trait for converting `rusqlite` errors to `cpp_to_rust_common` errors.
trait ConvertError<T> {
    fn convert_err(self) -> Result<T>;
}

impl<T> ConvertError<T> for ::std::result::Result<T, rusqlite::Error> {
    fn convert_err(self) -> Result<T> {
        self.map_err(|err| format!("sqlite error: {}", err).into())
    }
}

/// Decoded documentation data.
#[derive(Debug)]
pub struct DocData {
    index: Vec<DocIndexItem>,
    connection: rusqlite::Connection,
}

/// An item of the documentation's index.
#[derive(Debug, Clone)]
pub struct DocIndexItem {
    /// Identifier in the index.
    /// For types and methods, this is roughly the same as
    /// the fully qualified name but has some quirks.
    pub name: String,
    /// Identifier of the virtual HTML file this item refers to.
    pub document_id: i32,
    /// HTML link anchor in the file this item refers to,
    /// or `None` if the item refers to the entire file (e.g. if it's a class).
    pub anchor: Option<String>,
    /// If `true`, this item was previously accessed using `DocData::find_index_item`.
    pub accessed: bool,
}

use std::io::Read;

use compress;

impl DocData {
    /// Returns all index items.
    pub fn index(&self) -> &[DocIndexItem] {
        &self.index
    }

    /// Returns parsed HTML document by id.
    pub fn document(&self, id: i32) -> Result<Document> {
        let file_data_query = "select Data from FileDataTable where Id==?";
        let mut file_data_query = self.connection.prepare(file_data_query).convert_err()?;
        let mut file_data = file_data_query.query(&[&id]).convert_err()?;
        let file_data = file_data.next().chain_err(|| "invalid file id")?;
        let file_data = file_data.convert_err()?;
        let file_data: Vec<u8> = file_data.get_checked(0).convert_err()?;
        let mut file_html = Vec::new();
        compress::zlib::Decoder::new(&file_data[4..])
            .read_to_end(&mut file_html)
            .map_err(|err| format!("zlib decoder failed: {}", err))?;
        let file_html = String::from_utf8_lossy(&file_html);
        Ok(Document::from(file_html.as_ref()))
    }

    /// Returns virtual file name of the document by id.
    pub fn file_name(&self, id: i32) -> Result<String> {
        let query = "select Name from FileNameTable where FileId==?";
        let mut query = self.connection.prepare(query).convert_err()?;
        let mut result = query.query(&[&id]).convert_err()?;
        let row = result.next().chain_err(|| "invalid file id")?;
        let row = row.convert_err()?;
        row.get_checked(0).convert_err()
    }

    /// Searches for an index item by lambda condition.
    pub fn find_index_item<F: Fn(&DocIndexItem) -> bool>(&mut self, f: F) -> Option<DocIndexItem> {
        self.index.iter_mut().find(|item| f(item)).and_then(|item| {
            item.accessed = true;
            Some(item.clone())
        })
    }

    /// Parses Qt documentation of module `qt_crate_name` located at `docs_path`.
    pub fn new(qt_crate_name: &str, docs_path: &Path) -> Result<DocData> {
        if !docs_path.exists() {
            return Err(format!(
                "Documentation directory does not exist: {}",
                docs_path.display()
            )
            .into());
        }
        let doc_file_name = if qt_crate_name.starts_with("3d_") {
            "3d".to_string()
        } else {
            qt_crate_name.replace("_", "")
        };

        let doc_file_path = docs_path.with_added(format!("{}.qch", doc_file_name));
        if !doc_file_path.exists() {
            return Err(format!(
                "Documentation file does not exist: {}",
                doc_file_path.display()
            )
            .into());
        }
        log::status(format!(
            "Adding Qt documentation from {}",
            doc_file_path.display()
        ));
        let connection =
            rusqlite::Connection::open_with_flags(&doc_file_path, rusqlite::SQLITE_OPEN_READ_ONLY)
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
                    accessed: false,
                });
            }
        }
        Ok(DocData {
            index: index_data,
            connection: connection,
        })
    }
}
