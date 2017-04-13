//! Various utilities.

use errors::{Result, ChainErr};
use log;

use std;
use std::collections::HashMap;
use std::hash::Hash;
use std::process::Command;

#[cfg(windows)]
/// Returns proper executable file suffix on current platform.
/// Returns `".exe"` on Windows and `""` on other platforms.
pub fn exe_suffix() -> &'static str {
  return ".exe";
}

#[cfg(not(windows))]
/// Returns proper executable file suffix on current platform.
/// Returns `".exe"` on Windows and `""` on other platforms.
pub fn exe_suffix() -> &'static str {
  return "";
}

/// Creates and empty collection at `hash[key]` if there isn't one already.
/// Adds `value` to `hash[key]` collection.
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
  log::status(format!("Executing command: {:?}", command));
  let status = command.status()
    .chain_err(|| format!("failed to run command: {:?}", command))?;
  if status.success() {
    Ok(())
  } else {
    Err(format!("command failed with {}: {:?}", status, command).into())
  }
}

/// Runs a command and returns its stdout if it was successful
pub fn get_command_output(command: &mut Command) -> Result<String> {
  log::status(format!("Executing command: {:?}", command));
  command.stdout(std::process::Stdio::piped());
  command.stderr(std::process::Stdio::piped());
  let output = command.output()
    .chain_err(|| format!("failed to run command: {:?}", command))?;
  if output.status.success() {
    String::from_utf8(output.stdout).chain_err(|| "comand output is not valid unicode")
  } else {
    use std::io::Write;
    let mut stderr = std::io::stderr();
    writeln!(stderr, "Stdout:")?;
    stderr.write_all(&output.stdout)
      .chain_err(|| "output failed")?;
    writeln!(stderr, "Stderr:")?;
    stderr.write_all(&output.stderr)
      .chain_err(|| "output failed")?;
    Err(format!("command failed with {}: {:?}", output.status, command).into())
  }
}

/// Perform a map operation that can fail
pub trait MapIfOk<A> {
  /// Call closure `f` on each element of the collection and return
  /// `Vec` of values returned by the closure. If closure returns `Err`
  /// at some iteration, return that `Err` instead.
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
