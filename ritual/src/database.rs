use crate::cpp_checks::{CppChecks, CppChecksItem};
use crate::cpp_data::CppItem;
use crate::cpp_ffi_data::CppFfiItem;
use crate::rust_info::RustItem;
use crate::rust_type::RustPath;
use log::{debug, info, trace};
use ritual_common::errors::{bail, err_msg, format_err, Result};
use ritual_common::string_utils::ends_with_digit;
use ritual_common::target::LibraryTarget;
use serde_derive::{Deserialize, Serialize};
use std::fmt;

pub struct ItemWithSource<T> {
    pub source_id: ItemId,
    pub value: T,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DbItem<T> {
    pub id: ItemId,
    pub source_id: Option<ItemId>,
    pub item: T,
}

impl<T> DbItem<T> {
    pub fn as_ref(&self) -> DbItem<&T> {
        DbItem {
            id: self.id,
            source_id: self.source_id,
            item: &self.item,
        }
    }

    pub fn as_mut(&mut self) -> DbItem<&mut T> {
        DbItem {
            id: self.id,
            source_id: self.source_id,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(u32);

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ItemId {
    pub fn from_u32(value: u32) -> Self {
        ItemId(value)
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
        if let DatabaseItemData::CppItem(_) = self {
            true
        } else {
            false
        }
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
        if let DatabaseItemData::FfiItem(_) = self {
            true
        } else {
            false
        }
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
        if let DatabaseItemData::RustItem(_) = self {
            true
        } else {
            false
        }
    }
    pub fn as_rust_item(&self) -> Option<&RustItem> {
        if let DatabaseItemData::RustItem(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn is_cpp_checks_item(&self) -> bool {
        if let DatabaseItemData::CppChecksItem(_) = self {
            true
        } else {
            false
        }
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
        if let DatabaseItemData::DocItem(_) = self {
            true
        } else {
            false
        }
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    crate_name: String,
    crate_version: String,
    items: Vec<DbItem<DatabaseItemData>>,
    targets: Vec<LibraryTarget>,
    next_id: u32,
}

#[derive(Debug, Default)]
pub struct Counters {
    pub items_added: u32,
    pub items_ignored: u32,
}

/// Represents all collected data related to a crate.
#[derive(Debug)]
pub struct Database {
    data: Data,
    is_modified: bool,
    counters: Counters,
}

impl Database {
    pub fn new(data: Data) -> Database {
        Database {
            data,
            is_modified: false,
            counters: Counters::default(),
        }
    }

    pub fn data(&self) -> &Data {
        &self.data
    }

    pub fn empty(crate_name: impl Into<String>) -> Self {
        let crate_name = crate_name.into();
        Database {
            data: Data {
                crate_name: crate_name.clone(),
                crate_version: "0.0.0".into(),
                items: Vec::new(),
                targets: Vec::new(),
                next_id: 1,
            },
            is_modified: true,
            counters: Counters::default(),
        }
    }

    pub fn is_modified(&self) -> bool {
        self.is_modified
    }

    pub fn set_saved(&mut self) {
        self.is_modified = false;
    }

    pub fn items(&self) -> impl Iterator<Item = DbItem<&DatabaseItemData>> {
        self.data.items.iter().map(|item| item.as_ref())
    }
    pub fn items_mut(&mut self) -> impl Iterator<Item = DbItem<&mut DatabaseItemData>> {
        self.data.items.iter_mut().map(|item| item.as_mut())
    }
    pub fn cpp_items(&self) -> impl Iterator<Item = DbItem<&CppItem>> {
        self.items()
            .filter_map(|item| item.filter_map(|v| v.as_cpp_item()))
    }
    pub fn cpp_items_mut(&mut self) -> impl Iterator<Item = DbItem<&mut CppItem>> {
        self.items_mut()
            .filter_map(|item| item.filter_map(|v| v.as_cpp_item_mut()))
    }

    pub fn ffi_items(&self) -> impl Iterator<Item = DbItem<&CppFfiItem>> {
        self.items()
            .filter_map(|item| item.filter_map(|v| v.as_ffi_item()))
    }
    pub fn ffi_items_mut(&mut self) -> impl Iterator<Item = DbItem<&mut CppFfiItem>> {
        self.items_mut()
            .filter_map(|item| item.filter_map(|v| v.as_ffi_item_mut()))
    }

    pub fn rust_items(&self) -> impl Iterator<Item = DbItem<&RustItem>> {
        self.items()
            .filter_map(|item| item.filter_map(|v| v.as_rust_item()))
    }

    pub fn cpp_item_ids<'a>(&'a self) -> impl Iterator<Item = ItemId> + 'a {
        self.cpp_items().map(|item| item.id)
    }

    pub fn ffi_item_ids<'a>(&'a self) -> impl Iterator<Item = ItemId> + 'a {
        self.ffi_items().map(|item| item.id)
    }

    pub fn rust_item_ids<'a>(&'a self) -> impl Iterator<Item = ItemId> + 'a {
        self.rust_items().map(|item| item.id)
    }

