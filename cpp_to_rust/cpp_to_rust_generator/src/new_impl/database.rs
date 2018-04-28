use new_impl::cpp_type::CppType;
use new_impl::cpp_method::CppMethod;
use common::target::Target;
use common::log;

//use common::errors::Result;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataSource {
  Parser,
  Checker,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataEnv {
  target: Target,
  data_source: DataSource,
  cpp_library_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataEnvResult {
  env: DataEnv,
  error: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CppItemData {
  Type(CppType),
  Method(CppMethod),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseItem {
  environments: Vec<DataEnvResult>,
  cpp_data: CppItemData,
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

  pub fn crate_name(&self) -> &str {
    &self.crate_name
  }

  pub fn add_cpp_data(&mut self, env: DataEnv, data: CppItemData, error: Option<String>) {
    if let Some(r) = self.items.iter_mut().find(|item| item.cpp_data == data) {
      if let Some(env1) = r.environments.iter_mut().find(|env2| env2.env == env) {
        if env1.error != error {
          log::llog(log::LoggerCategory::DebugGeneral, || {
            format!(
              "cpp env result changed for existing data!\n\
               env: {:?}\ndata: {:?}\nnew error: {:?}\nold error: {:?}\n",
              env, data, error, env1.error
            )
          });
          env1.error = error;
        }
        return;
      }
      log::llog(log::LoggerCategory::DebugGeneral, || {
        format!(
          "cpp new env for existing data!\n\
           env: {:?}\ndata: {:?}\nerror: {:?}\n",
          env, data, error
        )
      });
      r.environments.push(DataEnvResult { env, error });
      return;
    }
    log::llog(log::LoggerCategory::DebugGeneral, || {
      format!(
        "cpp new data!\n\
         env: {:?}\ndata: {:?}\nerror: {:?}\n",
        env, data, error
      )
    });
    self.items.push(DatabaseItem {
      environments: vec![DataEnvResult { env, error }],
      cpp_data: data,
    });
  }
}
