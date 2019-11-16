//! Various utilities for working with files

use crate::errors::{bail, err_msg, format_err, Result, ResultExt};
use log::trace;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, BufRead, BufReader, BufWriter, Lines, Read, Write};
use std::path::{Path, PathBuf};
use toml;

/// Move file or directory `src` to `dst` recursively,
/// overwriting previous contents of `dst`. If corresponding
/// old file has the same content as the new file, timestamps
/// of the old file are preserved.
pub fn move_files(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    let inner = || -> Result<()> {
        if src.as_path().is_dir() {
            if !dst.as_path().is_dir() {
                trace!("[DebugMoveFiles] New dir created: {}", dst.display());
                create_dir(dst)?;
            }

            for item in read_dir(dst)? {
                let item = item?;
                if !src.join(item.file_name()).as_path().exists() {
                    let path = item.path();
                    if path.as_path().is_dir() {
                        trace!("[DebugMoveFiles] Old dir removed: {}", path.display());
                        remove_dir_all(&path)?;
                    } else {
                        trace!("[DebugMoveFiles] Old file removed: {}", path.display());
                        remove_file(&path)?;
                    }
                }
            }

            for item in read_dir(src)? {
                let item = item?;
                let from = item.path().to_path_buf();
                let to = dst.join(item.file_name());
                move_files(&from, &to)?;
            }
            remove_dir_all(src)?;
        } else {
            move_one_file(src, dst)?;
        }
        Ok(())
    };
    inner().with_context(|_| format!("failed: move_files({:?}, {:?})", src, dst))?;
    Ok(())
}

/// Copy file or directory `src` to `dst` recursively
pub fn copy_recursively(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    let inner = || -> Result<()> {
        if src.as_path().is_dir() {
            if !dst.is_dir() {
                create_dir(&dst)?;
            }
            for item in read_dir(src)? {
                let item = item?;
                let from = item.path().to_path_buf();
                let to = dst.join(item.file_name());
                copy_recursively(&from, &to)?;
            }
        } else {
            copy_file(src, dst)?;
        }
        Ok(())
    };
    inner().with_context(|_| format!("failed: copy_recursively({:?}, {:?})", src, dst))?;
    Ok(())
}

/// Move file `old_path` to `new_path`. If contents of files are the same,
/// timestamps of the old file are preserved.
fn move_one_file(old_path: &PathBuf, new_path: &PathBuf) -> Result<()> {
    let inner = || -> Result<()> {
        let is_changed = if new_path.as_path().is_file() {
            let string1 = file_to_string(old_path)?;
            let string2 = file_to_string(new_path)?;
            string1 != string2
        } else {
            true
        };

        if is_changed {
            if new_path.as_path().exists() {
                remove_file(&new_path)?;
            }
            rename_file(&old_path, &new_path)?;
            trace!("[DebugMoveFiles] File changed: {}", new_path.display());
        } else {
            remove_file(&old_path)?;
            trace!("[DebugMoveFiles] File not changed: {}", new_path.display());
        }
        Ok(())
    };
    inner().with_context(|_| format!("failed: move_one_file({:?}, {:?})", old_path, new_path))?;
    Ok(())
}

/// A wrapper over a buffered `std::fs::File` containing this file's  path.
pub struct File<F> {
    file: F,
    path: PathBuf,
}

/// A wrapper over `std::fs::File::open` with better error reporting.
pub fn open_file<P: AsRef<Path>>(path: P) -> Result<File<BufReader<fs::File>>> {
    let file = fs::File::open(path.as_ref())
        .with_context(|_| format!("Failed to open file for reading: {:?}", path.as_ref()))?;
    Ok(File {
        file: BufReader::new(file),
        path: path.as_ref().to_path_buf(),
    })
}

/// Returns content of the file `path` as a string.
pub fn file_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut f = open_file(path)?;
    f.read_all()
}

/// A wrapper over `std::fs::File::create` with better error reporting.
pub fn create_file<P: AsRef<Path>>(path: P) -> Result<File<BufWriter<fs::File>>> {
    let file = fs::File::create(path.as_ref())
        .with_context(|_| format!("Failed to create file: {:?}", path.as_ref()))?;
    Ok(File {
        file: BufWriter::new(file),
        path: path.as_ref().to_path_buf(),
    })
}

pub fn create_file_for_append<P: AsRef<Path>>(path: P) -> Result<File<BufWriter<fs::File>>> {
    let file = fs::OpenOptions::new()
        .append(true)
        .open(path.as_ref())
        .with_context(|_| format!("Failed to open file: {:?}", path.as_ref()))?;
    Ok(File {
        file: BufWriter::new(file),
        path: path.as_ref().to_path_buf(),
    })
}

