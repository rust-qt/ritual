use common::target::Target;
use common::log;
use cpp_data::CppTypeData;
use cpp_method::CppMethod;
use cpp_method::CppMethodDoc;
use cpp_data::CppTypeDoc;
use cpp_data::CppOriginLocation;
use cpp_data::CppEnumValue;
use cpp_data::CppClassField;
use cpp_data::CppBaseSpecifier;
use new_impl::html_logger::HtmlLogger;
use std::path::Path;
use common::errors::{ChainErr, Result};
use std::fmt::Display;
use std::fmt::Formatter;
use cpp_data::CppTypeKind;
use common::string_utils::JoinWithSeparator;
use cpp_data::CppVisibility;
use new_impl::html_logger::escape_html;
//use common::errors::Result;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CppField; // TODO: fill

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataSource {
  CppParser,
  CppChecker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataEnv {
  pub target: Target,
  pub data_source: DataSource,
  pub cpp_library_version: Option<String>,
}

impl DataEnv {
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

// TODO: attach this data to DataSource enum instead?
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataEnvInfo {
  pub is_success: bool,
  pub error: Option<String>,
  /// File name of the include file (without full path)
  pub include_file: Option<String>,
  /// Exact location of the declaration
  pub origin_location: Option<CppOriginLocation>,
  /// Set to true before a repeated check is performed in the same
  /// environment
  pub is_invalidated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataEnvWithInfo {
  pub env: DataEnv,
  pub info: DataEnvInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CppItemData {
  Type(CppTypeData),
  Method(CppMethod),
  EnumValue(CppEnumValue),
  ClassField(CppClassField),
  ClassBase(CppBaseSpecifier),
}

impl Display for CppItemData {
  fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
    let s = match *self {
      CppItemData::Type(ref type1) => match type1.kind {
        CppTypeKind::Enum => format!("enum {}", type1.name),
        CppTypeKind::Class {
          ref template_arguments,
        } => format!(
          "class {}{}",
          type1.name,
          if let Some(ref args) = *template_arguments {
            format!(
              "<{}>",
              args.iter().map(|arg| arg.to_cpp_pseudo_code()).join(", ")
            )
          } else {
            String::new()
          }
        ),
      },
      CppItemData::Method(ref method) => method.short_text(),
      CppItemData::EnumValue(ref value) => format!(
        "enum {} {{ {} = {}, ... }}",
        value.enum_name, value.name, value.value
      ),
      CppItemData::ClassField(ref field) => {
        let visibility_text = match field.visibility {
          CppVisibility::Public => "",
          CppVisibility::Protected => "protected ",
          CppVisibility::Private => "private ",
        };
        format!(
          "class {} {{ {}{} {}; ... }}",
          field.class_type.to_cpp_pseudo_code(),
          visibility_text,
          field.field_type.to_cpp_pseudo_code(),
          field.name
        )
      }
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

pub enum DatabaseUpdateResultType {
  ItemAdded,
  EnvAdded,
  EnvUpdated,
  Unchanged,
}

pub struct DatabaseUpdateResult {
  pub result_type: DatabaseUpdateResultType,
  pub old_data: Vec<DataEnvWithInfo>,
  pub new_data: Vec<DataEnvWithInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CppItemDoc {
  Type(CppTypeDoc),
  Method(CppMethodDoc),
  EnumValue(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseItem {
  pub environments: Vec<DataEnvWithInfo>,
  pub cpp_data: CppItemData,
  /// C++ documentation data for this type
  pub doc: Option<CppItemDoc>,
  // TODO: add cpp_ffi and rust data
}

/// Represents all collected data related to a crate.
#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
  pub crate_name: String,
  pub items: Vec<DatabaseItem>,
  pub environments: Vec<DataEnv>,
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

  pub fn crate_name(&self) -> &str {
    &self.crate_name
  }

  pub fn invalidate_env(&mut self, env: &DataEnv) {
    for item in &mut self.items {
      for env1 in &mut item.environments {
        if &env1.env == env {
          env1.info.is_invalidated = true;
        }
      }
    }
  }

  pub fn add_cpp_data(
    &mut self,
    env: DataEnv,
    data: CppItemData,
    info: DataEnvInfo,
  ) -> DatabaseUpdateResult {
    if let Some(item) = self.items.iter_mut().find(|item| item.cpp_data == data) {
      let mut result = DatabaseUpdateResult {
        result_type: DatabaseUpdateResultType::Unchanged,
        old_data: item.environments.clone(),
        new_data: Vec::new(),
      };
      let mut ok = false;
      if let Some(env1) = item.environments.iter_mut().find(|env2| env2.env == env) {
        env1.info.is_invalidated = false; // suppress false change detection
        if env1.info != info {
          //          log::llog(log::LoggerCategory::DebugGeneral, || {
          //            format!(
          //              "cpp env result changed for existing data!\n\
          //               env: {:?}\ndata: {:?}\nnew info: {:?}\nold info: {:?}\n",
          //              env, data, info, env1.info
          //            )
          //          });
          env1.info = info.clone();
          result.result_type = DatabaseUpdateResultType::EnvUpdated;
        } else {
          // result unchanged
        }
        ok = true;
      }
      if !ok {
        //        log::llog(log::LoggerCategory::DebugGeneral, || {
        //          format!(
        //            "cpp new env for existing data!\n\
        //           env: {:?}\ndata: {:?}\ninfo: {:?}\n",
        //            env, data, info
        //          )
        //        });
        item.environments.push(DataEnvWithInfo { env, info });
        result.result_type = DatabaseUpdateResultType::EnvAdded;
      }
      result.new_data = item.environments.clone();
      return result;
    }
    log::llog(log::LoggerCategory::DebugGeneral, || {
      format!(
        "cpp new data!\n\
         env: {:?}\ndata: {:?}\ninfo: {:?}\n",
        env, data, info
      )
    });
    let item = DatabaseItem {
      environments: vec![DataEnvWithInfo { env, info }],
      cpp_data: data,
      doc: None,
    };
    let result = DatabaseUpdateResult {
      result_type: DatabaseUpdateResultType::ItemAdded,
      old_data: Vec::new(),
      new_data: item.environments.clone(),
    };
    self.items.push(item);
    result
  }

  pub fn print_as_html(&self, path: &Path) -> Result<()> {
    let mut logger = HtmlLogger::new(
      path,
      &format!("Database for crate \"{}\"", &self.crate_name),
    )?;
    logger.add_header(&["Item", "Environments"]);
    for item in &self.items {
      let item_text = item.cpp_data.to_string();
      let mut env_text = String::new();
      if !item.environments.is_empty() {
        env_text += &format!(
          "C++ parser: {}",
          item
            .environments
            .iter()
            .map(|env| env.env.short_text())
            .join(", ")
        );
      }

      logger.add(&[escape_html(&item_text), env_text], "");
    }
    Ok(())
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
