//! Various utilities for working with files

use crate::errors::{ChainErr, Result};
use crate::log;

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use toml;

/// Move file or directory `src` to `dst` recursively,
/// overwriting previous contents of `dst`. If corresponding
/// old file has the same content as the new file, timestamps
/// of the old file are preserved.
pub fn move_files(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    let err = || format!("failed: move_files({:?}, {:?})", src, dst);
    if src.as_path().is_dir() {
        if !dst.as_path().is_dir() {
            log::llog(log::DebugMoveFiles, || {
                format!("New dir created: {}", dst.display())
            });
            create_dir(dst).chain_err(&err)?;
        }

        for item in read_dir(dst).chain_err(&err)? {
            let item = item.chain_err(&err)?;
            if !src.with_added(item.file_name()).as_path().exists() {
                let path = item.path();
                if path.as_path().is_dir() {
                    log::llog(log::DebugMoveFiles, || {
                        format!("Old dir removed: {}", path.display())
                    });
                    remove_dir_all(&path).chain_err(&err)?;
                } else {
                    log::llog(log::DebugMoveFiles, || {
                        format!("Old file removed: {}", path.display())
                    });
                    remove_file(&path).chain_err(&err)?;
                }
            }
        }

        for item in read_dir(src).chain_err(&err)? {
            let item = item.chain_err(&err)?;
            let from = item.path().to_path_buf();
            let to = dst.with_added(item.file_name());
            move_files(&from, &to).chain_err(&err)?;
        }
        remove_dir_all(src).chain_err(&err)?;
    } else {
        move_one_file(src, dst).chain_err(&err)?;
    }
    Ok(())
}

/// Copy file or directory `src` to `dst` recursively
pub fn copy_recursively(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    let err = || format!("failed: copy_recursively({:?}, {:?})", src, dst);
    if src.as_path().is_dir() {
        create_dir(&dst).chain_err(&err)?;
        for item in read_dir(src).chain_err(&err)? {
            let item = item.chain_err(&err)?;
            let from = item.path().to_path_buf();
            let to = dst.with_added(item.file_name());
            copy_recursively(&from, &to).chain_err(&err)?;
        }
    } else {
        copy_file(src, dst).chain_err(&err)?;
    }
    Ok(())
}

/// Move file `old_path` to `new_path`. If contents of files are the same,
/// timestamps of the old file are preserved.
fn move_one_file(old_path: &PathBuf, new_path: &PathBuf) -> Result<()> {
    let err = || format!("failed: move_one_file({:?}, {:?})", old_path, new_path);
    let is_changed = if new_path.as_path().is_file() {
        let string1 = file_to_string(old_path).chain_err(&err)?;
        let string2 = file_to_string(new_path).chain_err(&err)?;
        string1 != string2
    } else {
        true
    };

    if is_changed {
        if new_path.as_path().exists() {
            remove_file(&new_path).chain_err(&err)?;
        }
        rename_file(&old_path, &new_path).chain_err(&err)?;
        log::llog(log::DebugMoveFiles, || {
            format!("File changed: {}", new_path.display())
        });
    } else {
        remove_file(&old_path).chain_err(&err)?;
        log::llog(log::DebugMoveFiles, || {
            format!("File not changed: {}", new_path.display())
        });
    }
    Ok(())
}

/// Adds `with_added` function for paths.
pub trait PathBufWithAdded {
    /// Appends `path` to `self` and returns it as new `PathBuf`,
    /// leaving `self` unchanged.
    fn with_added<P: AsRef<Path>>(&self, path: P) -> PathBuf;
}

impl<T: AsRef<Path>> PathBufWithAdded for T {
    fn with_added<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        let mut p = self.as_ref().to_path_buf();
        p.push(path);
        p
    }
}

/// A wrapper over `std::fs::File` containing ths file's  path.
pub struct FileWrapper {
    file: fs::File,
    path: PathBuf,
}

/// A wrapper over `std::fs::File::open` with better error reporting.
pub fn open_file<P: AsRef<Path>>(path: P) -> Result<FileWrapper> {
    Ok(FileWrapper {
        file: fs::File::open(path.as_ref())
            .chain_err(|| format!("Failed to open file for reading: {:?}", path.as_ref()))?,
        path: path.as_ref().to_path_buf(),
    })
}

/// Returns content of the file `path` as a string.
pub fn file_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut f = open_file(path)?;
    f.read_all()
}

