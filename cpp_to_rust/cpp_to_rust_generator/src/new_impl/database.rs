use common::target::Target;
use cpp_data::CppBaseSpecifier;
use cpp_data::CppClassField;
use cpp_data::CppEnumValue;
use cpp_data::CppOriginLocation;
use cpp_data::CppTypeData;
use cpp_data::CppTypeDataKind;

use cpp_data::CppVisibility;
use cpp_ffi_data::CppFfiMethod;
use cpp_method::CppMethod;

use cpp_type::CppType;
use cpp_type::CppTypeBase;
use cpp_type::CppTypeIndirection;
use std::fmt::Display;
use std::fmt::Formatter;
//use common::errors::Result;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CppField; // TODO: fill??

//#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//pub enum DataSource {
//  CppParser,
//  CppChecker,
//}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CppCheckerEnv {
  pub target: Target,
  pub cpp_library_version: Option<String>,
}

impl CppCheckerEnv {
  pub fn short_text(&self) -> String {
    format!(
      "{}/{:?}-{:?}-{:?}-{:?}",
      self
        .cpp_library_version
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("None"),
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
    include_file: Option<String>,
    /// Exact location of the declaration
    origin_location: Option<CppOriginLocation>,
  },
  Destructor,
  TemplateInstantiation,
  SignalArguments,
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
pub struct CppCheckerInfo {
  pub env: CppCheckerEnv,
  pub error: Option<String>,
}

impl CppCheckerInfo {
  pub fn error_to_log(error: &Option<String>) -> String {
    match error {
      None => "<span class='ok'>OK</span>".to_string(),
      Some(error) => format!("<span class='ok'>Error</span> ({})", error),
    }
  }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CppCheckerInfoList {
  pub items: Vec<CppCheckerInfo>,
}

pub enum CppCheckerAddResult {
  Added,
  Changed { old: Option<String> },
  Unchanged,
}

impl CppCheckerInfoList {
  pub fn add(&mut self, env: &CppCheckerEnv, error: Option<String>) -> CppCheckerAddResult {
    if let Some(item) = self.items.iter_mut().find(|i| &i.env == env) {
      let r = if item.error == error {
        CppCheckerAddResult::Unchanged
      } else {
        CppCheckerAddResult::Changed {
          old: item.error.clone(),
        }
      };
      item.error = error;
      return r;
    }
    self.items.push(CppCheckerInfo {
      env: env.clone(),
      error,
    });
    CppCheckerAddResult::Added
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CppItemData {
  Type(CppTypeData),
  EnumValue(CppEnumValue),
  Method(CppMethod),
  ClassField(CppClassField),
  ClassBase(CppBaseSpecifier),
}

impl CppItemData {
  pub fn is_same(&self, other: &CppItemData) -> bool {
    match (self, other) {
      (&CppItemData::Type(ref v), &CppItemData::Type(ref v2)) => v.is_same(v2),
      (&CppItemData::EnumValue(ref v), &CppItemData::EnumValue(ref v2)) => v.is_same(v2),
      (&CppItemData::Method(ref v), &CppItemData::Method(ref v2)) => v.is_same(v2),
      (&CppItemData::ClassField(ref v), &CppItemData::ClassField(ref v2)) => v.is_same(v2),
      (&CppItemData::ClassBase(ref v), &CppItemData::ClassBase(ref v2)) => v.is_same(v2),
      _ => false,
    }
  }

  pub fn all_involved_types(&self) -> Vec<CppType> {
    match *self {
      CppItemData::Type(ref t) => match t.kind {
        CppTypeDataKind::Enum => vec![
          CppType {
            indirection: CppTypeIndirection::None,
            is_const: false,
            is_const2: false,
            base: CppTypeBase::Enum {
              name: t.name.to_string(),
            },
          },
        ],
        CppTypeDataKind::Class { ref type_base } => vec![
          CppType {
            indirection: CppTypeIndirection::None,
            is_const: false,
            is_const2: false,
            base: CppTypeBase::Class(type_base.clone()),
          },
        ],
      },
      CppItemData::EnumValue(_) => Vec::new(),
      CppItemData::Method(ref method) => method.all_involved_types(),
      CppItemData::ClassField(ref field) => {
        let class_type = CppType {
          indirection: CppTypeIndirection::None,
          is_const: false,
          is_const2: false,
          base: CppTypeBase::Class(field.class_type.clone()),
        };
        vec![class_type, field.field_type.clone()]
      }
      CppItemData::ClassBase(ref base) => vec![
        CppType {
          indirection: CppTypeIndirection::None,
          is_const: false,
          is_const2: false,
          base: CppTypeBase::Class(base.base_class_type.clone()),
        },
        CppType {
          indirection: CppTypeIndirection::None,
          is_const: false,
          is_const2: false,
          base: CppTypeBase::Class(base.derived_class_type.clone()),
        },
      ],
    }
  }
}

impl Display for CppItemData {
  fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
    let s = match *self {
      CppItemData::Type(ref type1) => match type1.kind {
        CppTypeDataKind::Enum => format!("enum {}", type1.name),
        CppTypeDataKind::Class { ref type_base } => {
          format!("class {}", type_base.to_cpp_pseudo_code())
        }
      },
      CppItemData::Method(ref method) => method.short_text(),
      CppItemData::EnumValue(ref value) => format!(
        "enum {} {{ {} = {}, ... }}",
        value.enum_name, value.name, value.value
      ),
      CppItemData::ClassField(ref field) => field.short_text(),
      CppItemData::ClassBase(ref class_base) => {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseItem {
  pub cpp_data: CppItemData,
  pub source: DatabaseItemSource,
  pub cpp_ffi_methods: Option<Vec<CppFfiMethod>>,
  // TODO: add rust data
}

/// Represents all collected data related to a crate.
#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
  pub crate_name: String,
  pub items: Vec<DatabaseItem>,
  pub environments: Vec<CppCheckerEnv>,
}

impl Database {
  pub fn empty(crate_name: &str) -> Database {
    Database {
      crate_name: crate_name.to_owned(),
      items: Vec::new(),
      environments: Vec::new(),
    }
  }

  pub fn items(&self) -> &[DatabaseItem] {
    &self.items
  }

  pub fn clear(&mut self) {
    self.items.clear();
    self.environments.clear();
  }

  pub fn crate_name(&self) -> &str {
    &self.crate_name
  }

  pub fn add_cpp_data(&mut self, source: DatabaseItemSource, data: CppItemData) -> bool {
    if let Some(item) = self
      .items
      .iter_mut()
      .find(|item| item.cpp_data.is_same(&data))
    {
      // parser data takes priority
      if source.is_parser() && !item.source.is_parser() {
        item.source = source;
      }
      return false;
    }
    self.items.push(DatabaseItem {
      cpp_data: data,
      source: source,
      cpp_ffi_methods: None,
    });
    true
  }

  /*
  pub fn mark_missing_cpp_data(&mut self, env: DataEnv) {
    let info = DataEnvInfo {
      is_success: false,
      ..DataEnvInfo::default()
    };
    for item in &mut self.items {
      if !item.environments.iter().any(|env2| env2.env == env) {
        item.environments.push(DataEnvWithInfo {
          env: env.clone(),
          info: info.clone(),
        });
      }
    }
  }*/
}
