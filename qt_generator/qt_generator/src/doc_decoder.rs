//! Converts Qt docs from the internal format.

use cpp_to_rust_generator::common::errors::{bail, err_msg, Result, ResultExt};
use log::info;
use rusqlite;
use select::document::Document;
use std::path::Path;

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
        let mut file_data_query = self.connection.prepare(file_data_query)?;
        let mut file_data = file_data_query.query(&[&id])?;
        let file_data = file_data.next().ok_or_else(|| err_msg("invalid file id"))?;
        let file_data = file_data?;
        let file_data: Vec<u8> = file_data.get_checked(0)?;
        let mut file_html = Vec::new();
        compress::zlib::Decoder::new(&file_data[4..])
            .read_to_end(&mut file_html)
            .with_context(|_| "zlib decoder failed")?;
        let file_html = String::from_utf8_lossy(&file_html);
        Ok(Document::from(file_html.as_ref()))
    }

    /// Returns virtual file name of the document by id.
    pub fn file_name(&self, id: i32) -> Result<String> {
        let query = "select Name from FileNameTable where FileId==?";
        let mut query = self.connection.prepare(query)?;
        let mut result = query.query(&[&id])?;
        let row = result.next().ok_or_else(|| err_msg("invalid file id"))?;
        let row = row?;
        Ok(row.get_checked(0)?)
    }

    /// Searches for an index item by lambda condition.
    pub fn find_index_item<F: Fn(&DocIndexItem) -> bool>(&mut self, f: F) -> Option<DocIndexItem> {
        self.index.iter_mut().find(|item| f(item)).and_then(|item| {
            item.accessed = true;
            Some(item.clone())
        })
    }

    /// Parses Qt documentation of module `qt_crate_name` located at `docs_path`.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(qt_crate_name: &str, docs_path: &Path) -> Result<DocData> {
        if !docs_path.exists() {
            bail!(
                "Documentation directory does not exist: {}",
                docs_path.display()
            );
        }
        let doc_file_name = if qt_crate_name.starts_with("3d_") {
            "3d".to_string()
        } else {
            qt_crate_name.replace("_", "")
        };

        let doc_file_path = docs_path.join(format!("{}.qch", doc_file_name));
        if !doc_file_path.exists() {
            bail!(
                "Documentation file does not exist: {}",
                doc_file_path.display()
            );
        }
        info!("Adding Qt documentation from {}", doc_file_path.display());
        let connection = rusqlite::Connection::open_with_flags(
            &doc_file_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;

        let mut index_data = Vec::new();
        {
            let index_query = "select IndexTable.Identifier, IndexTable.FileId, IndexTable.Anchor \
                               from IndexTable";
            let mut index = connection.prepare(index_query)?;
            let mut index_rows = index.query(rusqlite::NO_PARAMS)?;
            while let Some(index_row) = index_rows.next() {
                let index_row = index_row?;
                let name: String = index_row.get_checked(0)?;
                let file_id: i32 = index_row.get_checked(1)?;
                let anchor: Option<String> = index_row.get_checked(2)?;
                index_data.push(DocIndexItem {
                    name,
                    document_id: file_id,
                    anchor,
                    accessed: false,
                });
            }
        }
        Ok(DocData {
            index: index_data,
            connection,
        })
    }
}
