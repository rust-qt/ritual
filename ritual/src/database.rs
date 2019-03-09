use crate::cpp_data::CppBaseSpecifier;
use crate::cpp_data::CppClassField;
use crate::cpp_data::CppEnumValue;
use crate::cpp_data::CppOriginLocation;
use crate::cpp_data::CppPath;
use crate::cpp_data::CppTypeDeclaration;
use crate::cpp_data::CppTypeDeclarationKind;
use crate::cpp_data::CppVisibility;
use crate::cpp_ffi_data::QtSlotWrapper;
use crate::cpp_ffi_data::{CppFfiFunction, CppFfiFunctionKind};
use crate::cpp_function::CppFunction;
use crate::cpp_type::CppType;
use crate::rust_info::{RustDatabase, RustDatabaseItem};
use itertools::Itertools;
use log::{debug, trace};
use ritual_common::target::Target;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;
use std::fmt::Formatter;

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
    ImplicitDestructor,
    TemplateInstantiation,
    NamespaceInferring,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum CppItemData {
    Namespace(CppPath),
    Type(CppTypeDeclaration),
    EnumValue(CppEnumValue),
    Function(CppFunction),
    ClassField(CppClassField),
    ClassBase(CppBaseSpecifier),
}

impl CppItemData {
    pub fn is_same(&self, other: &CppItemData) -> bool {
        use self::CppItemData::*;

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
            CppItemData::Namespace(data) => data,
            CppItemData::Type(data) => &data.path,
            CppItemData::EnumValue(data) => &data.path,
            CppItemData::Function(data) => &data.path,
            CppItemData::ClassField(data) => &data.path,
            CppItemData::ClassBase(_) => return None,
        };
        Some(path)
    }

    pub fn all_involved_types(&self) -> Vec<CppType> {
        match self {
            CppItemData::Type(t) => match t.kind {
                CppTypeDeclarationKind::Enum => vec![CppType::Enum {
                    path: t.path.clone(),
                }],
                CppTypeDeclarationKind::Class { .. } => vec![CppType::Class(t.path.clone())],
            },
            CppItemData::EnumValue(enum_value) => vec![CppType::Enum {
                path: enum_value
                    .path
                    .parent()
                    .expect("enum value must have parent path"),
            }],
            CppItemData::Namespace(_) => Vec::new(),
            CppItemData::Function(function) => function.all_involved_types(),
            CppItemData::ClassField(field) => {
                let class_type =
                    CppType::Class(field.path.parent().expect("field path must have parent"));
                vec![class_type, field.field_type.clone()]
            }
            CppItemData::ClassBase(base) => vec![
                CppType::Class(base.base_class_type.clone()),
                CppType::Class(base.derived_class_type.clone()),
            ],
        }
    }

    pub fn as_namespace_ref(&self) -> Option<&CppPath> {
        if let CppItemData::Namespace(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_function_ref(&self) -> Option<&CppFunction> {
        if let CppItemData::Function(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_field_ref(&self) -> Option<&CppClassField> {
        if let CppItemData::ClassField(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_enum_value_ref(&self) -> Option<&CppEnumValue> {
        if let CppItemData::EnumValue(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_base_ref(&self) -> Option<&CppBaseSpecifier> {
        if let CppItemData::ClassBase(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_type_ref(&self) -> Option<&CppTypeDeclaration> {
        if let CppItemData::Type(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_type_mut(&mut self) -> Option<&mut CppTypeDeclaration> {
        if let CppItemData::Type(data) = self {
            Some(data)
        } else {
            None
        }
    }

    /*pub fn path(&self) -> Option<String> {
        unimplemented!()
    }*/
}

impl Display for CppItemData {
    fn fmt(&self, f: &mut Formatter<'_>) -> ::std::result::Result<(), ::std::fmt::Error> {
        let s = match self {
            CppItemData::Namespace(path) => format!("namespace {}", path.to_cpp_pseudo_code()),
            CppItemData::Type(type1) => match type1.kind {
                CppTypeDeclarationKind::Enum => format!("enum {}", type1.path.to_cpp_pseudo_code()),
                CppTypeDeclarationKind::Class { .. } => {
                    format!("class {}", type1.path.to_cpp_pseudo_code())
                }
            },
            CppItemData::Function(method) => method.short_text(),
            CppItemData::EnumValue(value) => format!(
                "enum value {} = {}",
                value.path.to_cpp_pseudo_code(),
                value.value
            ),
            CppItemData::ClassField(field) => field.short_text(),
            CppItemData::ClassBase(class_base) => {
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
pub enum CppFfiItemKind {
    Function(CppFfiFunction),

    // TODO: separate custom C++ wrapper logic from core implementation,
    // run cpp_parser on wrappers instead of constructing results manually
    QtSlotWrapper(QtSlotWrapper),
}

impl CppFfiItemKind {
    pub fn short_text(&self) -> String {
        match self {
            CppFfiItemKind::Function(function) => match &function.kind {
                CppFfiFunctionKind::Function {
                    cpp_function,
                    omitted_arguments,
                    ..
                } => {
                    if let Some(num) = omitted_arguments {
                        format!("[omitted arguments: {}] {}", num, cpp_function.short_text())
                    } else {
                        cpp_function.short_text()
                    }
                }
                CppFfiFunctionKind::FieldAccessor {
                    accessor_type,
                    field,
                } => format!("[{:?}] {}", accessor_type, field.short_text()),
            },
            CppFfiItemKind::QtSlotWrapper(slot_wrapper) => format!(
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
pub struct CppFfiItem {
    pub kind: CppFfiItemKind,
    pub checks: CppChecks,
    pub is_rust_processed: bool,
}

impl CppFfiItem {
    pub fn from_function(function: CppFfiFunction) -> Self {
        CppFfiItem {
            kind: CppFfiItemKind::Function(function),
            checks: CppChecks::default(),
            is_rust_processed: false,
        }
    }

    pub fn from_qt_slot_wrapper(wrapper: QtSlotWrapper) -> Self {
        CppFfiItem {
            kind: CppFfiItemKind::QtSlotWrapper(wrapper),
            checks: CppChecks::default(),
            is_rust_processed: false,
        }
    }

    pub fn path(&self) -> &CppPath {
        match &self.kind {
            CppFfiItemKind::Function(f) => &f.path,
            CppFfiItemKind::QtSlotWrapper(s) => &s.class_path,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppDatabaseItem {
    pub cpp_data: CppItemData,
    pub source: DatabaseItemSource,
    pub is_cpp_ffi_processed: bool,
    pub is_rust_processed: bool,
}

/// Represents all collected data related to a crate.
#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    crate_name: String,
    crate_version: String,
    cpp_items: Vec<CppDatabaseItem>,
    ffi_items: Vec<CppFfiItem>,
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

    pub fn ffi_items(&self) -> &[CppFfiItem] {
        &self.ffi_items
    }

    pub fn ffi_items_mut(&mut self) -> &mut [CppFfiItem] {
        self.is_modified = true;
        &mut self.ffi_items
    }

    pub fn add_ffi_item(&mut self, item: CppFfiItem) {
        self.is_modified = true;
        self.ffi_items.push(item);
    }

    pub fn add_ffi_items(&mut self, items: Vec<CppFfiItem>) {
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

    pub fn add_cpp_item(&mut self, source: DatabaseItemSource, data: CppItemData) -> bool {
        if let Some(item) = self
            .cpp_items
            .iter_mut()
            .find(|item| item.cpp_data.is_same(&data))
        {
            // parser data takes priority
            if source.is_parser() && !item.source.is_parser() {
                item.source = source;
            }
            return false;
        }
        self.is_modified = true;
        debug!("added cpp item: {}, source: {:?}", data, source);
        trace!("cpp item data: {:?}", data);
        self.cpp_items.push(CppDatabaseItem {
            cpp_data: data,
            source,
            is_cpp_ffi_processed: false,
            is_rust_processed: false,
        });
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