    pub fn item(&self, id: ItemId) -> Result<DbItem<&DatabaseItemData>> {
        match self.data.items.binary_search_by_key(&id, |item| item.id) {
            Ok(index) => Ok(self.data.items[index].as_ref()),
            Err(_) => bail!("invalid item id: {}", id),
        }
    }

    // TODO: try to remove this
    pub fn item_mut(&mut self, id: ItemId) -> Result<DbItem<&mut DatabaseItemData>> {
        match self.data.items.binary_search_by_key(&id, |item| item.id) {
            Ok(index) => {
                self.is_modified = true;
                Ok(self.data.items[index].as_mut())
            }
            Err(_) => bail!("invalid item id: {}", id),
        }
    }

    pub fn cpp_item(&self, id: ItemId) -> Result<DbItem<&CppItem>> {
        self.item(id)?
            .filter_map(|v| v.as_cpp_item())
            .ok_or_else(|| err_msg("not a cpp item"))
    }

    pub fn cpp_item_mut(&mut self, id: ItemId) -> Result<DbItem<&mut CppItem>> {
        self.item_mut(id)?
            .filter_map(|v| v.as_cpp_item_mut())
            .ok_or_else(|| err_msg("not a cpp item"))
    }

    pub fn ffi_item(&self, id: ItemId) -> Result<DbItem<&CppFfiItem>> {
        self.item(id)?
            .filter_map(|v| v.as_ffi_item())
            .ok_or_else(|| err_msg("not a ffi item"))
    }

    pub fn ffi_item_mut(&mut self, id: ItemId) -> Result<DbItem<&mut CppFfiItem>> {
        self.item_mut(id)?
            .filter_map(|v| v.as_ffi_item_mut())
            .ok_or_else(|| err_msg("not a ffi item"))
    }

    pub fn rust_item(&self, id: ItemId) -> Result<DbItem<&RustItem>> {
        self.item(id)?
            .filter_map(|v| v.as_rust_item())
            .ok_or_else(|| err_msg("not a rust item"))
    }

    pub fn source_ffi_item(&self, id: ItemId) -> Result<Option<DbItem<&CppFfiItem>>> {
        let mut current_item = self.item(id)?;
        loop {
            let new_id = if let Some(id) = current_item.source_id {
                id
            } else {
                return Ok(None);
            };
            current_item = self.item(new_id)?;
            if let Some(item) = current_item.filter_map(|i| i.as_ffi_item()) {
                return Ok(Some(item));
            }
        }
    }

    pub fn add_ffi_item(
        &mut self,
        source_id: Option<ItemId>,
        item: CppFfiItem,
    ) -> Result<Option<ItemId>> {
        self.is_modified = true;
        if self
            .ffi_items()
            .any(|other| other.source_id == source_id && other.item.has_same_kind(&item))
        {
            self.counters.items_ignored += 1;
            return Ok(None);
        }

        let id = ItemId(self.data.next_id);
        self.data.next_id += 1;

        debug!("added ffi item #{}: {}", id, item.short_text());
        trace!("    ffi item data: {:?}", item);
        self.data.items.push(DbItem {
            id,
            source_id,
            item: DatabaseItemData::FfiItem(item),
        });
        self.counters.items_added += 1;
        Ok(Some(id))
    }

    pub fn clear(&mut self) {
        self.is_modified = true;
        self.data.items.clear();
        self.data.targets.clear();
    }

    fn collect_garbage(&mut self) {
        // remove items with dead source
        unimplemented!()
    }

    pub fn crate_name(&self) -> &str {
        &self.data.crate_name
    }

    pub fn crate_version(&self) -> &str {
        &self.data.crate_version
    }

    pub fn set_crate_version(&mut self, version: String) {
        if self.data.crate_version != version {
            self.is_modified = true;
            self.data.crate_version = version;
        }
    }

    pub fn add_cpp_item(
        &mut self,
        source_id: Option<ItemId>,
        data: CppItem,
    ) -> Result<Option<ItemId>> {
        if self.cpp_items().any(|item| item.item.is_same(&data)) {
            self.counters.items_ignored += 1;
            return Ok(None);
        }
        self.is_modified = true;
        let id = ItemId(self.data.next_id);
        self.data.next_id += 1;
        debug!("added cpp item #{}: {}", id, data);
        let item = DbItem {
            id,
            source_id,
            item: DatabaseItemData::CppItem(data),
        };
        trace!("    cpp item data: {:?}", item);
        self.data.items.push(item);
        self.counters.items_added += 1;
        Ok(Some(id))
    }

