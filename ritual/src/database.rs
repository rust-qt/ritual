use crate::cpp_checks::{CppChecks, CppChecksItem};
use crate::cpp_data::{CppItem, CppPath};
use crate::cpp_ffi_data::CppFfiItem;
use crate::rust_info::RustItem;
use crate::rust_type::RustPath;
use log::{debug, error, info, trace, warn};
use once_cell::sync::OnceCell;
use ritual_common::errors::{bail, err_msg, format_err, Result};
use ritual_common::file_utils::load_json;
use ritual_common::string_utils::ends_with_digit;
use ritual_common::target::LibraryTarget;
use ritual_common::ReadOnly;
use serde_derive::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::iter::once;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{fmt, mem};

pub const CRATE_DB_FILE_NAME: &str = "ritual_db_v1.json";

pub struct DatabaseCache(HashMap<PathBuf, IndexedDatabase>);

impl DatabaseCache {
    pub fn global() -> &'static Mutex<Self> {
        static INSTANCE: OnceCell<Mutex<DatabaseCache>> = OnceCell::new();
        INSTANCE.get_or_init(|| Mutex::new(DatabaseCache(HashMap::new())))
    }

    pub fn get(
        &mut self,
        path: impl AsRef<Path>,
        crate_name: &str,
        allow_load: bool,
        allow_create: bool,
    ) -> Result<IndexedDatabase> {
        let path = PathBuf::from(path.as_ref());
        if allow_load {
            if let Some(r) = self.0.remove(&path) {
                return Ok(r);
            }
            if path.exists() {
                info!("Loading database for {}", crate_name);
                let db = load_json(&path)?;
                return Ok(IndexedDatabase::new(db, path));
            }
        }
        if allow_create {
            let db = Database::empty(crate_name.into());
            return Ok(IndexedDatabase::new(db, path));
        }
        bail!("can't get database for {}", crate_name);
    }

    pub fn put(&mut self, db: IndexedDatabase) {
        let path = db.path.clone();
        let r = self.0.insert(path, db);
        if r.is_some() {
            warn!("duplicate db put in cache");
        }
    }

    pub fn remove_if_exists(&mut self, path: impl AsRef<Path>) {
        let path = PathBuf::from(path.as_ref());
        self.0.remove(&path);
    }
}

pub struct ItemWithSource<T> {
    pub source_id: ItemId,
    pub item: T,
}

