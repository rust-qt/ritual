use std::path::PathBuf;
use structs::*;
use std::fs::File;
use std::io::Write;

pub struct CGenerator {

  qtcw_path: PathBuf,
  all_data: Vec<CppHeaderData>,
  sized_classes: Vec<String>,
}



impl CGenerator {
  pub fn new(all_data: Vec<CppHeaderData>, qtcw_path: PathBuf) -> Self {
    CGenerator { all_data: all_data, qtcw_path: qtcw_path, sized_classes: Vec::new() }
  }

  pub fn generate_all(&mut self) {
    self.sized_classes = self.generate_size_definer_class_list();
    for data in &self.all_data {


      // white list for now
      if let Some(ref class_name) = data.class_name {
        if class_name != "QPoint" { continue; }
      } else {
        continue;
      }


      self.generate_one(data);

    }



  }




  pub fn generate_size_definer_class_list(&self)
  -> Vec<String> {
    let mut sized_classes = Vec::new();
    // TODO: black magic happens here
    let blacklist = vec!["QFlags", "QWinEventNotifier", "QPair", "QGlobalStatic"];

    let mut h_path = self.qtcw_path.clone();
    h_path.push("size_definer");
    h_path.push("classes_list.h");
    println!("Generating file: {:?}", h_path);
    let mut h_file = File::create(&h_path).unwrap();
    for item in &self.all_data {
      if item.involves_templates() {
        // TODO: support template classes!
        println!("Ignoring {} because it involves templates.",
        item.include_file);
        continue;
      }
      if let Some(ref class_name) = item.class_name {
        if class_name.contains("::") {
          // TODO: support nested classes!
          println!("Ignoring {} because it is a nested class.",
          item.include_file);
          continue;
        }
        if blacklist.iter().find(|&&x| x == class_name.as_ref() as &str).is_some() {
          println!("Ignoring {} because it is blacklisted.", item.include_file);
          continue;

        }
        println!("Requesting size definition for {}.", class_name);
        write!(h_file, "ADD({});\n", class_name).unwrap();
        sized_classes.push(class_name.clone());
      }
    }
    println!("Done.\n");
    sized_classes
  }


  pub fn generate_one(&self, data: &CppHeaderData) {
    println!("test {}", data.process_methods().len());
    let mut cpp_path = self.qtcw_path.clone();
    cpp_path.push("src");
    cpp_path.push(format!("qtcw_{}.cpp", data.include_file));
    println!("Generating source file: {:?}", cpp_path);

    let mut h_path = self.qtcw_path.clone();
    h_path.push("include");
    h_path.push(format!("qtcw_{}.h", data.include_file));
    println!("Generating header file: {:?}", h_path);

    let mut cpp_file = File::create(&cpp_path).unwrap();
    let mut h_file = File::create(&h_path).unwrap();

    write!(cpp_file, "#include \"qtcw_{}.h\"\n", data.include_file).unwrap();
    let include_guard_name = format!("QTCW_{}_H", data.include_file.to_uppercase());
    write!(h_file,
    "#ifndef {}\n#define {}\n\n",
    include_guard_name,
    include_guard_name)
    .unwrap();

    write!(h_file, "#include \"qtcw_global.h\"\n\n").unwrap();


    write!(h_file, "#ifdef __cplusplus\n").unwrap();
    write!(h_file, "#include <{}>\n", data.include_file).unwrap();
    write!(h_file, "#endif\n\n").unwrap();

    if let Some(ref class_name) = data.class_name {
      // write C struct definition
      write!(h_file, "#ifndef __cplusplus // if C\n").unwrap();
      if self.sized_classes.iter().find(|&x| x == class_name).is_some() {
        write!(h_file,
        "struct QTCW_{} {{ char space[QTCW_sizeof_{}]; }};\n",
        class_name,
        class_name)
        .unwrap();
      } else {
        write!(h_file, "struct QTCW_{};\n", class_name).unwrap();

      }
      write!(h_file,
      "typedef struct QTCW_{} {};\n",
      class_name,
      class_name)
      .unwrap();
      write!(h_file, "#endif\n\n").unwrap();

    } else {
      println!("Not a class header. Wrapper struct is not generated.");
    }

    write!(h_file, "QTCW_EXTERN_C_BEGIN\n\n").unwrap();

    for method in data.process_methods() {
      println!("method:\n{:?}\n\n", method);
      write!(h_file,
      "{} QTCW_EXPORT {}({});\n",
      method.c_signature.return_type.c_type.to_c_code(),
      method.c_name,
      method.c_signature.arguments_to_c_code())
      .unwrap();





    }

    write!(h_file, "\nQTCW_EXTERN_C_END\n\n").unwrap();




    write!(h_file, "#endif // {}\n", include_guard_name).unwrap();
    println!("Done.\n")
  }
}
