//! Function for replacing plain boring names of the header files with
//! Qt's shortcut header names.

use log::{info, trace};
use ritual::database::CppDatabaseItem;
use ritual::database::CppItemData;
use ritual::database::DatabaseItemSource;
use ritual_common::errors::{err_msg, Result, ResultExt};
use ritual_common::file_utils::{file_to_string, os_str_to_str, read_dir};
use ritual_common::utils::add_to_multihash;
use std::collections::HashMap;
use std::path::PathBuf;

struct HeaderNameMap {
    map_real_to_all_fancy: HashMap<String, Vec<String>>,
    map_real_to_fancy: HashMap<String, String>,
}

impl HeaderNameMap {
    fn real_to_fancy(&self, real_header: &str, class_name: Option<&str>) -> String {
        if let Some(class_name) = class_name {
            if let Some(fancy_headers) = self.map_real_to_all_fancy.get(real_header) {
                if let Some(x) = fancy_headers
                    .iter()
                    .find(|&x| x == class_name || class_name.starts_with(&format!("{}::", x)))
                {
                    return x.clone();
                }
            }
        }
        if let Some(fancy_header) = self.map_real_to_fancy.get(real_header) {
            return fancy_header.clone();
        }
        real_header.to_string()
    }

    fn new(headers_dir: &PathBuf) -> Result<HeaderNameMap> {
        let re = ::regex::Regex::new(r#"^#include "([a-zA-Z0-9._]+)"$"#)?;
        let mut map_real_to_all_fancy: HashMap<_, Vec<_>> = HashMap::new();
        info!("Detecting fancy Qt header names");
        for header in read_dir(headers_dir)? {
            let header = header?;
            let header_path = header.path();
            if !header_path.is_file() {
                continue;
            }
            let metadata = ::std::fs::metadata(&header_path).with_context(|_| {
                format!("failed to get metadata for {}", header_path.display())
            })?;
            if metadata.len() < 100 {
                let file_content = file_to_string(&header_path)?;
                if let Some(matches) = re.captures(file_content.trim()) {
                    let real_header = matches
                        .get(1)
                        .ok_or_else(|| err_msg("invalid regexp matches"))?
                        .as_str()
                        .to_string();
                    let fancy_header = os_str_to_str(&header.file_name())?.to_string();
                    add_to_multihash(&mut map_real_to_all_fancy, real_header, fancy_header);
                }
            }
        }
        if map_real_to_all_fancy.contains_key("qsharedpointer.h") {
            let v = map_real_to_all_fancy["qsharedpointer.h"].clone();
            map_real_to_all_fancy.insert("qsharedpointer_impl.h".to_string(), v);
        }
        let mut map_real_to_fancy = HashMap::new();
        for (real_header, fancy_headers) in &map_real_to_all_fancy {
            let fancy_header = if fancy_headers.len() == 1 {
                fancy_headers[0].clone()
            } else {
                let mut result = fancy_headers[0].clone();
                let mut ok = false;
                for h in fancy_headers {
                    if format!("{}.h", h.to_lowercase()) == *real_header {
                        result = h.clone();
                        ok = true;
                        break;
                    }
                }
                if !ok {
                    trace!("{} -> {:?} (detect failed)", real_header, fancy_headers);
                }
                result
            };
            trace!("{} -> {}", real_header, fancy_header);
            map_real_to_fancy.insert(real_header.clone(), fancy_header);
        }
        Ok(HeaderNameMap {
            map_real_to_all_fancy,
            map_real_to_fancy,
        })
    }
}

/// Replaces names of header files in `data` with Qt's shortcut headers.
pub fn fix_header_names(data: &mut [CppDatabaseItem], headers_dir: &PathBuf) -> Result<()> {
    // TODO: only run on new database items?
    let map = HeaderNameMap::new(headers_dir)?;
    for item in data {
        let class_name = match &item.cpp_data {
            CppItemData::Type(data) => Some(data.path.doc_id()),
            CppItemData::Function(data) => data.class_type().ok().map(|x| x.doc_id()),
            _ => continue,
        };

        if let DatabaseItemSource::CppParser { include_file, .. } = &mut item.source {
            let new_include_file =
                map.real_to_fancy(include_file, class_name.as_ref().map(|s| s.as_str()));
            *include_file = new_include_file;
        }
    }
    Ok(())
}

#[test]
fn test_qt_fix_header_names() {
    let map = HeaderNameMap::new(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_assets")
            .join("qt_headers"),
    )
    .unwrap();
    assert_eq!(map.real_to_fancy("qfile.h", None), "QFile");
    assert_eq!(map.real_to_fancy("qfile.h", Some("QFile")), "QFile");
    assert_eq!(map.real_to_fancy("qnotmap.h", None), "qnotmap.h");
    assert_eq!(
        map.real_to_fancy("qfactoryinterface.h", None),
        "QFactoryInterface"
    );
    assert_eq!(
        map.real_to_fancy("qfactoryinterface.h", Some("^_^")),
        "QFactoryInterface"
    );
}