/// A wrapper over `std::fs::File::create` with better error reporting.
pub fn create_file<P: AsRef<Path>>(path: P) -> Result<FileWrapper> {
    Ok(FileWrapper {
        file: fs::File::create(path.as_ref())
            .chain_err(|| format!("Failed to create file: {:?}", path.as_ref()))?,
        path: path.as_ref().to_path_buf(),
    })
}

/// A wrapper over `std::fs::OpenOptions::open` with better error reporting.
pub fn open_file_with_options<P: AsRef<Path>>(
    path: P,
    options: &fs::OpenOptions,
) -> Result<FileWrapper> {
    Ok(FileWrapper {
        file: options
            .open(path.as_ref())
            .chain_err(|| format!("Failed to open file: {:?}", path.as_ref()))?,
        path: path.as_ref().to_path_buf(),
    })
}

impl FileWrapper {
    /// Read content of the file to a string
    pub fn read_all(&mut self) -> Result<String> {
        let mut r = String::new();
        self.file
            .read_to_string(&mut r)
            .chain_err(|| format!("Failed to read from file: {:?}", self.path))?;
        Ok(r)
    }

    /// Write `text` to the file
    pub fn write<S: AsRef<str>>(&mut self, text: S) -> Result<()> {
        use std::io::Write;
        self.file
            .write_all(text.as_ref().as_bytes())
            .chain_err(|| format!("Failed to write to file: {:?}", self.path))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns underlying `std::fs::File`
    pub fn into_file(self) -> fs::File {
        self.file
    }
}

/// Deserialize value from JSON file `path`.
pub fn load_json<P: AsRef<Path>, T: serde::de::DeserializeOwned>(path: P) -> Result<T> {
    let file = open_file(path.as_ref())?;
    ::serde_json::from_reader(file.into_file())
        .chain_err(|| format!("failed to parse file as JSON: {}", path.as_ref().display()))
}

/// Serialize `value` into JSON file `path`.
pub fn save_json<P: AsRef<Path>, T: ::serde::Serialize>(path: P, value: &T) -> Result<()> {
    let tmp_path = {
        let mut buf = path.as_ref().to_path_buf();
        let tmp_file_name = format!("{}.new", os_str_to_str(&buf.file_name().unwrap())?);
        buf.set_file_name(tmp_file_name);
        buf
    };
    {
        let file = create_file(&tmp_path)?;
        ::serde_json::to_writer(&mut file.into_file(), value).chain_err(|| {
            format!(
                "failed to serialize to JSON file: {}",
                path.as_ref().display()
            )
        })?;
    }
    if path.as_ref().exists() {
        remove_file(path.as_ref())?;
    }
    rename_file(&tmp_path, path.as_ref())?;
    Ok(())
}

/// Deserialize value from binary file `path`.
pub fn load_bincode<P: AsRef<Path>, T: serde::de::DeserializeOwned>(path: P) -> Result<T> {
    let mut file = open_file(path.as_ref())?.into_file();
    bincode::deserialize_from(&mut file)
        .chain_err(|| format!("load_bincode failed: {}", path.as_ref().display()))
}

/// Serialize `value` into binary file `path`.
pub fn save_bincode<P: AsRef<Path>, T: ::serde::Serialize>(path: P, value: &T) -> Result<()> {
    let mut file = create_file(path.as_ref())?.into_file();
    bincode::serialize_into(&mut file, value)
        .chain_err(|| format!("save_bincode failed: {}", path.as_ref().display()))
}

/// Load data from a TOML file
pub fn load_toml<P: AsRef<Path>>(path: P) -> Result<toml::value::Table> {
    let data = file_to_string(path.as_ref())?;
    let value: toml::Value = data
        .parse()
        .chain_err(|| format!("failed to parse TOML file: {}", path.as_ref().display()))?;
    if let toml::value::Value::Table(table) = value {
        Ok(table)
    } else {
        Err("TOML is not a table".into())
    }
}

/// Save `data` to a TOML file
pub fn save_toml<P: AsRef<Path>>(path: P, data: &toml::Value) -> Result<()> {
    let mut file = create_file(path.as_ref())?;
    file.write(data.to_string())
        .chain_err(|| format!("failed to write to TOML file: {}", path.as_ref().display()))
}

/// A wrapper over `std::fs::create_dir` with better error reporting
pub fn create_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    fs::create_dir(path.as_ref()).chain_err(|| format!("Failed to create dir: {:?}", path.as_ref()))
}

/// A wrapper over `std::fs::create_dir_all` with better error reporting
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    fs::create_dir_all(path.as_ref()).chain_err(|| {
        format!(
            "Failed to create dirs (with parent components): {:?}",
            path.as_ref()
        )
    })
}

