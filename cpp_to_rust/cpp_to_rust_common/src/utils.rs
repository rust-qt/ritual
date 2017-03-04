use errors::{Result, ChainErr};
use log;

use std;
use std::path::PathBuf;
use std::collections::HashMap;
use std::hash::Hash;
use std::process::Command;

// TODO: remove is_msvc and use ::target instead
#[cfg(all(windows, target_env = "msvc"))]
pub fn is_msvc() -> bool {
  true
}

#[cfg(not(all(windows, target_env = "msvc")))]
pub fn is_msvc() -> bool {
  false
}


#[cfg(windows)]
pub fn exe_suffix() -> &'static str {
  return ".exe";
}

#[cfg(not(windows))]
pub fn exe_suffix() -> &'static str {
  return "";
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



/// Runs a command and checks that it was successful
pub fn run_command(command: &mut Command) -> Result<()> {
  log::info(format!("Executing command: {:?}", command));
  let status = command.status().chain_err(|| format!("failed to run command: {:?}", command))?;
  if status.success() {
    Ok(())
  } else {
    Err(format!("command failed with {}: {:?}", status, command).into())
  }
}

/// Runs a command and returns its stdout if it was successful
pub fn get_command_output(command: &mut Command) -> Result<String> {
  log::info(format!("Executing command: {:?}", command));
  command.stdout(std::process::Stdio::piped());
  command.stderr(std::process::Stdio::piped());
  let output = command.output().chain_err(|| format!("failed to run command: {:?}", command))?;
  if output.status.success() {
    String::from_utf8(output.stdout).chain_err(|| "comand output is not valid unicode")
  } else {
    use std::io::Write;
    log::error("Stdout:");
    std::io::stderr().write_all(&output.stdout).chain_err(|| "output failed")?;
    log::error("Stderr:");
    std::io::stderr().write_all(&output.stderr).chain_err(|| "output failed")?;
    Err(format!("command failed with {}: {:?}", output.status, command).into())
  }
}


#[cfg_attr(feature="clippy", allow(or_fun_call))]
pub fn add_env_path_item(env_var_name: &'static str,
                         mut new_paths: Vec<PathBuf>)
                         -> Result<std::ffi::OsString> {
  use std::env;
  for path in env::split_paths(&env::var(env_var_name).unwrap_or(String::new())) {
    if new_paths.iter().find(|&x| x == &path).is_none() {
      new_paths.push(path);
    }
  }
  env::join_paths(new_paths).chain_err(|| "env::join_paths failed")
}

pub trait MapIfOk<A> {
  fn map_if_ok<B, E, F: Fn(A) -> std::result::Result<B, E>>(self,
                                                            f: F)
                                                            -> std::result::Result<Vec<B>, E>;
}

impl<A, T: IntoIterator<Item = A>> MapIfOk<A> for T {
  fn map_if_ok<B, E, F: Fn(A) -> std::result::Result<B, E>>(self,
                                                            f: F)
                                                            -> std::result::Result<Vec<B>, E> {
    let mut r = Vec::new();
    for item in self {
      r.push(f(item)?);
    }
    Ok(r)
  }
}
