use itertools::Itertools;
use log::trace;
use regex::Regex;
use ritual::cpp_data::{inherits2, CppItem, CppPath};
use ritual::cpp_parser::{Context2, CppParserOutput};
use ritual_common::errors::{err_msg, Result, ResultExt};
use ritual_common::file_utils::open_file;
use std::collections::{HashMap, HashSet};

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
pub fn detect_signals_and_slots(data: Context2<'_>, output: &CppParserOutput) -> Result<()> {
    // TODO: only run if it's a new class or it has some new methods; don't change existing old methods
    let mut files = HashSet::new();

    let qobject_path = CppPath::from_good_str("QObject");
    for item in &output.0 {
        let cpp_item = data
            .current_database
            .items()
            .get(item.index)
            .ok_or_else(|| err_msg("invalid item index in CppParserContext"))?;

        if let CppItem::Type(type1) = &cpp_item.item {
            if type1.kind.is_class() {
                if inherits2(&data, &type1.path, &qobject_path) {
                    if !files.contains(&item.origin_location.include_file_path) {
                        files.insert(item.origin_location.include_file_path.clone());
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
    for item in &output.0 {
        let cpp_item = data
            .current_database
            .items()
            .get(item.index)
            .ok_or_else(|| err_msg("invalid item index in CppParserContext"))?;

        if let CppItem::Type(type1) = &cpp_item.item {
            if let Some(sections) = sections.get(&item.origin_location.include_file_path) {
                let sections_for_class = sections
                    .iter()
                    .filter(|x| x.line + 1 >= item.origin_location.line as usize)
                    .collect_vec();
                sections_per_class.insert(type1.path.clone(), sections_for_class);
            }
        }
    }

    for item in &output.0 {
        let cpp_item = data
            .current_database
            .items_mut()
            .get_mut(item.index)
            .ok_or_else(|| err_msg("invalid item index in CppParserContext"))?;
        if let CppItem::Function(method) = &mut cpp_item.item {
            let mut section_type = SectionType::Other;
            if let Ok(class_name) = method.class_path() {
                if let Some(sections) = sections_per_class.get(&class_name) {
                    let matching_sections = sections
                        .clone()
                        .into_iter()
                        .filter(|x| x.line < item.origin_location.line as usize)
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
    Ok(())
}
