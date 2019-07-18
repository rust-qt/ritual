use crate::cpp_checks::CppChecks;
use crate::cpp_code_generator;
use crate::cpp_data::{CppItem, CppOriginLocation, CppPath};
use crate::cpp_ffi_data::{CppFfiFunction, CppFfiItem, QtSlotWrapper};
use crate::rust_info::{RustDatabase, RustDatabaseItem};
use log::{debug, trace};
use ritual_common::errors::{bail, Result};
use ritual_common::target::LibraryTarget;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseItemSource {
    CppParser {
        /// File name of the include file (without full path)
        include_file: String,
        /// Exact location of the declaration
        origin_location: CppOriginLocation,
    },
    ImplicitMethod,
    TemplateInstantiation,
    NamespaceInferring,
    OmittingArguments,
}

impl DatabaseItemSource {
    pub fn is_parser(&self) -> bool {
        match *self {
            DatabaseItemSource::CppParser { .. } => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppFfiDatabaseItem {
    pub item: CppFfiItem,
    pub source_ffi_item: Option<usize>,
    pub checks: CppChecks,
    pub is_rust_processed: bool,
}

impl CppFfiDatabaseItem {
    pub fn from_function(function: CppFfiFunction, source_ffi_item: Option<usize>) -> Self {
        CppFfiDatabaseItem {
            item: CppFfiItem::Function(function),
            source_ffi_item,
            checks: CppChecks::default(),
            is_rust_processed: false,
        }
    }

    pub fn from_qt_slot_wrapper(wrapper: QtSlotWrapper, source_ffi_item: Option<usize>) -> Self {
        CppFfiDatabaseItem {
            item: CppFfiItem::QtSlotWrapper(wrapper),
            source_ffi_item,
            checks: CppChecks::default(),
            is_rust_processed: false,
        }
    }

    pub fn path(&self) -> &CppPath {
        match &self.item {
            CppFfiItem::Function(f) => &f.path,
            CppFfiItem::QtSlotWrapper(s) => &s.class_path,
        }
    }

    pub fn is_source_item(&self) -> bool {
        match &self.item {
            CppFfiItem::Function(_) => false,
            CppFfiItem::QtSlotWrapper(_) => true,
        }
    }

    pub fn source_item_cpp_code(&self) -> Result<String> {
        match &self.item {
            CppFfiItem::Function(_) => bail!("not a source item"),
            CppFfiItem::QtSlotWrapper(slot_wrapper) => {
                cpp_code_generator::qt_slot_wrapper(slot_wrapper)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppDatabaseItem {
    pub item: CppItem,
    pub source: DatabaseItemSource,
    pub source_ffi_item: Option<usize>,
    pub is_cpp_ffi_processed: bool,
    pub is_rust_processed: bool,
}

/// Represents all collected data related to a crate.
#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    crate_name: String,
    crate_version: String,
    cpp_items: Vec<CppDatabaseItem>,
    ffi_items: Vec<CppFfiDatabaseItem>,
    rust_database: RustDatabase,
    environments: Vec<LibraryTarget>,
    #[serde(skip)]
    is_modified: bool,
}

impl Database {
    pub fn empty(crate_name: impl Into<String>) -> Self {
        let crate_name = crate_name.into();
        Database {
            crate_name: crate_name.clone(),
            crate_version: "0.0.0".into(),
            cpp_items: Vec::new(),
            ffi_items: Vec::new(),
            rust_database: RustDatabase::new(crate_name),
            environments: Vec::new(),
            is_modified: true,
        }
    }

    pub fn is_modified(&self) -> bool {
        self.is_modified
    }

    pub fn set_saved(&mut self) {
        self.is_modified = false;
    }

    pub fn cpp_items(&self) -> &[CppDatabaseItem] {
        &self.cpp_items
    }

    pub fn cpp_items_mut(&mut self) -> &mut [CppDatabaseItem] {
        self.is_modified = true;
        &mut self.cpp_items
    }

    pub fn ffi_items(&self) -> &[CppFfiDatabaseItem] {
        &self.ffi_items
    }

    pub fn ffi_items_mut(&mut self) -> &mut [CppFfiDatabaseItem] {
        self.is_modified = true;
        &mut self.ffi_items
    }

    pub fn add_ffi_item(&mut self, item: CppFfiDatabaseItem) {
        self.is_modified = true;
        self.ffi_items.push(item);
    }

    pub fn add_ffi_items(&mut self, items: Vec<CppFfiDatabaseItem>) {
        self.is_modified = true;
        self.ffi_items.extend(items);
    }

    pub fn clear(&mut self) {
        self.is_modified = true;
        self.cpp_items.clear();
        self.environments.clear();
    }

    pub fn clear_ffi(&mut self) {
        self.is_modified = true;
        self.ffi_items.clear();
        for item in &mut self.cpp_items {
            item.is_cpp_ffi_processed = false;
        }
        self.cpp_items.retain(|item| item.source_ffi_item.is_none());
        // TODO: deal with rust items that now have invalid index references
    }

    pub fn clear_cpp_checks(&mut self) {
        self.is_modified = true;
        for item in &mut self.ffi_items {
            item.checks.clear();
        }
    }

    pub fn crate_name(&self) -> &str {
        &self.crate_name
    }

    pub fn crate_version(&self) -> &str {
        &self.crate_version
    }

    pub fn set_crate_version(&mut self, version: String) {
        if self.crate_version != version {
            self.is_modified = true;
            self.crate_version = version;
        }
    }

    pub fn add_cpp_item(
        &mut self,
        source: DatabaseItemSource,
        source_ffi_item: Option<usize>,
        data: CppItem,
    ) -> bool {
        if let Some(item) = self
            .cpp_items
            .iter_mut()
            .find(|item| item.item.is_same(&data))
        {
            // parser data takes priority
            if source.is_parser() && !item.source.is_parser() {
                item.source = source;
            }
            return false;
        }
        self.is_modified = true;
        debug!("added cpp item: {}, source: {:?}", data, source);
        let item = CppDatabaseItem {
            item: data,
            source,
            source_ffi_item,
            is_cpp_ffi_processed: false,
            is_rust_processed: false,
        };
        trace!("cpp item data: {:?}", item);
        self.cpp_items.push(item);
        true
    }

    pub fn rust_database(&self) -> &RustDatabase {
        &self.rust_database
    }

    pub fn rust_items(&self) -> &[RustDatabaseItem] {
        self.rust_database.items()
    }

    pub fn add_rust_item(&mut self, item: RustDatabaseItem) -> Result<()> {
        self.is_modified = true;
        self.rust_database.add_item(item)
    }

    pub fn clear_rust_info(&mut self) {
        self.is_modified = true;
        self.rust_database.clear();
        for item in &mut self.cpp_items {
            item.is_rust_processed = false;
        }
        for item in &mut self.ffi_items {
            item.is_rust_processed = false;
        }
    }

    pub fn add_environment(&mut self, env: LibraryTarget) {
        if !self.environments.iter().any(|e| e == &env) {
            self.is_modified = true;
            self.environments.push(env.clone());
        }
    }

    pub fn environments(&self) -> &[LibraryTarget] {
        &self.environments
    }
}
