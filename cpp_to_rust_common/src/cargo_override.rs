use errors::{Result, ChainErr};
use std::path::Path;
use toml;
use file_utils::{PathBufWithAdded, path_to_str, save_toml, load_toml};

pub fn set_cargo_override<P1: AsRef<Path>, P: AsRef<Path>>(cargo_toml_location: P1,
                                                           paths: &[P])
                                                           -> Result<()> {
  let mut data = load_toml(cargo_toml_location.as_ref())?;
  data.insert("replace".to_string(),
              toml::Value::Table({
                let mut table = toml::Table::new();
                for path in paths {
                  let item_cargo_toml_path = path.as_ref().with_added("Cargo.toml");
                  let item_data = load_toml(&item_cargo_toml_path)?;
                  let package = item_data.get("package")
                    .chain_err(|| "no 'package' in Cargo.toml")?;
                  let name = package.as_table()
                    .chain_err(|| "'package' must be a table")?
                    .get("name")
                    .chain_err(|| "no 'package.name' in Cargo.toml")?;
                  let version = package.as_table()
                    .chain_err(|| "'package' must be a table")?
                    .get("version")
                    .chain_err(|| "no 'package.version' in Cargo.toml")?;
                  let key = format!("{}:{}",
                                    name.as_str()
                                        .chain_err(|| "'package.name' must be a string")?,
                                    version.as_str()
                                        .chain_err(|| "'package.version' must be a string")?);
                  let mut value = toml::Table::new();
                  value.insert("path".to_string(),
                               toml::Value::String(path_to_str(path.as_ref())?.to_string()));
                  table.insert(key,
                               toml::Value::Table(value));
                }
                table
              }));
  save_toml(cargo_toml_location.as_ref(), data)?;
  Ok(())
}
