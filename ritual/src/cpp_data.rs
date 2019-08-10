//! Types for handling information about C++ library APIs.

use crate::cpp_function::CppFunction;
pub use crate::cpp_operator::CppOperator;
use crate::cpp_type::CppType;
use itertools::Itertools;
use ritual_common::errors::{bail, ensure, Error, Result};
use ritual_common::utils::MapIfOk;
use serde_derive::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// One item of a C++ enum declaration
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CppEnumValue {
    /// Full path containing enum path and variant name.
    pub path: CppPath,
    /// Corresponding value
    pub value: i64,
}

impl CppEnumValue {
    pub fn is_same(&self, other: &CppEnumValue) -> bool {
        self.path == other.path && self.value == other.value
    }

    pub fn unscoped_path(&self) -> CppPath {
        let mut name = self.path.clone();
        if name.items.len() < 2 {
            panic!("enum path is too short: {:?}", name);
        }
        name.items.remove(name.items.len() - 2);
        name
    }
}

#[test]
fn unscoped_path_should_work() {
    fn check(path: &str, result: &str) {
        let v = CppEnumValue {
            path: CppPath::from_good_str(path),
            value: 0,
        };
        assert_eq!(v.unscoped_path(), CppPath::from_good_str(result));
    }

    check("A::B::C::D", "A::B::D");
    check("A::B", "B");
}

/// Member field of a C++ class declaration
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppClassField {
    pub path: CppPath,
    /// Field type
    pub field_type: CppType,
    /// Visibility
    pub visibility: CppVisibility,
    pub is_static: bool,
}

impl CppClassField {
    pub fn is_same(&self, other: &CppClassField) -> bool {
        self.path == other.path
            && self.field_type == other.field_type
            && self.visibility == other.visibility
            && self.is_static == other.is_static
    }

    pub fn short_text(&self) -> String {
        let visibility_text = match self.visibility {
            CppVisibility::Public => "",
            CppVisibility::Protected => "protected ",
            CppVisibility::Private => "private ",
        };
        format!(
            "{}{} {}",
            visibility_text,
            self.field_type.to_cpp_pseudo_code(),
            self.path.to_cpp_pseudo_code(),
        )
    }
}

/// Item of base class list in a class declaration
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppBaseSpecifier {
    /// Base class type (can include template arguments)
    pub base_class_type: CppPath,
    /// Index of this base (for classes that have multiple base classes)
    pub base_index: usize,
    /// True if this base is virtual
    pub is_virtual: bool,
    /// Base visibility (public, protected or private)
    pub visibility: CppVisibility,

    /// Name and template arguments of the class type that
    /// inherits this base class
    pub derived_class_type: CppPath,
}

/// Location of a C++ type's definition in header files.
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppOriginLocation {
    // Full path to the include file
    pub include_file_path: String,
    /// Line of the file
    pub line: u32,
    /// Column of the file
    pub column: u32,
}

/// Visibility of a C++ entity. Defaults to `Public`
/// for entities that can't have visibility (like free functions)
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub enum CppVisibility {
    Public,
    Protected,
    Private,
}

#[derive(PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppPathItem {
    pub name: String,
    pub template_arguments: Option<Vec<CppType>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppPath {
    /// Parts of the path
    items: Vec<CppPathItem>,
}

impl CppPath {
    pub fn from_good_str(path: &str) -> Self {
        CppPath::from_str(path).unwrap()
    }

    pub fn from_item(item: CppPathItem) -> Self {
        CppPath { items: vec![item] }
    }

    pub fn from_items(items: Vec<CppPathItem>) -> Self {
        CppPath { items }
    }

    pub fn into_items(self) -> Vec<CppPathItem> {
        self.items
    }

    pub fn items(&self) -> &[CppPathItem] {
        &self.items
    }

    pub fn to_cpp_code(&self) -> Result<String> {
        Ok(self
            .items
            .iter()
            .map_if_ok(CppPathItem::to_cpp_code)?
            .join("::"))
    }

    pub fn to_cpp_pseudo_code(&self) -> String {
        self.items
            .iter()
            .map(CppPathItem::to_cpp_pseudo_code)
            .join("::")
    }

    pub fn join(&self, item: CppPathItem) -> CppPath {
        let mut result = self.clone();
        result.items.push(item);
        result
    }

    pub fn last(&self) -> &CppPathItem {
        self.items.last().expect("empty CppPath encountered")
    }

    pub fn last_mut(&mut self) -> &mut CppPathItem {
        self.items.last_mut().expect("empty CppPath encountered")
    }

    pub fn has_parent(&self) -> bool {
        self.items.len() > 1
    }

    pub fn parent(&self) -> Result<CppPath> {
        if self.items.len() > 1 {
            Ok(CppPath {
                items: self.items[..self.items.len() - 1].to_vec(),
            })
        } else {
            bail!("failed to get parent path for {:?}", self)
        }
    }

