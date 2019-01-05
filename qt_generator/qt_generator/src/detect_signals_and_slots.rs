use cpp_to_rust_generator::common::errors::{Result, ResultExt};
use cpp_to_rust_generator::common::file_utils::open_file;
use cpp_to_rust_generator::cpp_data::CppPath;
use cpp_to_rust_generator::database::CppItemData;
use cpp_to_rust_generator::database::DatabaseItemSource;
use cpp_to_rust_generator::processor::ProcessorData;
use log::trace;
use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;

/// Checks if `class_name` types inherits `base_name` type directly or indirectly.
pub fn inherits(
    data: &ProcessorData,
    derived_class_name: &CppPath,
    base_class_name: &CppPath,
) -> bool {
    for item in data.all_items() {
        if let CppItemData::ClassBase(ref base_data) = item.cpp_data {
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

/// Parses include files to detect which methods are signals or slots.
#[allow(clippy::cyclomatic_complexity)]
pub fn detect_signals_and_slots(data: &mut ProcessorData) -> Result<()> {
    // TODO: only run if it's a new class or it has some new methods; don't change existing old methods
    let mut files = HashSet::new();

    for item in &data.current_database.items {
        if let DatabaseItemSource::CppParser {
            ref origin_location,
            ..
        } = item.source
        {
            if let CppItemData::Type(ref type1) = item.cpp_data {
                if type1.kind.is_class() {
                    if inherits(&data, &type1.path, &CppPath::from_str_unchecked("QObject")) {
                        if !files.contains(&origin_location.include_file_path) {
                            files.insert(origin_location.include_file_path.clone());
                        }
                    }
                }
            }
        }
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
    for item in &data.current_database.items {
        if let DatabaseItemSource::CppParser {
            ref origin_location,
            ..
        } = item.source
        {
            if let CppItemData::Type(ref type1) = item.cpp_data {
                if let Some(sections) = sections.get(&origin_location.include_file_path) {
                    let sections_for_class: Vec<_> = sections
                        .iter()
                        .filter(|x| x.line + 1 >= origin_location.line as usize)
                        .collect();
                    sections_per_class.insert(type1.path.clone(), sections_for_class);
                }
            }
        }
    }

    for item in &mut data.current_database.items {
        if let DatabaseItemSource::CppParser {
            ref origin_location,
            ..
        } = item.source
        {
            if let CppItemData::Function(ref mut method) = item.cpp_data {
                let mut section_type = SectionType::Other;
                if let Some(class_name) = method.class_type() {
                    if let Some(sections) = sections_per_class.get(&class_name) {
                        let matching_sections: Vec<_> = sections
                            .clone()
                            .into_iter()
                            .filter(|x| x.line < origin_location.line as usize)
                            .collect();
                        if !matching_sections.is_empty() {
                            let section = matching_sections[matching_sections.len() - 1];
                            section_type = section.section_type.clone();
                            match section.section_type {
                                SectionType::Signals => {
                                    trace!("[DebugSignals] Found signal: {}", method.short_text());
                                }
                                SectionType::Slots => {
                                    trace!("[DebugSignals] Found slot: {}", method.short_text());
                                }
                                SectionType::Other => {}
                            }
                        }
                    }
                }
                if let Some(ref mut info) = method.member {
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
