use crate::database::{DatabaseCache, DatabaseClient, IndexedDatabase};
use log::info;
use ritual_common::errors::{bail, Result};
use ritual_common::file_utils::{
    create_dir_all, load_json, os_string_into_string, read_dir, remove_file, save_json,
    save_toml_table,
};
use ritual_common::utils::MapIfOk;
use ritual_common::{toml, ReadOnly};
use serde_derive::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {}

/// Provides access to data stored in the user's project directory.
/// The directory contains a subdirectory for each crate the user wants
/// to process. When running any operations, the data is read from and
/// saved to the workspace files. Global workspace configuration
/// can also be set through the `Workspace` object.
#[derive(Debug)]
pub struct Workspace {
    path: PathBuf,
    config: WorkspaceConfig,
}

fn config_path(path: &Path) -> PathBuf {
    path.join("config.json")
}

fn database_path(workspace_path: &Path, crate_name: &str) -> PathBuf {
    workspace_path
        .join("db")
        .join(format!("{}.json", crate_name))
}

impl Workspace {
    pub fn new(path: PathBuf) -> Result<Self> {
        if !path.is_dir() {
            bail!("No such directory: {}", path.display());
        }
        let config_path = config_path(&path);
        for &dir in &["tmp", "out", "log", "backup", "db"] {
            create_dir_all(path.join(dir))?;
        }
        let w = Workspace {
            path,
            config: if config_path.exists() {
                load_json(config_path)?
            } else {
                WorkspaceConfig::default()
            },
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

    pub fn crate_path(&self, crate_name: &str) -> PathBuf {
        self.path.join("out").join(crate_name)
    }

    pub fn delete_database_if_exists(&mut self, crate_name: &str) -> Result<()> {
        let path = database_path(&self.path, crate_name);
        let mut cache = DatabaseCache::global().lock().unwrap();
        cache.remove_if_exists(&path);
        if path.exists() {
            remove_file(path)?;
        }
        Ok(())
    }

    fn get_database(
        &mut self,
        crate_name: &str,
        allow_load: bool,
        allow_create: bool,
    ) -> Result<IndexedDatabase> {
        let mut cache = DatabaseCache::global().lock().unwrap();
        cache.get(
            self.database_path(crate_name),
            crate_name,
            allow_load,
            allow_create,
        )
    }

    pub fn get_database_client<'a>(
        &mut self,
        crate_name: &str,
        dependency_names: impl Iterator<Item = &'a str>,
        allow_load: bool,
        allow_create: bool,
    ) -> Result<DatabaseClient> {
        let current_database = self.get_database(crate_name, allow_load, allow_create)?;
        let dependencies =
            dependency_names.map_if_ok(|name| self.get_database(name, true, false))?;
        Ok(DatabaseClient::new(
            current_database,
            ReadOnly::new(dependencies),
        ))
    }

    fn database_backup_path(&self, crate_name: &str) -> PathBuf {
        let date = chrono::Local::now();
        self.path.join("backup").join(format!(
            "db_{}_{}.json",
            crate_name,
            date.format("%Y-%m-%d_%H-%M-%S")
        ))
    }

    pub fn save_database(&self, database: &mut DatabaseClient) -> Result<()> {
        if database.is_modified() {
            info!("Saving data");
            let backup_path = self.database_backup_path(database.crate_name());
            save_json(
                database_path(&self.path, database.crate_name()),
                database.data(),
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

        save_toml_table(
            self.path.join("Cargo.toml"),
            &toml::Value::Table(cargo_toml),
        )?;
        Ok(())
    }
}
