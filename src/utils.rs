use std::path::PathBuf;
use std::collections::HashMap;
use std::hash::Hash;
use std;
use log;
use std::process::Command;
use std::env;
use errors::{ErrorKind, Result, ChainErr};

#[cfg(all(windows, target_env = "msvc"))]
pub fn is_msvc() -> bool {
  true
}

#[cfg(not(all(windows, target_env = "msvc")))]
pub fn is_msvc() -> bool {
  false
}

pub fn add_to_multihash<K: Eq + Hash + Clone, T, V: Default + Extend<T>>(hash: &mut HashMap<K, V>,
                                                                         key: K,
                                                                         value: T) {
  use std::collections::hash_map::Entry;
  match hash.entry(key) {
    Entry::Occupied(mut entry) => entry.get_mut().extend(std::iter::once(value)),
    Entry::Vacant(entry) => {
      let mut r = V::default();
      r.extend(std::iter::once(value));
      entry.insert(r);
    }
  }
}

/// Runs a command, checks that it is successful, and
/// returns its output if requested
pub fn run_command(command: &mut Command, fetch_stdout: bool) -> Result<String> {
  log::info(format!("Executing command: {:?}", command));
  let result = if fetch_stdout {
    let output = try!(command.output()
      .chain_err(|| format!("command execution failed: {:?}", command)));
    String::from_utf8(output.stdout).unwrap()
  } else {
    String::new()
  };
  let status = try!(command.status()
    .chain_err(|| format!("command execution failed: {:?}", command)));
  if !status.success() {
    return Err(format!("command failed with status {:?}: {:?}", status, command).into());
  }
  Ok(result)
}

#[cfg_attr(feature="clippy", allow(or_fun_call))]
pub fn add_env_path_item(env_var_name: &'static str,
                         mut new_paths: Vec<PathBuf>)
                         -> Result<std::ffi::OsString> {
  for path in env::split_paths(&env::var(env_var_name).unwrap_or(String::new())) {
    if new_paths.iter().find(|&x| x == &path).is_none() {
      new_paths.push(path);
    }
  }
  env::join_paths(new_paths).chain_err(|| "env::join_paths failed")
}

pub fn manifest_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
