use std::path::PathBuf;
use structs::*;
use std::fs::File;
use std::io::Write;


pub fn generate_size_definer_class_list(data: &Vec<CppHeaderData>, qtcw_path: &PathBuf) {
  // TODO: black magic happens here
  let blacklist = vec!["QFlags",
                       "QWinEventNotifier",
                       "QPair",
                       "QGlobalStatic"];

  let mut h_path = qtcw_path.clone();
  h_path.push("size_definer");
  h_path.push("classes_list.h");
  println!("Generating file: {:?}", h_path);
  let mut h_file = File::create(&h_path).unwrap();
  for item in data {
    if item.involves_templates() {
      println!("Ignoring {} because it involves templates.",
               item.include_file);
      continue;
    }
    if let Some(ref class_name) = item.class_name {
      if class_name.contains("::") {
        println!("Ignoring {} because it is a nested class.", item.include_file);
        continue;
      }
      if blacklist.iter().find(|&&x| x == class_name.as_ref() as &str).is_some() {
        println!("Ignoring {} because it is blacklisted.", item.include_file);
        continue;

      }
      println!("Requesting size definition for {}.", class_name);
      write!(h_file, "ADD({});\n", class_name);
    }
  }
  println!("Done.\n")
}


pub fn generate_one(data: &CppHeaderData, qtcw_path: &PathBuf) {
  println!("test {}", data.process_methods().len());
  let mut cpp_path = qtcw_path.clone();
  cpp_path.push("src");
  cpp_path.push(format!("qtcw_{}.cpp", data.include_file));
  println!("Generating source file: {:?}", cpp_path);

  let mut h_path = qtcw_path.clone();
  h_path.push("include");
  h_path.push(format!("qtcw_{}.h", data.include_file));
  println!("Generating header file: {:?}", h_path);

  let mut cpp_file = File::create(&cpp_path).unwrap();
  let mut h_file = File::create(&h_path).unwrap();

  write!(cpp_file, "#include \"qtcw_{}.h\"\n", data.include_file);
  let include_guard_name = format!("QTCW_{}_H", data.include_file.to_uppercase());
  write!(h_file,
         "#ifndef {}\n#define {}\n\n",
         include_guard_name,
         include_guard_name);

  if let Some(ref class_name) = data.class_name {




  } else {
    println!("Not a class header. Wrapper structs are not generated.");
  }




  write!(h_file, "#endif // {}\n", include_guard_name);
  println!("Done.\n")
}
