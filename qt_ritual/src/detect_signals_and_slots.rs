use itertools::Itertools;
use log::trace;
use regex::Regex;
use ritual::cpp_data::{CppItem, CppPath};
use ritual::database::DatabaseItemSource;
use ritual::processor::ProcessorData;
use ritual_common::errors::{Result, ResultExt};
use ritual_common::file_utils::open_file;
use std::collections::{HashMap, HashSet};

/// Checks if `class_name` types inherits `base_name` type directly or indirectly.
pub fn inherits(
    data: &ProcessorData<'_>,
    derived_class_name: &CppPath,
    base_class_name: &CppPath,
) -> bool {
    for item in data.all_cpp_items() {
        if let CppItem::ClassBase(base_data) = &item.item {
            if &base_data.derived_class_type == derived_class_name {
                if &base_data.base_class_type == base_class_name {
                    return true;
                }
                if inherits(data, &base_data.base_class_type, base_class_name) {
                    return true;
                }
            }
        }
    }
    false
}

#[derive(Debug, Clone)]
enum SectionType {
    Signals,
    Slots,
    Other,
}
#[derive(Debug)]
struct Section {
    line: usize,
    section_type: SectionType,
}

/// Parses include files to detect which methods are signals or slots.
#[allow(clippy::cognitive_complexity, clippy::collapsible_if)]
pub fn detect_signals_and_slots(data: &mut ProcessorData<'_>) -> Result<()> {
    // TODO: only run if it's a new class or it has some new methods; don't change existing old methods
    let mut files = HashSet::new();

    let qobject_path = CppPath::from_good_str("QObject");
    for item in data.current_database.cpp_items() {
        if let DatabaseItemSource::CppParser {
            origin_location, ..
        } = &item.source
        {
            if let CppItem::Type(type1) = &item.item {
                if type1.kind.is_class() {
                    if type1.path == qobject_path || inherits(&data, &type1.path, &qobject_path) {
                        if !files.contains(&origin_location.include_file_path) {
                            files.insert(origin_location.include_file_path.clone());
                        }
                    }
                }
            }
        }
    }

    if files.is_empty() {
        return Ok(());
    }
    let re_signals = Regex::new(r"(signals|Q_SIGNALS)\s*:")?;
    let re_slots = Regex::new(r"(slots|Q_SLOTS)\s*:")?;
    let re_other = Regex::new(r"(public|protected|private)\s*:")?;
    let mut sections = HashMap::new();

    for file_path in files {
        let mut file_sections = Vec::new();
        let file = open_file(&file_path)?;
        for (line_num, line) in file.lines().enumerate() {
            let line =
                line.with_context(|_| format!("failed while reading lines from {}", &file_path))?;
            let section_type = if re_signals.is_match(&line) {
                Some(SectionType::Signals)
            } else if re_slots.is_match(&line) {
                Some(SectionType::Slots)
            } else if re_other.is_match(&line) {
                Some(SectionType::Other)
            } else {
                None
            };
            if let Some(section_type) = section_type {
                file_sections.push(Section {
                    line: line_num,
                    section_type,
                });
            }
        }
        // println!("sections: {:?}", file_sections);
        if !file_sections.is_empty() {
            sections.insert(file_path, file_sections);
        }
    }

    let mut sections_per_class = HashMap::new();
    for item in data.current_database.cpp_items() {
        if let DatabaseItemSource::CppParser {
            origin_location, ..
        } = &item.source
        {
            if let CppItem::Type(type1) = &item.item {
                if let Some(sections) = sections.get(&origin_location.include_file_path) {
                    let sections_for_class = sections
                        .iter()
                        .filter(|x| x.line + 1 >= origin_location.line as usize)
                        .collect_vec();
                    sections_per_class.insert(type1.path.clone(), sections_for_class);
                }
            }
        }
    }

    for item in data.current_database.cpp_items_mut() {
        if let DatabaseItemSource::CppParser {
            origin_location, ..
        } = &item.source
        {
            if let CppItem::Function(method) = &mut item.item {
                let mut section_type = SectionType::Other;
                if let Ok(class_name) = method.class_type() {
                    if let Some(sections) = sections_per_class.get(&class_name) {
                        let matching_sections = sections
                            .clone()
                            .into_iter()
                            .filter(|x| x.line < origin_location.line as usize)
                            .collect_vec();
                        if !matching_sections.is_empty() {
                            let section = matching_sections[matching_sections.len() - 1];
                            section_type = section.section_type.clone();
                            match section.section_type {
                                SectionType::Signals => {
                                    trace!("Found signal: {}", method.short_text());
                                }
                                SectionType::Slots => {
                                    trace!("Found slot: {}", method.short_text());
                                }
                                SectionType::Other => {}
                            }
                        }
                    }
                }
                if let Some(info) = &mut method.member {
                    match section_type {
                        SectionType::Signals => {
                            info.is_signal = true;
                        }
                        SectionType::Slots => {
                            info.is_slot = true;
                        }
                        SectionType::Other => {}
                    }
                }
            }
        }
    }
    Ok(())
}
