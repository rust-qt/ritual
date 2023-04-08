use std::sync::Arc;

use ritual_common::target::LibraryTarget;
use serde::{Deserialize, Serialize};

use crate::cpp_data::CppItem;

#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    crate_name: Arc<String>,
    crate_version: String,
    items: Vec<DatabaseItem>,
    targets: Vec<LibraryTarget>,
}

impl Database {
    pub fn add_target(&mut self, target: LibraryTarget) -> usize {
        if let Some(pos) = self.targets.iter().position(|t| t == &target) {
            return pos;
        }
        self.targets.push(target);
        self.targets.len() - 1
    }

    pub fn add_item(&mut self, target: usize, item: CppItem) -> Option<usize> {
        if let Some((_index, i)) = self
            .items
            .iter_mut()
            .enumerate()
            .find(|(_, i)| i.item.is_same(&item))
        {
            if !i.targets.contains(&target) {
                i.targets.push(target);
            }
            return None;
        }
        self.items.push(DatabaseItem {
            targets: vec![target],
            item,
        });
        Some(self.items.len() - 1)
    }
    pub fn items(&self) -> &[DatabaseItem] {
        &self.items
    }
    pub fn items_mut(&mut self) -> &mut [DatabaseItem] {
        &mut self.items
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseItem {
    pub targets: Vec<usize>,
    pub item: CppItem,
}
