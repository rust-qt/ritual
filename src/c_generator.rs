use std::path::{PathBuf};
use structs::*;
use std::fs::File;
use std::io::Write;

pub fn generate_one(data: &CppHeaderData, qtcw_path: &PathBuf) {
  println!("test {}", data.process_methods().len());
  let mut cpp_path = qtcw_path.clone();
  cpp_path.push("src");
  cpp_path.push(format!("qtcw_{}.cpp", data.include_file));
  println!("file 1 {:?}", cpp_path);

  let mut h_path = qtcw_path.clone();
  h_path.push("include");
  h_path.push(format!("qtcw_{}.h", data.include_file));
  println!("file 2 {:?}", h_path);

  let mut cpp_file = File::create(&cpp_path).unwrap();
  let mut h_file = File::create(&h_path).unwrap();

  write!(cpp_file, "#include \"qtcw_{}.h\"\n", data.include_file);
  let include_guard_name = format!("QTCW_{}_H", data.include_file.to_uppercase());
  write!(h_file, "#ifndef {}\n#define {}\n\n", include_guard_name, include_guard_name);



  write!(h_file, "#endif // {}\n", include_guard_name);

}