impl<T> ItemWithSource<T> {
    pub fn new(source_id: &ItemId, item: T) -> Self {
        Self {
            source_id: source_id.clone(),
            item,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DbItem<T> {
    pub id: ItemId,
    pub source_id: Option<ItemId>,
    pub item: T,
}

impl<T> DbItem<T> {
    pub fn as_ref(&self) -> DbItem<&T> {
        DbItem {
            id: self.id.clone(),
            source_id: self.source_id.clone(),
            item: &self.item,
        }
    }

    pub fn as_mut(&mut self) -> DbItem<&mut T> {
        DbItem {
            id: self.id.clone(),
            source_id: self.source_id.clone(),
            item: &mut self.item,
        }
    }

    pub fn map<U, F>(self, mut func: F) -> DbItem<U>
    where
        F: FnMut(T) -> U,
    {
        DbItem {
            id: self.id,
            source_id: self.source_id,
            item: func(self.item),
        }
    }

    pub fn filter_map<U, F>(self, mut func: F) -> Option<DbItem<U>>
    where
        F: FnMut(T) -> Option<U>,
    {
        Some(DbItem {
            id: self.id,
            source_id: self.source_id,
            item: func(self.item)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId {
    crate_name: Arc<String>,
    id: u32,
}

impl ItemId {
    pub fn new(crate_name: String, id: u32) -> Self {
        Self {
            crate_name: Arc::new(crate_name),
            id,
        }
    }

    pub fn crate_name(&self) -> &str {
        &self.crate_name
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.crate_name, self.id)
    }
}

/// C++ documentation for a method
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct DocItem {
    /// HTML anchor of this documentation entry
    /// (used to detect duplicates)
    pub anchor: Option<String>,
    /// HTML content
    pub html: String,
    /// If the documentation parser couldn't find documentation for the exact same
    /// method, it can still provide documentation entry for the closest match.
    /// In this case, this field should contain C++ declaration of the found method.
    pub mismatched_declaration: Option<String>,
    /// Absolute URL to online documentation page for this method
    pub url: Option<String>,
    /// Absolute documentation URLs encountered in the content
    pub cross_references: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum DatabaseItemData {
    CppItem(CppItem),
    FfiItem(CppFfiItem),
    CppChecksItem(CppChecksItem),
    RustItem(RustItem),
    DocItem(DocItem),
}

impl DatabaseItemData {
    pub fn is_cpp_item(&self) -> bool {
        matches!(self, DatabaseItemData::CppItem(_))
    }
    pub fn as_cpp_item(&self) -> Option<&CppItem> {
        if let DatabaseItemData::CppItem(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_cpp_item_mut(&mut self) -> Option<&mut CppItem> {
        if let DatabaseItemData::CppItem(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn is_ffi_item(&self) -> bool {
        matches!(self, DatabaseItemData::FfiItem(_))
    }
    pub fn as_ffi_item(&self) -> Option<&CppFfiItem> {
        if let DatabaseItemData::FfiItem(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_ffi_item_mut(&mut self) -> Option<&mut CppFfiItem> {
        if let DatabaseItemData::FfiItem(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn is_rust_item(&self) -> bool {
        matches!(self, DatabaseItemData::RustItem(_))
    }
    pub fn as_rust_item(&self) -> Option<&RustItem> {
        if let DatabaseItemData::RustItem(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn is_cpp_checks_item(&self) -> bool {
        matches!(self, DatabaseItemData::CppChecksItem(_))
    }
    pub fn as_cpp_checks_item(&self) -> Option<&CppChecksItem> {
        if let DatabaseItemData::CppChecksItem(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_cpp_checks_item_mut(&mut self) -> Option<&mut CppChecksItem> {
        if let DatabaseItemData::CppChecksItem(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn is_doc_item(&self) -> bool {
        matches!(self, DatabaseItemData::DocItem(_))
    }
    pub fn as_doc_item(&self) -> Option<&DocItem> {
        if let DatabaseItemData::DocItem(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_doc_item_mut(&mut self) -> Option<&mut DocItem> {
        if let DatabaseItemData::DocItem(data) = self {
            Some(data)
        } else {
            None
        }
    }

    pub fn short_text(&self) -> String {
        match self {
            DatabaseItemData::CppItem(item) => item.short_text(),
            DatabaseItemData::FfiItem(item) => item.short_text(),
            DatabaseItemData::RustItem(item) => item.short_text(),
            DatabaseItemData::CppChecksItem(_) => "CppChecksItem".into(),
            DatabaseItemData::DocItem(_) => "DocItem".into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    crate_name: Arc<String>,
    crate_version: String,
    items: Vec<DbItem<DatabaseItemData>>,
    targets: Vec<LibraryTarget>,
    next_id: u32,
}

impl Database {
    pub fn empty(crate_name: String) -> Self {
        Database {
            crate_name: Arc::new(crate_name),
            crate_version: "0.0.0".into(),
            items: Vec::new(),
            targets: Vec::new(),
            next_id: 1,
        }
    }

    fn items(&self) -> impl Iterator<Item = DbItem<&DatabaseItemData>> {
        self.items.iter().map(|item| item.as_ref())
    }
    fn items_mut(&mut self) -> impl Iterator<Item = DbItem<&mut DatabaseItemData>> {
        self.items.iter_mut().map(|item| item.as_mut())
    }
    fn cpp_items(&self) -> impl Iterator<Item = DbItem<&CppItem>> {
        self.items()
            .filter_map(|item| item.filter_map(|v| v.as_cpp_item()))
    }
    fn cpp_items_mut(&mut self) -> impl Iterator<Item = DbItem<&mut CppItem>> {
        self.items_mut()
            .filter_map(|item| item.filter_map(|v| v.as_cpp_item_mut()))
    }
    fn ffi_items(&self) -> impl Iterator<Item = DbItem<&CppFfiItem>> {
        self.items()
            .filter_map(|item| item.filter_map(|v| v.as_ffi_item()))
    }
    fn ffi_items_mut(&mut self) -> impl Iterator<Item = DbItem<&mut CppFfiItem>> {
        self.items_mut()
            .filter_map(|item| item.filter_map(|v| v.as_ffi_item_mut()))
    }
    fn rust_items(&self) -> impl Iterator<Item = DbItem<&RustItem>> {
        self.items()
            .filter_map(|item| item.filter_map(|v| v.as_rust_item()))
    }
}

#[derive(Debug)]
pub struct IndexedDatabase {
    db: Database,
    path: PathBuf,
    source_id_to_index: HashMap<Option<ItemId>, Vec<usize>>,
    cpp_path_to_index: HashMap<CppPath, Vec<usize>>,
    rust_path_to_index: HashMap<RustPath, usize>,
}

impl IndexedDatabase {
    pub fn new(db: Database, path: PathBuf) -> Self {
        let mut value = Self {
            db,
            path,
            source_id_to_index: HashMap::new(),
            cpp_path_to_index: HashMap::new(),
            rust_path_to_index: HashMap::new(),
        };
        value.refresh();
        value
    }

    pub fn database(&self) -> &Database {
        &self.db
    }

    fn refresh(&mut self) {
        self.source_id_to_index.clear();
        self.cpp_path_to_index.clear();
        self.rust_path_to_index.clear();
        for (index, item) in self.db.items.iter().enumerate() {
            self.source_id_to_index
                .entry(item.source_id.clone())
                .or_default()
                .push(index);
            if let Some(path) = item.item.as_rust_item().and_then(|item| item.path()) {
                self.rust_path_to_index.insert(path.clone(), index);
            }
            if let Some(path) = item.item.as_cpp_item().and_then(|item| item.path()) {
                self.cpp_path_to_index
                    .entry(path.clone())
                    .or_default()
                    .push(index);
            }
        }
    }

    fn push(&mut self, item: DbItem<DatabaseItemData>) {
        let index = self.db.items.len();
        self.source_id_to_index
            .entry(item.source_id.clone())
            .or_default()
            .push(index);
        if let Some(path) = item.item.as_rust_item().and_then(|item| item.path()) {
            self.rust_path_to_index.insert(path.clone(), index);
        }
        if let Some(path) = item.item.as_cpp_item().and_then(|item| item.path()) {
            self.cpp_path_to_index
                .entry(path.clone())
                .or_default()
                .push(index);
        }
        self.db.items.push(item);
    }

    fn filter_by_source(
        &self,
        source_id: &Option<ItemId>,
    ) -> impl Iterator<Item = DbItem<&DatabaseItemData>> {
        self.source_id_to_index
            .get(source_id)
            .into_iter()
            .flat_map(move |ids| ids.iter().map(move |&id| self.db.items[id].as_ref()))
    }

    fn filter_by_cpp_path(&self, path: &CppPath) -> impl Iterator<Item = DbItem<&CppItem>> {
        self.cpp_path_to_index
            .get(path)
            .into_iter()
            .flat_map(move |ids| {
                ids.iter().map(move |&id| {
                    self.db.items[id]
                        .as_ref()
                        .map(|item| item.as_cpp_item().expect("invalid db index"))
                })
            })
    }

    fn find_rust_item(&self, path: &RustPath) -> Option<DbItem<&RustItem>> {
        self.rust_path_to_index.get(path).map(|&index| {
            self.db.items[index]
                .as_ref()
                .map(|item| item.as_rust_item().expect("invalid db index"))
        })
    }
}

#[derive(Debug, Default)]
pub struct Counters {
    pub items_added: u32,
    pub items_ignored: u32,
    pub items_deleted: u32,
}

/// Represents all collected data related to a crate.
#[derive(Debug)]
pub struct DatabaseClient {
    current_database: IndexedDatabase,
    dependencies: ReadOnly<Vec<IndexedDatabase>>,
    is_modified: bool,
    counters: Counters,
}

impl Drop for DatabaseClient {
    fn drop(&mut self) {
        let current_database = mem::replace(
            &mut self.current_database,
            IndexedDatabase::new(Database::empty(String::new()), PathBuf::new()),
        );
        let dependencies = mem::replace(&mut self.dependencies, ReadOnly::new(Vec::new()));

        let mut cache = DatabaseCache::global().lock().unwrap();
        cache.put(current_database);
        for dependency in dependencies.into_inner() {
            cache.put(dependency);
        }
    }
}

impl DatabaseClient {
    pub fn new(
        current_database: IndexedDatabase,
        dependencies: ReadOnly<Vec<IndexedDatabase>>,
    ) -> DatabaseClient {
        DatabaseClient {
            current_database,
            dependencies,
            is_modified: false,
            counters: Counters::default(),
        }
    }

    pub fn data(&self) -> &Database {
        &self.current_database.db
    }

    pub fn is_modified(&self) -> bool {
        self.is_modified
    }

    pub fn set_saved(&mut self) {
        self.is_modified = false;
    }

    pub fn items(&self) -> impl Iterator<Item = DbItem<&DatabaseItemData>> {
        self.current_database.db.items()
    }
    pub fn items_mut(&mut self) -> impl Iterator<Item = DbItem<&mut DatabaseItemData>> {
        self.current_database.db.items_mut()
    }
    pub fn cpp_items(&self) -> impl Iterator<Item = DbItem<&CppItem>> {
        self.current_database.db.cpp_items()
    }
    pub fn cpp_items_mut(&mut self) -> impl Iterator<Item = DbItem<&mut CppItem>> {
        self.current_database.db.cpp_items_mut()
    }

    pub fn ffi_items(&self) -> impl Iterator<Item = DbItem<&CppFfiItem>> {
        self.current_database.db.ffi_items()
    }
    pub fn ffi_items_mut(&mut self) -> impl Iterator<Item = DbItem<&mut CppFfiItem>> {
        self.current_database.db.ffi_items_mut()
    }

    pub fn rust_items(&self) -> impl Iterator<Item = DbItem<&RustItem>> {
        self.current_database.db.rust_items()
    }

    pub fn cpp_item_ids(&self) -> impl Iterator<Item = ItemId> + '_ {
        self.cpp_items().map(|item| item.id)
    }

    pub fn ffi_item_ids(&self) -> impl Iterator<Item = ItemId> + '_ {
        self.ffi_items().map(|item| item.id)
    }

    pub fn rust_item_ids(&self) -> impl Iterator<Item = ItemId> + '_ {
        self.rust_items().map(|item| item.id)
    }

    pub fn item(&self, id: &ItemId) -> Result<DbItem<&DatabaseItemData>> {
        let db = self.database(&id.crate_name)?;
        match db.db.items.binary_search_by_key(&id, |item| &item.id) {
            Ok(index) => Ok(db.db.items[index].as_ref()),
            Err(_) => bail!("invalid item id: {}", id),
        }
    }

    // TODO: try to remove this
    pub fn item_mut(&mut self, id: &ItemId) -> Result<DbItem<&mut DatabaseItemData>> {
        if *id.crate_name != self.crate_name() {
            bail!("can't modify item of dependency");
        }
        self.is_modified = true;
        match self
            .current_database
            .db
            .items
            .binary_search_by_key(&id, |item| &item.id)
        {
            Ok(index) => Ok(self.current_database.db.items[index].as_mut()),
            Err(_) => bail!("invalid item id: {}", id),
        }
    }

    pub fn cpp_item(&self, id: &ItemId) -> Result<DbItem<&CppItem>> {
        self.item(id)?
            .filter_map(|v| v.as_cpp_item())
            .ok_or_else(|| err_msg("not a cpp item"))
    }

    pub fn cpp_item_mut(&mut self, id: &ItemId) -> Result<DbItem<&mut CppItem>> {
        self.item_mut(id)?
            .filter_map(|v| v.as_cpp_item_mut())
            .ok_or_else(|| err_msg("not a cpp item"))
    }

    pub fn ffi_item(&self, id: &ItemId) -> Result<DbItem<&CppFfiItem>> {
        self.item(id)?
            .filter_map(|v| v.as_ffi_item())
            .ok_or_else(|| err_msg("not a ffi item"))
    }

    pub fn ffi_item_mut(&mut self, id: &ItemId) -> Result<DbItem<&mut CppFfiItem>> {
        self.item_mut(id)?
            .filter_map(|v| v.as_ffi_item_mut())
            .ok_or_else(|| err_msg("not a ffi item"))
    }

    pub fn rust_item(&self, id: &ItemId) -> Result<DbItem<&RustItem>> {
        self.item(id)?
            .filter_map(|v| v.as_rust_item())
            .ok_or_else(|| err_msg("not a rust item"))
    }

    fn new_id(&mut self) -> ItemId {
        let id = self.current_database.db.next_id;
        self.current_database.db.next_id += 1;
        ItemId {
            crate_name: self.current_database.db.crate_name.clone(),
            id,
        }
    }

    pub fn add_ffi_item(
        &mut self,
        source_id: Option<ItemId>,
        item: CppFfiItem,
    ) -> Result<Option<ItemId>> {
        self.is_modified = true;
        if self
            .current_database
            .filter_by_source(&source_id)
            .filter_map(|other| other.item.as_ffi_item())
            .any(|other| other.has_same_kind(&item))
        {
            self.counters.items_ignored += 1;
            return Ok(None);
        }

        let id = self.new_id();

        debug!("added ffi item {}: {}", id, item.short_text());
        if let Some(source_id) = &source_id {
            debug!("    source: {}", source_id);
        }
        trace!("    ffi item data: {:?}", item);
        self.current_database.push(DbItem {
            id: id.clone(),
            source_id,
            item: DatabaseItemData::FfiItem(item),
        });
        self.counters.items_added += 1;
        Ok(Some(id))
    }

    pub fn crate_name(&self) -> &str {
        &self.current_database.db.crate_name
    }

    pub fn crate_version(&self) -> &str {
        &self.current_database.db.crate_version
    }

    pub fn set_crate_version(&mut self, version: String) {
        if self.current_database.db.crate_version != version {
            self.is_modified = true;
            self.current_database.db.crate_version = version;
        }
    }

    pub fn add_cpp_item_without_hook(
        &mut self,
        source_id: Option<ItemId>,
        data: CppItem,
    ) -> Result<Option<ItemId>> {
        if self.cpp_items().any(|item| item.item.is_same(&data)) {
            self.counters.items_ignored += 1;
            return Ok(None);
        }
        self.is_modified = true;
        let id = self.new_id();
        debug!("added cpp item {}: {}", id, data);
        if let Some(source_id) = &source_id {
            debug!("    source: {}", source_id);
        }
        let item = DbItem {
            id: id.clone(),
            source_id,
            item: DatabaseItemData::CppItem(data),
        };
        trace!("    cpp item data: {:?}", item);
        self.current_database.push(item);
        self.counters.items_added += 1;
        Ok(Some(id))
    }

    pub fn add_environment(&mut self, env: LibraryTarget) {
        if !self.current_database.db.targets.iter().any(|e| e == &env) {
            self.is_modified = true;
            self.current_database.db.targets.push(env);
        }
    }

    pub fn environments(&self) -> &[LibraryTarget] {
        &self.current_database.db.targets
    }

    pub fn find_rust_item(&self, path: &RustPath) -> Option<DbItem<&RustItem>> {
        self.current_database.find_rust_item(path)
    }

    pub fn rust_children<'a>(
        &'a self,
        path: &'a RustPath,
    ) -> impl Iterator<Item = DbItem<&RustItem>> {
        self.rust_items()
            .filter(move |item| item.item.is_child_of(path))
    }

    pub fn add_rust_item(
        &mut self,
        source_id: Option<ItemId>,
        item: RustItem,
    ) -> Result<Option<ItemId>> {
        self.is_modified = true;
        if item.is_crate_root() {
            let item_path = item.path().expect("crate root must have path");
            let crate_name = item_path.crate_name();
            if crate_name != *self.current_database.db.crate_name {
                bail!("can't add rust item with different crate name: {:?}", item);
            }
        } else {
            let mut path = item
                .parent_path()
                .map_err(|_| format_err!("path has no parent for rust item: {:?}", item))?;
            let crate_name = path.crate_name();
            if crate_name != *self.current_database.db.crate_name {
                bail!("can't add rust item with different crate name: {:?}", item);
            }
            while path.parts.len() > 1 {
                if self.find_rust_item(&path).is_none() {
                    bail!("unreachable path {:?} for rust item: {:?}", path, item);
                }
                path.parts.pop();
            }
        }

        if self
            .current_database
            .filter_by_source(&source_id)
            .filter_map(|other| other.item.as_rust_item())
            .any(|other| other.has_same_kind(&item))
        {
            self.counters.items_ignored += 1;
            return Ok(None);
        }

        let id = self.new_id();

        debug!("added rust item {}: {}", id, item.short_text());
        if let Some(source_id) = &source_id {
            debug!("    source: {}", source_id);
        }
        trace!("    rust item data: {:?}", item);
        self.current_database.push(DbItem {
            id: id.clone(),
            source_id,
            item: DatabaseItemData::RustItem(item),
        });
        self.counters.items_added += 1;
        Ok(Some(id))
    }

    pub fn make_unique_rust_path(&self, path: &RustPath) -> RustPath {
        let mut number = None;
        let mut path_try = path.clone();
        loop {
            if let Some(number) = number {
                *path_try.last_mut() = format!(
                    "{}{}{}",
                    path.last(),
                    if ends_with_digit(path.last()) {
                        "_"
                    } else {
                        ""
                    },
                    number
                );
            }
            if self.find_rust_item(&path_try).is_none() {
                return path_try;
            }

            number = Some(number.unwrap_or(1) + 1);
        }
        // TODO: check for conflicts with types from crate template (how?)
    }

    pub fn report_counters(&mut self) {
        if self.counters.items_added > 0 || self.counters.items_ignored > 0 {
            if self.counters.items_ignored == 0 {
                info!("Items added: {}", self.counters.items_added);
            } else {
                info!(
                    "Items added: {}, ignored: {}",
                    self.counters.items_added, self.counters.items_ignored
                );
            }
        }
        if self.counters.items_deleted > 0 {
            info!("Items deleted: {}", self.counters.items_deleted);
        }
        self.counters = Counters::default();
    }

    pub fn add_cpp_checks_item(
        &mut self,
        source_id: ItemId,
        item: CppChecksItem,
    ) -> Option<ItemId> {
        if self
            .current_database
            .filter_by_source(&Some(source_id.clone()))
            .filter_map(|other| other.filter_map(|other| other.as_cpp_checks_item()))
            .any(|other| other.item.env == item.env)
        {
            self.counters.items_ignored += 1;
            return None;
        }

        let id = self.new_id();

        self.current_database.push(DbItem {
            id: id.clone(),
            source_id: Some(source_id),
            item: DatabaseItemData::CppChecksItem(item),
        });
        self.counters.items_added += 1;
        Some(id)
    }

    pub fn add_doc_item(&mut self, source_id: ItemId, item: DocItem) -> Option<ItemId> {
        if self
            .current_database
            .filter_by_source(&Some(source_id.clone()))
            .any(|other| other.item.is_doc_item())
        {
            self.counters.items_ignored += 1;
            return None;
        }

        let id = self.new_id();

        self.current_database.push(DbItem {
            id: id.clone(),
            source_id: Some(source_id),
            item: DatabaseItemData::DocItem(item),
        });
        self.counters.items_added += 1;
        Some(id)
    }

    pub fn cpp_checks(&self, source_id: &ItemId) -> Result<CppChecks> {
        let items = self
            .database(&source_id.crate_name)?
            .filter_by_source(&Some(source_id.clone()))
            .filter_map(|item| item.item.as_cpp_checks_item().cloned());
        Ok(CppChecks::new(items))
    }

    pub fn delete_items(&mut self, mut function: impl FnMut(DbItem<&DatabaseItemData>) -> bool) {
        let mut ids = HashSet::new();
        let mut items_deleted = 0;
        self.current_database.db.items.retain(|i| {
            let result = !function(i.as_ref());
            if !result {
                ids.insert(i.id.clone());
                items_deleted += 1;
            }
            result
        });
        self.counters.items_deleted += items_deleted;
        if items_deleted > 0 {
            self.is_modified = true;
        }
        self.delete_children(ids);
        self.current_database.refresh();
    }

    fn delete_children(&mut self, mut ids: HashSet<ItemId>) {
        let mut items_deleted = 0;
        loop {
            let mut new_ids = HashSet::new();
            self.current_database.db.items.retain(|i| {
                let result = i
                    .source_id
                    .as_ref()
                    .map_or(true, |source_id| !ids.contains(source_id));
                if !result {
                    new_ids.insert(i.id.clone());
                    items_deleted += 1;
                }
                result
            });
            if new_ids.is_empty() {
                break;
            }
            ids = new_ids;
        }
        if items_deleted > 0 {
            self.is_modified = true;
        }
        self.counters.items_deleted += items_deleted;
    }

    pub fn source_cpp_item(&self, id: &ItemId) -> Result<Option<DbItem<&CppItem>>> {
        let mut current_item = self.item(id)?;
        loop {
            let new_id = if let Some(id) = &current_item.source_id {
                id.clone()
            } else {
                return Ok(None);
            };
            current_item = self.item(&new_id)?;
            if let Some(item) = current_item.clone().filter_map(|i| i.as_cpp_item()) {
                return Ok(Some(item));
            }
        }
    }

    pub fn original_cpp_item(&self, id: &ItemId) -> Result<Option<DbItem<&CppItem>>> {
        let mut current_item = self.item(id)?;
        let mut last_cpp_item = None;
        loop {
            if let Some(item) = current_item.clone().filter_map(|i| i.as_cpp_item()) {
                last_cpp_item = Some(item);
            }
            let new_id = if let Some(id) = &current_item.source_id {
                id.clone()
            } else {
                return Ok(last_cpp_item);
            };
            current_item = self.item(&new_id)?;
        }
    }

    pub fn source_ffi_item(&self, id: &ItemId) -> Result<Option<DbItem<&CppFfiItem>>> {
        let mut current_item = self.item(id)?;
        loop {
            let new_id = if let Some(id) = &current_item.source_id {
                id.clone()
            } else {
                return Ok(None);
            };
            current_item = self.item(&new_id)?;
            if let Some(item) = current_item.clone().filter_map(|i| i.as_ffi_item()) {
                return Ok(Some(item));
            }
        }
    }

    pub fn find_doc_for(&self, id: &ItemId) -> Result<Option<DbItem<&DocItem>>> {
        let mut current_item = self.item(id)?;
        loop {
            if let Some(doc) = self
                .database(&current_item.id.crate_name)?
                .filter_by_source(&Some(current_item.id.clone()))
                .filter_map(|i| i.filter_map(|i| i.as_doc_item()))
                .next()
            {
                return Ok(Some(doc));
            }

            let new_id = if let Some(id) = &current_item.source_id {
                id.clone()
            } else {
                return Ok(None);
            };
            current_item = self.item(&new_id)?;
        }
    }

    fn all_databases(&self) -> impl Iterator<Item = &IndexedDatabase> {
        once(&self.current_database as &_).chain(self.dependencies.iter())
    }

    pub fn all_cpp_items(&self) -> impl Iterator<Item = DbItem<&CppItem>> {
        self.all_databases().flat_map(|d| d.db.cpp_items())
    }

    pub fn all_ffi_items(&self) -> impl Iterator<Item = DbItem<&CppFfiItem>> {
        self.all_databases().flat_map(|d| d.db.ffi_items())
    }

    pub fn find_rust_items_for_cpp_path(
        &self,
        cpp_path: &CppPath,
        allow_dependencies: bool,
    ) -> Result<impl Iterator<Item = DbItem<&RustItem>>> {
        let databases = once(&self.current_database as &_)
            .chain(self.dependencies.iter().filter(|_| allow_dependencies));

        for db in databases {
            if let Some(cpp_item) = db.filter_by_cpp_path(cpp_path).next() {
                return Ok(db
                    .filter_by_source(&Some(cpp_item.id))
                    .filter_map(|item| item.filter_map(|item| item.as_rust_item())));
            }
        }

        bail!("unknown cpp path: {}", cpp_path.to_cpp_pseudo_code())
    }

    fn database(&self, crate_name: &str) -> Result<&IndexedDatabase> {
        self.all_databases()
            .find(|db| *db.db.crate_name == crate_name)
            .ok_or_else(|| format_err!("no database found for crate: {}", crate_name))
    }

    pub fn dependency_version(&self, crate_name: &str) -> Result<&str> {
        Ok(&self.database(crate_name)?.db.crate_version)
    }

    pub fn print_item_trace(&self, item_id: &ItemId) -> Result<()> {
        info!("Sources:");
        let mut sources = Vec::new();
        let mut item = self.item(item_id)?;
        loop {
            sources.push(item.clone());
            if let Some(source_id) = &item.source_id {
                match self.item(source_id) {
                    Ok(i) => {
                        item = i;
                    }
                    Err(err) => {
                        error!("source is missing: {}", err);
                        break;
                    }
                }
            } else {
                break;
            }
        }
        for source in sources.iter().rev() {
            info!("{:?}", source);
        }
        info!("Children:");
        self.print_item_children(item_id);
        Ok(())
    }

    fn print_item_children(&self, item_id: &ItemId) {
        let item_id = Some(item_id.clone());
        let children = self
            .all_databases()
            .flat_map(|db| db.filter_by_source(&item_id));

        for child in children {
            info!("{:?}", child);
            self.print_item_children(&child.id);
        }
    }
}
