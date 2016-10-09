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
                                                                         key: &K,
                                                                         value: T) {
  if !hash.contains_key(key) {
    hash.insert(key.clone(), Default::default());
  }
  hash.get_mut(key).unwrap().extend(std::iter::once(value));
}


/// Runs a command, checks that it is successful, and
/// returns its output if requested
pub fn run_command(command: &mut Command, fetch_stdout: bool) -> String {
  log::info(format!("Executing command: {:?}", command));
  if fetch_stdout {
    match command.output() {
      Ok(output) => {
        match command.status() {
          Ok(status) => {
            if !status.success() {
              panic!("Command failed: {:?} (status: {})", command, status);
            }
          }
          Err(error) => {
            panic!("Execution failed: {}", error);
          }
        }
        String::from_utf8(output.stdout).unwrap()
      }
      Err(error) => {
        panic!("Execution failed: {}", error);
      }
    }
  } else {
    match command.status() {
      Ok(status) => {
        if !status.success() {
          panic!("Command failed: {:?} (status: {})", command, status);
        }
      }
      Err(error) => {
        panic!("Execution failed: {}", error);
      }
    }
    String::new()
  }
}

#[cfg_attr(feature="clippy", allow(or_fun_call))]
pub fn add_env_path_item(env_var_name: &'static str,
                         mut new_paths: Vec<PathBuf>)
                         -> std::ffi::OsString {
  for path in env::split_paths(&env::var(env_var_name).unwrap_or(String::new())) {
    if new_paths.iter().find(|&x| x == &path).is_none() {
      new_paths.push(path);
    }
  }
  env::join_paths(new_paths).unwrap()
}