impl<F> File<F> {
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns underlying `std::fs::File`
    pub fn into_inner(self) -> F {
        self.file
    }
}

impl<F: Read> File<F> {
    /// Read content of the file to a string
    pub fn read_all(&mut self) -> Result<String> {
        let mut r = String::new();
        self.file
            .read_to_string(&mut r)
            .with_context(|_| format!("Failed to read from file: {:?}", self.path))?;
        Ok(r)
    }
}

impl<F: BufRead> File<F> {
    pub fn lines(self) -> Lines<F>
    where
        F: Sized,
    {
        self.file.lines()
    }
}

impl<F: Write> Write for File<F> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf).map_err(|err| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to write to file: {:?}: {}", self.path, err),
            )
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush().map_err(|err| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to flush file: {:?}: {}", self.path, err),
            )
        })
    }
}

/// Deserialize value from JSON file `path`.
pub fn load_json<P: AsRef<Path>, T: serde::de::DeserializeOwned>(path: P) -> Result<T> {
    let file = open_file(path.as_ref())?;
    Ok(::serde_json::from_reader(file.into_inner())
        .with_context(|_| format!("failed to parse file as JSON: {}", path.as_ref().display()))?)
}

/// Serialize `value` into JSON file `path`.
pub fn save_json<P: AsRef<Path>, T: ::serde::Serialize>(
    path: P,
    value: &T,
    backup_path: Option<&Path>,
) -> Result<()> {
    let tmp_path = {
        let mut buf = path.as_ref().to_path_buf();
        let tmp_file_name = format!("{}.new", os_str_to_str(&buf.file_name().unwrap())?);
        buf.set_file_name(tmp_file_name);
        buf
    };
    {
        let file = create_file(&tmp_path)?;
        ::serde_json::to_writer(&mut file.into_inner(), value).with_context(|_| {
            format!(
                "failed to serialize to JSON file: {}",
                path.as_ref().display()
            )
        })?;
    }
    if path.as_ref().exists() {
        if let Some(backup_path) = backup_path {
            rename_file(path.as_ref(), backup_path)?;
        } else {
            remove_file(path.as_ref())?;
        }
    }
    rename_file(&tmp_path, path.as_ref())?;
    Ok(())
}

/// Deserialize value from binary file `path`.
pub fn load_bincode<P: AsRef<Path>, T: serde::de::DeserializeOwned>(path: P) -> Result<T> {
    let mut file = open_file(path.as_ref())?.into_inner();
    Ok(bincode::deserialize_from(&mut file)
        .with_context(|_| format!("load_bincode failed: {}", path.as_ref().display()))?)
}

/// Serialize `value` into binary file `path`.
pub fn save_bincode<P: AsRef<Path>, T: ::serde::Serialize>(path: P, value: &T) -> Result<()> {
    let mut file = create_file(path.as_ref())?.into_inner();
    bincode::serialize_into(&mut file, value)
        .with_context(|_| format!("save_bincode failed: {}", path.as_ref().display()))?;
    Ok(())
}

/// Load data from a TOML file
pub fn load_toml_table<P: AsRef<Path>>(path: P) -> Result<toml::value::Table> {
    let data = file_to_string(path.as_ref())?;
    let value = data
        .parse::<toml::Value>()
        .with_context(|_| format!("failed to parse TOML file: {}", path.as_ref().display()))?;
    if let toml::value::Value::Table(table) = value {
        Ok(table)
    } else {
        bail!("TOML is not a table");
    }
}

pub fn crate_version(path: impl AsRef<Path>) -> Result<String> {
    let cargo_toml_path = path.as_ref().join("Cargo.toml");
    let table = load_toml_table(cargo_toml_path)?;
    let package = table
        .get("package")
        .ok_or_else(|| err_msg("Cargo.toml doesn't contain package field"))?;
    let package = package
        .as_table()
        .ok_or_else(|| err_msg("invalid Cargo.toml: package is not a table"))?;
    let version = package
        .get("version")
        .ok_or_else(|| err_msg("Cargo.toml doesn't contain package.version field"))?;
    let version = version
        .as_str()
        .ok_or_else(|| err_msg("invalid Cargo.toml: package.version is not a string"))?;
    Ok(version.into())
}

/// Save `data` to a TOML file
pub fn save_toml_table<P: AsRef<Path>>(path: P, data: &toml::Value) -> Result<()> {
    let mut file = create_file(path.as_ref())?;
    write!(file, "{}", data)
        .with_context(|_| format!("failed to write to TOML file: {}", path.as_ref().display()))?;
    Ok(())
}

/// A wrapper over `std::fs::create_dir` with better error reporting
pub fn create_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    fs::create_dir(path.as_ref())
        .with_context(|_| format!("Failed to create dir: {:?}", path.as_ref()))?;
    Ok(())
}

