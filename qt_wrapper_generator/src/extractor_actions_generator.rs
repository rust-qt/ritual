use cpp_data::CppData;
use enums::CppTypeKind;

use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

pub fn do_it(cpp_data: CppData, extractor_actions_path: PathBuf) {
  let show_output = true;
  println!("Generating file: {:?}", extractor_actions_path);
  let mut h_file = File::create(&extractor_actions_path).unwrap();
  for item in &cpp_data.headers {
    if let Some(ref class_name) = item.class_name {
      if cpp_data.classes_blacklist.iter().find(|&x| x == class_name.as_ref() as &str).is_some() {
        if show_output {
          println!("Ignoring {} because it is blacklisted.", item.include_file);
        }
        continue;
      }
      match cpp_data.is_template_class(class_name) {
        Err(msg) => {
          if show_output {
            println!("Ignoring {}: {}", class_name, msg);
          }
          continue;
        }
        Ok(is_template_class) => {
          if is_template_class {
            // TODO: support template classes!
            if show_output {
              println!("Ignoring {} because it is a template class.", class_name);
            }
            continue;
          }
        }
      }
      if show_output {
        println!("Requesting size definition for {}.", class_name);
      }
      write!(h_file, "  e.add_class<{0}>(\"{0}\");\n", class_name).unwrap();
    }
  }

  for (_, type_info) in &cpp_data.types.0 {
    if let CppTypeKind::Enum { ref values } = type_info.kind {
      println!("enum {:?}", type_info.name);
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

  println!("Done.\n");


}
