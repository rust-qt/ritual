use new_impl::final_type::FinalType;
use new_impl::final_method::FinalMethod;
//use common::errors::Result;

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct Database {
  crate_name: String,
  types: Vec<FinalType>,
  methods: Vec<FinalMethod>,
}

impl Database {
  pub fn empty(crate_name: &str) -> Database {
    Database {
      crate_name: crate_name.to_owned(),
      types: Vec::new(),
      methods: Vec::new(),
    }
  }

  pub fn crate_name(&self) -> &str {
    &self.crate_name
  }
}
