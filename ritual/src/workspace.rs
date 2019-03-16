use crate::database::Database;
use log::info;
use ritual_common::errors::{bail, Result};
use ritual_common::file_utils::create_dir_all;
use ritual_common::file_utils::os_string_into_string;
use ritual_common::file_utils::read_dir;
use ritual_common::file_utils::remove_file;
use ritual_common::file_utils::save_toml;
use ritual_common::file_utils::{create_dir, load_json, save_json};
use ritual_common::toml;
use serde_derive::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub write_dependencies_local_paths: bool,
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
    databases: Vec<Database>,
}

fn config_path(path: &Path) -> PathBuf {
    path.join("config.json")
}

fn database_path(workspace_path: &Path, crate_name: &str) -> PathBuf {
    workspace_path
        .join("out")
        .join(crate_name)
        .join("database.json")
}

impl Workspace {
    pub fn new(path: PathBuf) -> Result<Self> {
        if !path.is_dir() {
            bail!("No such directory: {}", path.display());
        }
        let config_path = config_path(&path);
        for &dir in &["tmp", "out", "log", "backup"] {
            create_dir_all(path.join(dir))?;
        }
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

    pub fn database_path(&self, crate_name: &str) -> PathBuf {
        database_path(&self.path, crate_name)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn tmp_path(&self) -> PathBuf {
        self.path.join("tmp")
    }

    pub fn config(&self) -> &WorkspaceConfig {
        &self.config
    }

    pub fn log_path(&self) -> PathBuf {
        self.path.join("log")
    }

    pub fn crate_path(&self, crate_name: &str) -> Result<PathBuf> {
        let path = self.path.join("out").join(crate_name);
        if !path.exists() {
            create_dir(&path)?;
        }
        Ok(path)
    }

    // TODO: import published crates

    fn take_loaded_crate(&mut self, crate_name: &str) -> Option<Database> {
        self.databases
            .iter()
            .position(|d| d.crate_name() == crate_name)
            .and_then(|i| Some(self.databases.swap_remove(i)))
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

    pub fn delete_database_if_exists(&mut self, crate_name: &str) -> Result<()> {
        self.databases.retain(|d| d.crate_name() != crate_name);
        let path = database_path(&self.path, crate_name);
        if path.exists() {
            remove_file(path)?;
        }
        Ok(())
    }

    pub fn get_database(
        &mut self,
        crate_name: &str,
        allow_load: bool,
        allow_create: bool,
    ) -> Result<Database> {
        if allow_load {
            if let Some(r) = self.take_loaded_crate(crate_name) {
                return Ok(r);
            }
            let path = database_path(&self.path, crate_name);
            if path.exists() {
                return load_json(path);
            }
        }
        if allow_create {
            // make sure crate dir exists
            let _ = self.crate_path(crate_name)?;
            return Ok(Database::empty(crate_name));
        }
        bail!("can't get database");
    }

    pub fn put_crate(&mut self, database: Database) {
        self.databases.push(database);
    }

    pub fn set_write_dependencies_local_paths(&mut self, value: bool) -> Result<()> {
        if self.config.write_dependencies_local_paths == value {
            return Ok(());
        }
        self.config.write_dependencies_local_paths = value;
        self.save_config()
    }

    fn save_config(&self) -> Result<()> {
        save_json(config_path(&self.path), &self.config, None)
    }

    fn database_backup_path(&self, crate_name: &str) -> PathBuf {
        let date = chrono::Local::now();
        self.path.join("backup").join(format!(
            "db_{}_{}.json",
            crate_name,
            date.format("%Y-%m-%d_%H-%M-%S")
        ))
    }

    pub fn save_database(&self, database: &mut Database) -> Result<()> {
        if database.is_modified() {
            info!("Saving data");
            let backup_path = self.database_backup_path(database.crate_name());
            save_json(
                database_path(&self.path, database.crate_name()),
                database,
                Some(&backup_path),
            )?;
            database.set_saved();
        }
        Ok(())
    }

    pub fn update_cargo_toml(&self) -> Result<()> {
        let mut members = Vec::new();
        for item in read_dir(self.path.join("out"))? {
            let item = item?;
            let path = item.path().join("Cargo.toml");
            if path.exists() {
                let dir_name = os_string_into_string(item.file_name())?;
                members.push(toml::Value::String(format!("out/{}", dir_name)));
            }
        }

        let mut table = toml::value::Table::new();
        table.insert("members".to_string(), toml::Value::Array(members));

        let mut cargo_toml = toml::value::Table::new();
        cargo_toml.insert("workspace".to_string(), toml::Value::Table(table));

        save_toml(
            self.path.join("Cargo.toml"),
            &toml::Value::Table(cargo_toml),
        )?;
        Ok(())
    }
}
