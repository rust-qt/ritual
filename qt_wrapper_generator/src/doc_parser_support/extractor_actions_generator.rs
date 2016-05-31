use doc_parser_support::cpp_data::CppData;
use doc_parser_support::enums::CppTypeKind;
use log;

use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

#[allow(dead_code)]
pub fn do_it(cpp_data: CppData, extractor_actions_path: PathBuf) {
  let show_output = true;
  log::info(format!("Generating file: {:?}", extractor_actions_path));
  let mut h_file = File::create(&extractor_actions_path).unwrap();
  for item in &cpp_data.headers {
    if let Some(ref class_name) = item.class_name {
      if cpp_data.classes_blacklist.iter().find(|&x| x == class_name.as_ref() as &str).is_some() {
        if show_output {
          log::warning(format!("Ignoring {} because it is blacklisted.", item.include_file));
        }
        continue;
      }
      if show_output {
        log::debug(format!("Requesting size definition for {}.", class_name));
      }
      write!(h_file, "  e.add_class<{0}>(\"{0}\");\n", class_name).unwrap();
    }
  }

  for (_, type_info) in &cpp_data.types.0 {
    if let CppTypeKind::Enum { ref values } = type_info.kind {
      log::debug(format!("Requesting enum values for {:?}.", type_info.name));
      let enum_cpp_namespace = match type_info.name.rfind("::") {
        Some(enum_last_part_index) => type_info.name[0..enum_last_part_index + 2].to_string(),
        None => String::new(),
      };
      for value in values {
        write!(h_file,
               "  e.add_enum_value(\"{0}\", \"{1}\", {2}{1});\n",
               type_info.name,
               value.name,
               enum_cpp_namespace)
          .unwrap();
      }
    }
  }
}