/// A wrapper over `std::fs::remove_dir` with better error reporting
pub fn remove_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    fs::remove_dir(path.as_ref()).chain_err(|| format!("Failed to remove dir: {:?}", path.as_ref()))
}

/// A wrapper over `std::fs::remove_dir_all` with better error reporting
pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    fs::remove_dir_all(path.as_ref())
        .chain_err(|| format!("Failed to remove dir (recursively): {:?}", path.as_ref()))
}

/// A wrapper over `std::fs::remove_file` with better error reporting
pub fn remove_file<P: AsRef<Path>>(path: P) -> Result<()> {
    fs::remove_file(path.as_ref())
        .chain_err(|| format!("Failed to remove file: {:?}", path.as_ref()))
}

/// A wrapper over `std::fs::rename` with better error reporting
pub fn rename_file<P: AsRef<Path>, P2: AsRef<Path>>(path1: P, path2: P2) -> Result<()> {
    fs::rename(path1.as_ref(), path2.as_ref()).chain_err(|| {
        format!(
            "Failed to rename file from {:?} to {:?}",
            path1.as_ref(),
            path2.as_ref()
        )
    })
}

/// A wrapper over `std::fs::copy` with better error reporting
pub fn copy_file<P: AsRef<Path>, P2: AsRef<Path>>(path1: P, path2: P2) -> Result<()> {
    fs::copy(path1.as_ref(), path2.as_ref())
        .map(|_| ())
        .chain_err(|| {
            format!(
                "Failed to copy file from {:?} to {:?}",
                path1.as_ref(),
                path2.as_ref()
            )
        })
}

/// A wrapper over `std::fs::DirEntry` iterator with better error reporting
pub struct ReadDirWrapper {
    read_dir: fs::ReadDir,
    path: PathBuf,
}

/// A wrapper over `std::fs::read_dir` with better error reporting
pub fn read_dir<P: AsRef<Path>>(path: P) -> Result<ReadDirWrapper> {
    Ok(ReadDirWrapper {
        read_dir: fs::read_dir(path.as_ref())
            .chain_err(|| format!("Failed to read dir: {:?}", path.as_ref()))?,
        path: path.as_ref().to_path_buf(),
    })
}

impl Iterator for ReadDirWrapper {
    type Item = Result<fs::DirEntry>;
    fn next(&mut self) -> Option<Result<fs::DirEntry>> {
        self.read_dir.next().map(|value| {
            value.chain_err(|| format!("Failed to read dir (in item): {:?}", self.path))
        })
    }
}

/// Canonicalize `path`. Similar to `std::fs::canonicalize`, but
/// `\\?\` prefix is removed. Windows implementation of `std::fs::canonicalize`
/// adds this prefix, but many tools don't process it correctly, including
/// CMake and compilers.
pub fn canonicalize<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let r = fs::canonicalize(path.as_ref())
        .chain_err(|| format!("failed to canonicalize {}", path.as_ref().display()))?;
    {
        let str = path_to_str(&r)?;
        if str.starts_with(r"\\?\") {
            return Ok(PathBuf::from(&str[4..]));
        }
    }
    Ok(r)
}

/// A wrapper over `Path::to_str` with better error reporting
pub fn path_to_str(path: &Path) -> Result<&str> {
    path.to_str()
        .chain_err(|| format!("Path is not valid unicode: {}", path.display()))
}

use std::ffi::{OsStr, OsString};

/// A wrapper over `OsStr::to_str` with better error reporting
pub fn os_str_to_str(os_str: &OsStr) -> Result<&str> {
    os_str
        .to_str()
        .chain_err(|| format!("String is not valid unicode: {}", os_str.to_string_lossy()))
}

/// A wrapper over `OsString::into_string` with better error reporting
pub fn os_string_into_string(s: OsString) -> Result<String> {
    s.into_string()
        .map_err(|s| format!("String is not valid unicode: {}", s.to_string_lossy()).into())
}

/// Returns current absolute path of `relative_path`.
/// `relative_path` is relative to the repository root.
pub fn repo_crate_local_path(relative_path: &str) -> Result<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parent = path
        .parent()
        .chain_err(|| "failed to get parent directory")?;
    let parent2 = parent
        .parent()
        .chain_err(|| "failed to get parent directory")?;
    let result = parent2.with_added(relative_path);
    if !result.exists() {
        return Err(format!("detected path does not exist: {}", result.display()).into());
    }
    Ok(result)
}