    pub fn ascii_caption(&self) -> String {
        self.items
            .iter()
            .map(|item| {
                let name: String = item
                    .name
                    .chars()
                    .map(|c| {
                        if c == '~' {
                            'd'
                        } else if !c.is_digit(36) && c != '_' {
                            '_'
                        } else {
                            c
                        }
                    })
                    .collect();
                if let Some(args) = &item.template_arguments {
                    format!(
                        "{}_{}",
                        name,
                        args.iter().map(CppType::ascii_caption).join("_")
                    )
                } else {
                    name
                }
            })
            .join("_")
    }

    /// Returns the identifier this method would be presented with
    /// in Qt documentation.
    pub fn doc_id(&self) -> String {
        self.to_templateless_string()
    }

    pub fn to_templateless_string(&self) -> String {
        self.items().iter().map(|item| &item.name).join("::")
    }

    /// Attempts to replace template types at `nested_level1`
    /// within this type with `template_arguments1`.
    pub fn instantiate(
        &self,
        nested_level1: usize,
        template_arguments1: &[CppType],
    ) -> Result<CppPath> {
        let mut new_path = self.clone();
        for path_item in &mut new_path.items {
            if let Some(template_arguments) = &mut path_item.template_arguments {
                for arg in template_arguments {
                    *arg = arg.instantiate(nested_level1, template_arguments1)?;
                }
            }
        }
        Ok(new_path)
    }

    pub fn deinstantiate(&self) -> CppPath {
        let mut path = self.clone();
        let mut nested_level = 0;
        for item in &mut path.items {
            if let Some(args) = &mut item.template_arguments {
                *args = (0..args.len())
                    .map(|index| CppType::TemplateParameter {
                        nested_level,
                        index,
                        name: format!("T{}_{}", nested_level, index),
                    })
                    .collect();
                nested_level += 1;
            }
        }
        path
    }
}

impl FromStr for CppPath {
    type Err = Error;

    fn from_str(path: &str) -> Result<Self> {
        if path.contains('<') || path.contains('>') {
            bail!("attempted to add template arguments to CppPath");
        }
        if path.is_empty() {
            bail!("attempted to construct an empty CppPath");
        }
        let items = path
            .split("::")
            .map(|item| CppPathItem {
                name: item.into(),
                template_arguments: None,
            })
            .collect();
        Ok(CppPath { items })
    }
}

impl CppPathItem {
    pub fn to_cpp_code(&self) -> Result<String> {
        let args = match &self.template_arguments {
            None => "".to_string(),
            Some(args) => format!(
                "< {} >",
                args.map_if_ok(|arg| arg.to_cpp_code(None))?.join(", ")
            ),
        };
        Ok(format!("{}{}", self.name, args))
    }

    pub fn to_cpp_pseudo_code(&self) -> String {
        let args = match &self.template_arguments {
            None => "".to_string(),
            Some(args) => format!(
                "<{}>",
                args.iter().map(CppType::to_cpp_pseudo_code).join(", ")
            ),
        };
        format!("{}{}", self.name, args)
    }

    pub fn from_good_str(name: &str) -> Self {
        Self::from_str(name).unwrap()
    }
}

impl FromStr for CppPathItem {
    type Err = Error;

    fn from_str(name: &str) -> Result<CppPathItem> {
        ensure!(
            !name.contains('<'),
            "attempted to construct CppPathItem containing template arguments"
        );
        ensure!(
            !name.contains('>'),
            "attempted to construct CppPathItem containing template arguments"
        );
        ensure!(!name.is_empty(), "attempted to construct empty CppPathItem");
        Ok(CppPathItem {
            name: name.into(),
            template_arguments: None,
        })
    }
}

impl fmt::Debug for CppPathItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::result::Result<(), fmt::Error> {
        write!(f, "{:?}", self.name)?;
        if let Some(args) = &self.template_arguments {
            write!(
                f,
                "<{}>",
                args.iter().map(|arg| format!("{:?}", arg)).join(", ")
            )?;
        }
        Ok(())
    }
}

/// Information about a C++ type declaration
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum CppTypeDeclarationKind {
    Enum,
    Class,
}

/// Information about a C++ type declaration
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CppTypeDeclaration {
    /// Identifier, including namespaces and nested classes
    pub path: CppPath,
    pub kind: CppTypeDeclarationKind,
}

impl CppTypeDeclaration {
    pub fn is_same(&self, other: &CppTypeDeclaration) -> bool {
        self.path == other.path
    }
}

impl CppTypeDeclarationKind {
    /// Checks if the type is a class type.
    pub fn is_class(&self) -> bool {
        match self {
            CppTypeDeclarationKind::Class { .. } => true,
            _ => false,
        }
    }

    pub fn is_enum(&self) -> bool {
        match self {
            CppTypeDeclarationKind::Enum => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CppNamespace {
    pub path: CppPath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum CppItem {
    Namespace(CppNamespace),
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
            CppItem::Namespace(data) => &data.path,
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

    pub fn as_namespace_ref(&self) -> Option<&CppNamespace> {
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
            CppItem::Namespace(namespace) => {
                format!("namespace {}", namespace.path.to_cpp_pseudo_code())
            }
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
