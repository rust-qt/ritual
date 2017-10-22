use new_impl::database::Database;
use common::errors::Result;
use common::file_utils::PathBufWithAdded;
use common::file_utils::{save_json, load_json};
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
#[derive(Serialize, Deserialize)]
pub struct WorkspaceConfig {}

#[derive(Debug)]
pub struct Workspace {
  path: PathBuf,
  config: WorkspaceConfig,
  databases: Vec<Database>,
}

fn config_path(path: &Path) -> PathBuf {
  path.with_added("config.json")
}

impl Workspace {
  pub fn new(path: PathBuf) -> Result<Workspace> {
    if !path.is_dir() {
      return Err(format!("No such directory: {}", path.display()).into());
    }
    let config_path = config_path(&path);
    Ok(Workspace {
      path,
      config: if config_path.exists() {
        load_json(config_path)?
      } else {
        WorkspaceConfig::default()
      },
      databases: Vec::new(),
    })
  }

  pub fn import_published_crate(&mut self, crate_name: &str) -> Result<()> {
    unimplemented!()
  }

  fn take_loaded_crate(&mut self, crate_name: &str) -> Option<Database> {
    self
      .databases
      .iter()
      .position(|d| d.crate_name() == crate_name)
      .and_then(|i| Some(self.databases.swap_remove(i)))
  }

  fn database_path(&self, crate_name: &str) -> PathBuf {
    self.path.with_added(crate_name).with_added("database.json")
  }

  pub fn load_crate(&mut self, crate_name: &str) -> Result<Database> {
    if let Some(r) = self.take_loaded_crate(crate_name) {
      return Ok(r);
    }
    load_json(self.database_path(crate_name))
  }

  pub fn load_or_create_crate(&mut self, crate_name: &str) -> Result<Database> {
    if let Some(r) = self.take_loaded_crate(crate_name) {
      return Ok(r);
    }
    let path = self.database_path(crate_name);
    if path.exists() {
      load_json(path)
    } else {
      Ok(Database::empty(crate_name))
    }
  }

  pub fn save_crate(&mut self, data: Database) -> Result<()> {
    save_json(self.database_path(data.crate_name()), &data)?;
    self.databases.push(data);
    Ok(())
  }

  fn save_config(&self) -> Result<()> {
    save_json(config_path(&self.path), &self.config)
  }
}
