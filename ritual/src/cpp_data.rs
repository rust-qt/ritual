//! Types for handling information about C++ library APIs.

pub use crate::cpp_operator::CppOperator;
use crate::cpp_type::CppType;
use itertools::Itertools;
use ritual_common::errors::{bail, Error, Result};
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
    /// C++ documentation for this item in HTML
    pub doc: Option<String>,
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
            path: CppPath::from_str_unchecked(path),
            value: 0,
            doc: None,
        };
        assert_eq!(v.unscoped_path(), CppPath::from_str_unchecked(result));
    }

    check("A::B::C::D", "A::B::D");
    check("A::B", "B");
}

/// Member field of a C++ class declaration
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppClassField {
    /// Identifier
    pub name: String, // TODO: merge with `class_type`
    /// Field type
    pub field_type: CppType,
    /// Visibility
    pub visibility: CppVisibility,
    //  /// Size of type in bytes
    //  pub size: Option<usize>,
    /// Name and template arguments of the class type that owns this field
    pub class_type: CppPath,

    pub is_const: bool,
    pub is_static: bool,
}

impl CppClassField {
    pub fn is_same(&self, other: &CppClassField) -> bool {
        // TODO: when doc is added to CppClassField, ignore it here
        self == other
    }

    pub fn short_text(&self) -> String {
        let visibility_text = match self.visibility {
            CppVisibility::Public => "",
            CppVisibility::Protected => "protected ",
            CppVisibility::Private => "private ",
        };
        format!(
            "class {} {{ {}{} {}; }}",
            self.class_type.to_cpp_pseudo_code(),
            visibility_text,
            self.field_type.to_cpp_pseudo_code(),
            self.name
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

/// C++ documentation for a type
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CppTypeDoc {
    pub path: CppPath,
    /// HTML content
    pub html: String,
    /// Absolute URL to online documentation page for this type
    pub url: String,
    /// Absolute documentation URLs encountered in the content
    pub cross_references: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppPathItem {
    pub name: String,
    pub template_arguments: Option<Vec<CppType>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppPath {
    /// Parts of the path
    pub items: Vec<CppPathItem>,
}

impl CppPath {
    pub fn from_str_unchecked(path: &str) -> CppPath {
        CppPath::from_str(path).unwrap()
    }

    pub fn from_item(item: CppPathItem) -> CppPath {
        CppPath { items: vec![item] }
    }

    pub fn from_items(items: Vec<CppPathItem>) -> CppPath {
        CppPath { items }
    }

    pub fn to_cpp_code(&self) -> Result<String> {
        Ok(self
            .items
            .iter()
            .map_if_ok(|item| item.to_cpp_code())?
            .join("::"))
    }

    pub fn to_cpp_pseudo_code(&self) -> String {
        self.items
            .iter()
            .map(|item| item.to_cpp_pseudo_code())
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

    pub fn parent(&self) -> Option<CppPath> {
        if self.items.len() > 1 {
            Some(CppPath {
                items: self.items[..self.items.len() - 1].to_vec(),
            })
        } else {
            None
        }
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
        let args = match self.template_arguments {
            None => "".to_string(),
            Some(ref args) => format!(
                "< {} >",
                args.map_if_ok(|arg| arg.to_cpp_code(None))?.join(", ")
            ),
        };
        Ok(format!("{}{}", self.name, args))
    }

    pub fn to_cpp_pseudo_code(&self) -> String {
        let args = match self.template_arguments {
            None => "".to_string(),
            Some(ref args) => format!(
                "<{}>",
                args.iter().map(|arg| arg.to_cpp_pseudo_code()).join(", ")
            ),
        };
        format!("{}{}", self.name, args)
    }

    pub fn from_str_unchecked(name: &str) -> CppPathItem {
        // TODO: Result?
        assert!(
            !name.contains('<'),
            "attempted to construct CppPathItem containing template arguments"
        );
        assert!(
            !name.contains('>'),
            "attempted to construct CppPathItem containing template arguments"
        );
        assert!(!name.is_empty(), "attempted to construct empty CppPathItem");
        CppPathItem {
            name: name.into(),
            template_arguments: None,
        }
    }
}

impl fmt::Display for CppPathItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::result::Result<(), fmt::Error> {
        write!(f, "{}", self.name)?;
        if let Some(ref args) = self.template_arguments {
            write!(
                f,
                "<{}>",
                args.iter().map(|arg| arg.to_cpp_pseudo_code()).join(", ")
            )?;
        }
        Ok(())
    }
}

impl fmt::Display for CppPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::result::Result<(), fmt::Error> {
        write!(f, "{}", self.to_cpp_pseudo_code())?;
        Ok(())
    }
}

/// Information about a C++ type declaration
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum CppTypeDataKind {
    Enum,
    Class,
}

/// Information about a C++ type declaration
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CppTypeData {
    /// Identifier, including namespaces and nested classes
    pub path: CppPath,
    pub kind: CppTypeDataKind,
    /// C++ documentation for the type
    pub doc: Option<CppTypeDoc>,
    pub is_movable: bool,
}

impl CppTypeData {
    pub fn is_same(&self, other: &CppTypeData) -> bool {
        self.path == other.path && self.kind == other.kind
    }
}

impl CppTypeDataKind {
    /// Checks if the type is a class type.
    pub fn is_class(&self) -> bool {
        match self {
            CppTypeDataKind::Class { .. } => true,
            _ => false,
        }
    }

    pub fn is_enum(&self) -> bool {
        match self {
            CppTypeDataKind::Enum { .. } => true,
            _ => false,
        }
    }
}