/// A wrapper over `std::fs::create_dir_all` with better error reporting
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    fs::create_dir_all(path.as_ref()).with_context(|_| {
        format!(
            "Failed to create dirs (with parent components): {:?}",
            path.as_ref()
        )
    })?;
    Ok(())
}

/// A wrapper over `std::fs::remove_dir` with better error reporting
pub fn remove_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    fs::remove_dir(path.as_ref())
        .with_context(|_| format!("Failed to remove dir: {:?}", path.as_ref()))?;
    Ok(())
}

/// A wrapper over `std::fs::remove_dir_all` with better error reporting
pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    fs::remove_dir_all(path.as_ref())
        .with_context(|_| format!("Failed to remove dir (recursively): {:?}", path.as_ref()))?;
    Ok(())
}

/// A wrapper over `std::fs::remove_file` with better error reporting
pub fn remove_file<P: AsRef<Path>>(path: P) -> Result<()> {
    fs::remove_file(path.as_ref())
        .with_context(|_| format!("Failed to remove file: {:?}", path.as_ref()))?;
    Ok(())
}

/// A wrapper over `std::fs::rename` with better error reporting
pub fn rename_file<P: AsRef<Path>, P2: AsRef<Path>>(path1: P, path2: P2) -> Result<()> {
    fs::rename(path1.as_ref(), path2.as_ref()).with_context(|_| {
        format!(
            "Failed to rename file from {:?} to {:?}",
            path1.as_ref(),
            path2.as_ref()
        )
    })?;
    Ok(())
}

/// A wrapper over `std::fs::copy` with better error reporting
pub fn copy_file<P: AsRef<Path>, P2: AsRef<Path>>(path1: P, path2: P2) -> Result<()> {
    fs::copy(path1.as_ref(), path2.as_ref())
        .map(|_| ())
        .with_context(|_| {
            format!(
                "Failed to copy file from {:?} to {:?}",
                path1.as_ref(),
                path2.as_ref()
            )
        })?;
    Ok(())
}

/// A wrapper over `std::fs::DirEntry` iterator with better error reporting
pub struct ReadDir {
    read_dir: fs::ReadDir,
    path: PathBuf,
}

/// A wrapper over `std::fs::read_dir` with better error reporting
pub fn read_dir<P: AsRef<Path>>(path: P) -> Result<ReadDir> {
    Ok(ReadDir {
        read_dir: fs::read_dir(path.as_ref())
            .with_context(|_| format!("Failed to read dir: {:?}", path.as_ref()))?,
        path: path.as_ref().to_path_buf(),
    })
}

impl Iterator for ReadDir {
    type Item = Result<fs::DirEntry>;
    fn next(&mut self) -> Option<Result<fs::DirEntry>> {
        self.read_dir.next().map(|value| {
            Ok(value.with_context(|_| format!("Failed to read dir (in item): {:?}", self.path))?)
        })
    }
}

/// Canonicalize `path`. Similar to `std::fs::canonicalize`, but
/// `\\?\` prefix is removed. Windows implementation of `std::fs::canonicalize`
/// adds this prefix, but many tools don't process it correctly, including
/// CMake and compilers.
pub fn canonicalize<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    Ok(dunce::canonicalize(path.as_ref())
        .with_context(|_| format!("failed to canonicalize {}", path.as_ref().display()))?)
}

/// A wrapper over `Path::to_str` with better error reporting
pub fn path_to_str(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| err_msg(format!("Path is not valid unicode: {}", path.display())))
}

/// A wrapper over `OsStr::to_str` with better error reporting
pub fn os_str_to_str(os_str: &OsStr) -> Result<&str> {
    os_str.to_str().ok_or_else(|| {
        err_msg(format!(
            "String is not valid unicode: {}",
            os_str.to_string_lossy()
        ))
    })
}

/// A wrapper over `OsString::into_string` with better error reporting
pub fn os_string_into_string(s: OsString) -> Result<String> {
    s.into_string().map_err(|s| {
        err_msg(format!(
            "String is not valid unicode: {}",
            s.to_string_lossy()
        ))
    })
}

/// Returns current absolute path of `relative_path`.
/// `relative_path` is relative to the repository root.
pub fn repo_dir_path(relative_path: &str) -> Result<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parent = path
        .parent()
        .ok_or_else(|| err_msg("failed to get parent directory"))?;
    let result = parent.join(relative_path);
    if !result.exists() {
        bail!("detected path does not exist: {}", result.display());
    }
    Ok(result)
}

pub fn diff_paths(path: &Path, base: &Path) -> Result<PathBuf> {
    pathdiff::diff_paths(path, base)
        .ok_or_else(|| format_err!("failed to get relative path to {:?} from {:?}", path, base))
}