    pub fn clear_rust_info(&mut self) {
        self.is_modified = true;
        self.data.items.retain(|item| !item.item.is_rust_item());
        self.collect_garbage();
    }

    pub fn add_environment(&mut self, env: LibraryTarget) {
        if !self.data.targets.iter().any(|e| e == &env) {
            self.is_modified = true;
            self.data.targets.push(env.clone());
        }
    }

    pub fn environments(&self) -> &[LibraryTarget] {
        &self.data.targets
    }

    pub fn find_rust_item(&self, path: &RustPath) -> Option<DbItem<&RustItem>> {
        self.rust_items()
            .find(|item| item.item.path() == Some(path))
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
            let crate_name = item_path
                .crate_name()
                .expect("rust item path must have crate name");
            if crate_name != self.data.crate_name {
                bail!("can't add rust item with different crate name: {:?}", item);
            }
        } else {
            let mut path = item
                .parent_path()
                .map_err(|_| format_err!("path has no parent for rust item: {:?}", item))?;
            let crate_name = path
                .crate_name()
                .expect("rust item path must have crate name");
            if crate_name != self.data.crate_name {
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
            .rust_items()
            .any(|other| other.source_id == source_id && other.item.has_same_kind(&item))
        {
            self.counters.items_ignored += 1;
            return Ok(None);
        }

        let id = ItemId(self.data.next_id);
        self.data.next_id += 1;

        debug!("added rust item #{}: {}", id, item.short_text());
        trace!("    rust item data: {:?}", item);
        self.data.items.push(DbItem {
            id,
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
        self.counters = Counters::default();
    }

    pub fn add_cpp_checks_item(&mut self, source_id: ItemId, item: CppChecksItem) -> ItemId {
        // we can't use `self.items_mut()` because of borrow checker
        if let Some(old_item) = self
            .data
            .items
            .iter_mut()
            .map(|item| item.as_mut())
            .filter_map(|i| i.filter_map(|i| i.as_cpp_checks_item_mut()))
            .find(|i| i.source_id == Some(source_id) && i.item.env == item.env)
        {
            if *old_item.item != item {
                *old_item.item = item;
                self.counters.items_added += 1;
            } else {
                self.counters.items_ignored += 1;
            }
            return old_item.id;
        }

        let id = ItemId(self.data.next_id);
        self.data.next_id += 1;

        self.data.items.push(DbItem {
            id,
            source_id: Some(source_id),
            item: DatabaseItemData::CppChecksItem(item),
        });
        self.counters.items_added += 1;
        id
    }

    pub fn add_doc_item(&mut self, source_id: ItemId, item: DocItem) -> ItemId {
        // we can't use `self.items_mut()` because of borrow checker
        if let Some(old_item) = self
            .data
            .items
            .iter_mut()
            .map(|item| item.as_mut())
            .filter_map(|i| i.filter_map(|i| i.as_doc_item_mut()))
            .find(|i| i.source_id == Some(source_id))
        {
            if *old_item.item != item {
                *old_item.item = item;
                self.counters.items_added += 1;
            } else {
                self.counters.items_ignored += 1;
            }
            return old_item.id;
        }

        let id = ItemId(self.data.next_id);
        self.data.next_id += 1;

        self.data.items.push(DbItem {
            id,
            source_id: Some(source_id),
            item: DatabaseItemData::DocItem(item),
        });
        self.counters.items_added += 1;
        id
    }

    pub fn cpp_checks(&self, source_id: ItemId) -> CppChecks {
        let items = self
            .items()
            .filter_map(|i| i.filter_map(|i| i.as_cpp_checks_item()))
            .filter(move |i| i.source_id == Some(source_id))
            .map(|i| i.item.clone());
        CppChecks::new(items)
    }

    pub fn delete_items(&mut self, mut function: impl FnMut(DbItem<&DatabaseItemData>) -> bool) {
        self.data.items.retain(|i| !function(i.as_ref()));
        self.collect_garbage();
    }

    pub fn find_doc_for(&self, id: ItemId) -> Result<Option<DbItem<&DocItem>>> {
        let mut current_item = self.item(id)?;
        loop {
            if let Some(doc) = self
                .items()
                .filter_map(|i| i.filter_map(|i| i.as_doc_item()))
                .find(|i| i.source_id == Some(current_item.id))
            {
                return Ok(Some(doc));
            }

            let new_id = if let Some(id) = current_item.source_id {
                id
            } else {
                return Ok(None);
            };
            current_item = self.item(new_id)?;
        }
    }
}
