use errors::{Result, ChainErr};
use log;

use std;
use std::path::PathBuf;
use std::collections::HashMap;
use std::hash::Hash;
use std::process::Command;

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
pub fn run_command(command: &mut Command, fetch_stdout: bool, pipe_output: bool) -> Result<String> {
  if pipe_output {
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
  }
  log::info(format!("Executing command: {:?}", command));

  // command.output() must be called before command.status()
  // to avoid freezes on Windows
  let output = if pipe_output || fetch_stdout {
    Some(try!(command.output().chain_err(|| format!("command execution failed: {:?}", command))))
  } else {
    None
  };

  let status = try!(command.status()
    .chain_err(|| format!("command execution failed: {:?}", command)));
  if status.success() {
    Ok(if let Some(output) = output {
      if fetch_stdout {
        try!(String::from_utf8(output.stdout).chain_err(|| "comand output is not valid unicode"))
      } else {
        String::new()
      }
    } else {
      String::new()
    })
  } else {
    if let Some(output) = output {
      use std::io::Write;
      log::error("Stdout:");
      try!(std::io::stderr().write_all(&output.stdout).chain_err(|| "output failed"));
      log::error("Stderr:");
      try!(std::io::stderr().write_all(&output.stderr).chain_err(|| "output failed"));
    }
    Err(format!("command failed with status {:?}: {:?}", status, command).into())
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

pub fn manifest_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub trait MapIfOk<A> {
  fn map_if_ok<B, E, F: Fn(A) -> std::result::Result<B, E>>(self,
                                                            f: F)
                                                            -> std::result::Result<Vec<B>, E>;
}

impl<A, T: Iterator<Item = A>> MapIfOk<A> for T {
  fn map_if_ok<B, E, F: Fn(A) -> std::result::Result<B, E>>(self,
                                                            f: F)
                                                            -> std::result::Result<Vec<B>, E> {
    let mut r = Vec::new();
    r.reserve(self.size_hint().0);
    for item in self {
      r.push(try!(f(item)));
    }
    Ok(r)
  }
}
