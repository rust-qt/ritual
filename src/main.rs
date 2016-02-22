
mod structs;
use structs::*;

mod read_parse_result;



fn main() {
  let parse_result = read_parse_result::do_it(&std::path::PathBuf::from("../generated_output/doc_parse_result.json"));
  println!("data: {:?}", parse_result);
}
