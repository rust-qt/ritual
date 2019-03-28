use crate::cpp_code_generator;
use crate::cpp_data::{
    CppBaseSpecifier, CppClassField, CppEnumValue, CppOriginLocation, CppPath, CppTypeDeclaration,
    CppTypeDeclarationKind, CppVisibility,
};
use crate::cpp_ffi_data::{CppFfiFunction, QtSlotWrapper};
use crate::cpp_function::CppFunction;
use crate::cpp_type::CppType;
use crate::rust_info::{RustDatabase, RustDatabaseItem};
use itertools::Itertools;
use log::{debug, trace};
use ritual_common::errors::{bail, Result};
use ritual_common::target::Target;
use serde_derive::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct CppCheckerEnv {
    pub target: Target,
    pub cpp_library_version: Option<String>,
}

impl CppCheckerEnv {
    pub fn short_text(&self) -> String {
        format!(
            "{}/{:?}-{:?}-{:?}-{:?}",
            self.cpp_library_version
                .as_ref()
                .map_or("None", |s| s.as_str()),
            self.target.arch,
            self.target.os,
            self.target.family,
            self.target.env
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseItemSource {
    CppParser {
        /// File name of the include file (without full path)
        include_file: String,
        /// Exact location of the declaration
        origin_location: CppOriginLocation,
    },
    ImplicitXstructor,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CppChecksItem {
    pub env: CppCheckerEnv,
    pub is_success: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CppChecks(Vec<CppChecksItem>);

impl CppChecks {
    pub fn has_env(&self, env: &CppCheckerEnv) -> bool {
        self.0.iter().any(|item| &item.env == env)
    }

    pub fn add(&mut self, env: CppCheckerEnv, is_success: bool) {
        self.0.retain(|item| item.env != env);
        self.0.push(CppChecksItem { env, is_success });
    }

    pub fn any_success(&self) -> bool {
        self.0.iter().any(|item| item.is_success)
    }

    pub fn all_success(&self) -> bool {
        self.0.iter().all(|item| item.is_success)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum CppItem {
    Namespace(CppPath),
    Type(CppTypeDeclaration),
    EnumValue(CppEnumValue),
    Function(CppFunction),
    ClassField(CppClassField),
    ClassBase(CppBaseSpecifier),
}

impl CppItem {
    pub fn is_same(&self, other: &CppItem) -> bool {
        use self::CppItem::*;

        match self {
            Namespace(v) => {
                if let Namespace(v2) = &other {
                    v == v2
                } else {
                    false
                }
            }
            Type(v) => {
                if let Type(v2) = &other {
                    v.is_same(v2)
                } else {
                    false
                }
            }
            EnumValue(v) => {
                if let EnumValue(v2) = &other {
                    v.is_same(v2)
                } else {
                    false
                }
            }
            Function(v) => {
                if let Function(v2) = &other {
                    v.is_same(v2)
                } else {
                    false
                }
            }
            ClassField(v) => {
                if let ClassField(v2) = &other {
                    v.is_same(v2)
                } else {
                    false
                }
            }
            ClassBase(v) => {
                if let ClassBase(v2) = &other {
                    v == v2
                } else {
                    false
                }
            }
        }
    }

    pub fn path(&self) -> Option<&CppPath> {
        let path = match self {
            CppItem::Namespace(data) => data,
            CppItem::Type(data) => &data.path,
            CppItem::EnumValue(data) => &data.path,
            CppItem::Function(data) => &data.path,
            CppItem::ClassField(data) => &data.path,
            CppItem::ClassBase(_) => return None,
        };
        Some(path)
    }

    pub fn all_involved_types(&self) -> Vec<CppType> {
        match self {
            CppItem::Type(t) => match t.kind {
                CppTypeDeclarationKind::Enum => vec![CppType::Enum {
                    path: t.path.clone(),
                }],
                CppTypeDeclarationKind::Class { .. } => vec![CppType::Class(t.path.clone())],
            },
            CppItem::EnumValue(enum_value) => vec![CppType::Enum {
                path: enum_value
                    .path
                    .parent()
                    .expect("enum value must have parent path"),
            }],
            CppItem::Namespace(_) => Vec::new(),
            CppItem::Function(function) => function.all_involved_types(),
            CppItem::ClassField(field) => {
                let class_type =
                    CppType::Class(field.path.parent().expect("field path must have parent"));
                vec![class_type, field.field_type.clone()]
            }
            CppItem::ClassBase(base) => vec![
                CppType::Class(base.base_class_type.clone()),
                CppType::Class(base.derived_class_type.clone()),
            ],
        }
    }

    pub fn as_namespace_ref(&self) -> Option<&CppPath> {
        if let CppItem::Namespace(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_function_ref(&self) -> Option<&CppFunction> {
        if let CppItem::Function(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_field_ref(&self) -> Option<&CppClassField> {
        if let CppItem::ClassField(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_enum_value_ref(&self) -> Option<&CppEnumValue> {
        if let CppItem::EnumValue(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_base_ref(&self) -> Option<&CppBaseSpecifier> {
        if let CppItem::ClassBase(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_type_ref(&self) -> Option<&CppTypeDeclaration> {
        if let CppItem::Type(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_type_mut(&mut self) -> Option<&mut CppTypeDeclaration> {
        if let CppItem::Type(data) = self {
            Some(data)
        } else {
            None
        }
    }

    /*pub fn path(&self) -> Option<String> {
        unimplemented!()
    }*/
}

impl fmt::Display for CppItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            CppItem::Namespace(path) => format!("namespace {}", path.to_cpp_pseudo_code()),
            CppItem::Type(type1) => match type1.kind {
                CppTypeDeclarationKind::Enum => format!("enum {}", type1.path.to_cpp_pseudo_code()),
                CppTypeDeclarationKind::Class { .. } => {
                    format!("class {}", type1.path.to_cpp_pseudo_code())
                }
            },
            CppItem::Function(method) => method.short_text(),
            CppItem::EnumValue(value) => format!(
                "enum value {} = {}",
                value.path.to_cpp_pseudo_code(),
                value.value
            ),
            CppItem::ClassField(field) => field.short_text(),
            CppItem::ClassBase(class_base) => {
                let virtual_text = if class_base.is_virtual {
                    "virtual "
                } else {
                    ""
                };
                let visibility_text = match class_base.visibility {
                    CppVisibility::Public => "public",
                    CppVisibility::Protected => "protected",
                    CppVisibility::Private => "private",
                };
                let index_text = if class_base.base_index > 0 {
                    format!(" (index: {}", class_base.base_index)
                } else {
                    String::new()
                };
                format!(
                    "class {} : {}{} {}{}",
                    class_base.derived_class_type.to_cpp_pseudo_code(),
                    virtual_text,
                    visibility_text,
                    class_base.base_class_type.to_cpp_pseudo_code(),
                    index_text
                )
            }
        };

        f.write_str(&s)
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CppFfiItem {
    Function(CppFfiFunction),
    QtSlotWrapper(QtSlotWrapper),
}

impl CppFfiItem {
    pub fn as_function_ref(&self) -> Option<&CppFfiFunction> {
        if let CppFfiItem::Function(data) = self {
            Some(data)
        } else {
            None
        }
    }

    pub fn is_slot_wrapper(&self) -> bool {
        if let CppFfiItem::QtSlotWrapper(_) = self {
            true
        } else {
            false
        }
    }

    pub fn short_text(&self) -> String {
        match self {
            CppFfiItem::Function(function) => function.short_text(),
            CppFfiItem::QtSlotWrapper(slot_wrapper) => format!(
                "slot wrapper for ({})",
                slot_wrapper
                    .signal_arguments
                    .iter()
                    .map(|arg| arg.to_cpp_pseudo_code())
                    .join(", ")
            ),
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
    environments: Vec<CppCheckerEnv>,
    #[serde(skip)]
    is_modified: bool,
}

impl Database {
    pub fn empty(crate_name: impl Into<String>) -> Self {
        Database {
            crate_name: crate_name.into(),
            crate_version: "0.0.0".into(),
            cpp_items: Vec::new(),
            ffi_items: Vec::new(),
            rust_database: RustDatabase::default(),
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

    pub fn add_rust_item(&mut self, item: RustDatabaseItem) {
        self.is_modified = true;
        self.rust_database.add_item(item);
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

    pub fn add_environment(&mut self, env: CppCheckerEnv) {
        if !self.environments.iter().any(|e| e == &env) {
            self.is_modified = true;
            self.environments.push(env.clone());
        }
    }
}
