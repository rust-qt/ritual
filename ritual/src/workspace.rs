use ritual_common::errors::{bail, FancyUnwrap, Result};
use ritual_common::file_utils::{create_dir, load_json, save_json};

use crate::database::Database;
use serde_derive::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub write_dependencies_local_paths: bool,
}

#[derive(Debug)]
struct EditedDatabase {
    database: Database,
    saved: bool,
}

/// Provides access to data stored in the user's project directory.
/// The directory contains a subdirectory for each crate the user wants
/// to process. When running any operations, the data is read from and
/// saved to the workspace files. Global workspace configuration
/// can also be set through the `Workspace` object.
#[derive(Debug)]
pub struct Workspace {
    path: PathBuf,
    config: WorkspaceConfig,
    databases: Vec<EditedDatabase>,
}

fn config_path(path: &Path) -> PathBuf {
    path.join("config.json")
}

fn database_path(workspace_path: &Path, crate_name: &str) -> PathBuf {
    workspace_path.join(crate_name).join("database.json")
}

impl Workspace {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(path: PathBuf) -> Result<Workspace> {
        if !path.is_dir() {
            bail!("No such directory: {}", path.display());
        }
        let config_path = config_path(&path);
        let w = Workspace {
            path,
            config: if config_path.exists() {
                load_json(config_path)?
            } else {
                WorkspaceConfig::default()
            },
            databases: Vec::new(),
        };
        Ok(w)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn tmp_path(&self) -> Result<PathBuf> {
        let path = self.path.join("tmp");
        if !path.exists() {
            create_dir(&path)?;
        }
        Ok(path)
    }

    pub fn config(&self) -> &WorkspaceConfig {
        &self.config
    }

    pub fn log_path(&self) -> Result<PathBuf> {
        let path = self.path.join("log");
        if !path.exists() {
            create_dir(&path)?;
        }
        Ok(path)
    }

    pub fn crate_path(&self, crate_name: &str) -> Result<PathBuf> {
        let path = self.path.join(crate_name);
        if !path.exists() {
            create_dir(&path)?;
        }
        Ok(path)
    }

    #[allow(unused_variables)]
    pub fn import_published_crate(&mut self, crate_name: &str) -> Result<()> {
        unimplemented!()
    }

    fn take_loaded_crate(&mut self, crate_name: &str) -> Option<Database> {
        self.databases
            .iter()
            .position(|d| d.database.crate_name() == crate_name)
            .and_then(|i| Some(self.databases.swap_remove(i).database))
    }
    /*
    pub fn crate_exists(&self, crate_name: &str) -> bool {
      database_path(&self.path, crate_name).exists()
    }

    pub fn create_crate(&mut self, crate_name: &str) -> Result<()> {
      create_dir(self.path.join(crate_name))?;
      save_json(database_path(&self.path, data.crate_name()), &Database::empty(crate_name))?;
      Ok(())
    }*/

    pub fn load_crate(&mut self, crate_name: &str) -> Result<Database> {
        if let Some(r) = self.take_loaded_crate(crate_name) {
            return Ok(r);
        }
        load_json(database_path(&self.path, crate_name))
    }

    pub fn load_or_create_crate(&mut self, crate_name: &str) -> Result<Database> {
        if let Some(r) = self.take_loaded_crate(crate_name) {
            return Ok(r);
        }
        let path = database_path(&self.path, crate_name);
        if path.exists() {
            load_json(path)
        } else {
            // make sure crate dir exists
            let _ = self.crate_path(crate_name)?;
            Ok(Database::empty(crate_name))
        }
    }

    pub fn put_crate(&mut self, database: Database, saved: bool) {
        self.databases.push(EditedDatabase { database, saved });
    }

    pub fn set_write_dependencies_local_paths(&mut self, value: bool) -> Result<()> {
        if self.config.write_dependencies_local_paths == value {
            return Ok(());
        }
        self.config.write_dependencies_local_paths = value;
        self.save_config()
    }

    fn save_config(&self) -> Result<()> {
        save_json(config_path(&self.path), &self.config)
    }

    pub fn save_data(&mut self) -> Result<()> {
        //log::status("test1: save data start!");
        for database in &mut self.databases {
            if !database.saved {
                //log::status("test1: save data - saving crate");
                let data = &database.database;
                save_json(database_path(&self.path, data.crate_name()), &data)?;
                database.saved = true;
            }
        }
        //log::status("test1: save data success!");
        Ok(())
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        //log::status("test1: Workspace drop!");
        self.save_data().fancy_unwrap();
    }
}
