
mod structs;
use structs::*;
mod c_generator;

mod read_parse_result;
use std::fs;
use std::path::{PathBuf, Path};
use std::io;
use std::process::Command;

extern crate find_folder;


fn copy_dir<P, Q>(from: P, to: Q)  where P: AsRef<Path>, Q: AsRef<Path> {
  let output = Command::new("cp")
  .arg("-r")
  .arg(from.as_ref().as_os_str())
  .arg(to.as_ref().as_os_str())
  .output()
  .unwrap();
  if !output.status.success() { panic!("cp failed"); }
}

fn remove_dir(path: &PathBuf) {
  match fs::metadata(path) {
    Ok(metadata) => {
      if metadata.is_dir() {
        fs::remove_dir_all(path).unwrap();
      } else {
        fs::remove_file(path).unwrap();
      }
    }
    _ => {}
  }
}

fn main() {
  let output_dir = PathBuf::from("../generated_output");
  let qtcw_template_dir = find_folder::Search::ParentsThenKids(3, 3)
                            .for_folder("qtcw_template")
                            .unwrap();
  let parse_result_path = {
    let mut r = output_dir.clone();
    r.push("doc_parse_result.json");
    r
  };
  let parse_result = read_parse_result::do_it(&parse_result_path);

  let qtcw_path = {
    let mut r = output_dir.clone();
    r.push("qtcw");
    r
  };
  remove_dir(&qtcw_path);
  copy_dir(&qtcw_template_dir, &qtcw_path);

  let sized_classes = c_generator::generate_size_definer_class_list(&parse_result, &qtcw_path);

  for data in parse_result {

    // white list for now
    if let Some(ref class_name) = data.class_name {
      if class_name != "QPoint" { continue; }
    } else {
      continue;
    }
    c_generator::generate_one(&data, &qtcw_path, &sized_classes);


  }







  // println!("data: {:?}", parse_result);
}
