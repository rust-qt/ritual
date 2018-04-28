use common::target::Target;
use common::log;
use cpp_data::CppTypeData;
use cpp_method::CppMethod;
use cpp_method::CppMethodDoc;
use cpp_data::CppTypeDoc;
use cpp_data::CppOriginLocation;

//use common::errors::Result;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CppField; // TODO: fill

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataSource {
  CppParser,
  CppChecker,
  DocParser,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataEnv {
  pub target: Target,
  pub data_source: DataSource,
  pub cpp_library_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataEnvInfo {
  pub error: Option<String>,
  /// File name of the include file (without full path)
  pub include_file: Option<String>,
  /// Exact location of the declaration
  pub origin_location: Option<CppOriginLocation>,
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
  ClassField(CppField),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CppItemDoc {
  Type(CppTypeDoc),
  Method(CppMethodDoc),
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
  crate_name: String,
  items: Vec<DatabaseItem>,
}

impl Database {
  pub fn empty(crate_name: &str) -> Database {
    Database {
      crate_name: crate_name.to_owned(),
      items: Vec::new(),
    }
  }

  pub fn items(&self) -> &[DatabaseItem] {
    &self.items
  }

  pub fn crate_name(&self) -> &str {
    &self.crate_name
  }

  pub fn add_cpp_data(&mut self, env: DataEnv, data: CppItemData, info: DataEnvInfo) {
    if let Some(r) = self.items.iter_mut().find(|item| item.cpp_data == data) {
      if let Some(env1) = r.environments.iter_mut().find(|env2| env2.env == env) {
        if env1.info != info {
          log::llog(log::LoggerCategory::DebugGeneral, || {
            format!(
              "cpp env result changed for existing data!\n\
               env: {:?}\ndata: {:?}\nnew info: {:?}\nold info: {:?}\n",
              env, data, info, env1.info
            )
          });
          env1.info = info;
        }
        return;
      }
      log::llog(log::LoggerCategory::DebugGeneral, || {
        format!(
          "cpp new env for existing data!\n\
           env: {:?}\ndata: {:?}\ninfo: {:?}\n",
          env, data, info
        )
      });
      r.environments.push(DataEnvWithInfo { env, info });
      return;
    }
    log::llog(log::LoggerCategory::DebugGeneral, || {
      format!(
        "cpp new data!\n\
         env: {:?}\ndata: {:?}\ninfo: {:?}\n",
        env, data, info
      )
    });
    self.items.push(DatabaseItem {
      environments: vec![DataEnvWithInfo { env, info }],
      cpp_data: data,
      doc: None,
    });
  }
}
