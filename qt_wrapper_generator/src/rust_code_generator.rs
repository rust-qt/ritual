use rust_type::{RustName, RustType, CompleteType, RustTypeIndirection, RustFFIFunction,
                RustFFIArgument, RustToCTypeConversion};
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use rust_info::{RustTypeDeclaration, RustTypeDeclarationKind, RustTypeWrapperKind};
use std::collections::{HashMap, HashSet};
use utils::JoinWithString;

fn rust_type_to_code(crate_name: &String, rust_type: &RustType) -> String {
  match rust_type {
    &RustType::Void => panic!("rust void can't be converted to code"),
    &RustType::NonVoid { ref base, ref is_const, ref indirection, ref is_option, .. } => {
      let base_s = base.full_name(&crate_name);
      let s = match indirection {
        &RustTypeIndirection::None => base_s,
        &RustTypeIndirection::Ref => {
          if *is_const {
            format!("&{}", base_s)
          } else {
            format!("&mut {}", base_s)
          }
        }
        &RustTypeIndirection::Ptr => {
          if *is_const {
            format!("*const {}", base_s)
          } else {
            format!("*mut {}", base_s)
          }
        }
      };
      if *is_option {
        format!("Option<{}>", s)
      } else {
        s
      }
    }
  }
}

fn rust_ffi_function_to_code(crate_name: &String, func: &RustFFIFunction) -> String {
  let args = func.arguments
                 .iter()
                 .map(|arg| format!("{}: {}", arg.name, rust_type_to_code(crate_name, &arg.argument_type)));
  format!("  pub fn {}({}){};\n",
          func.name.own_name,
          args.join(", "),
          match func.return_type {
            RustType::Void => String::new(),
            RustType::NonVoid { .. } => format!(" -> {}", rust_type_to_code(crate_name, &func.return_type)),
          })
}

pub fn generate_lib_file(output_path: &PathBuf, modules: &Vec<String>) {
  let mut lib_file_path = output_path.clone();
  lib_file_path.push("qt_core");
  lib_file_path.push("src");
  lib_file_path.push("lib.rs");
  let mut lib_file = File::create(&lib_file_path).unwrap();
  let built_in_modules = vec!["types", "flags", "extra", "ffi"];
  for module in built_in_modules {
    if modules.iter().find(|x| x.as_ref() as &str == module).is_some() {
      panic!("module name conflict");
    }
    if module == "ffi" {
      // TODO: remove allow directive
      // TODO: ffi should be a private mod
      write!(lib_file, "#[allow(dead_code)]\n").unwrap();
    }
    write!(lib_file, "pub mod {};\n\n", module).unwrap();
  }
  for module in modules {
    write!(lib_file, "pub mod {};\n", module).unwrap();
  }
}

pub fn generate_module_file(crate_name: &String,
                            module_name: &String,
                            output_path: &PathBuf,
                            types: &Vec<RustTypeDeclaration>) {
  let mut file_path = output_path.clone();
  file_path.push(crate_name);
  file_path.push("src");
  file_path.push(format!("{}.rs", module_name));
  let mut file = File::create(&file_path).unwrap();
  for type1 in types {
    match type1.kind {
      RustTypeDeclarationKind::CppTypeWrapper { ref kind, .. } => {
        match *kind {
          RustTypeWrapperKind::Enum { ref values } => {
            write!(file,
                   "#[repr(C)]\npub enum {} {{\n{}\n}}\n\n",
                   type1.name,
                   values.iter()
                         .map(|item| format!("  {} = {}", item.name, item.value))
                         .join(", \n"))
              .unwrap();
          }
          RustTypeWrapperKind::Struct { ref size } => {
            write!(file,
                   "#[repr(C)]\npub struct {} {{\n  _buffer: [u8; {}],\n}}\n\n",
                   type1.name,
                   size)
              .unwrap();
          }
        }
      }
      _ => unimplemented!(),
    }
  }

}


pub fn generate_ffi_file(crate_name: &String,
                         output_path: &PathBuf,
                         functions: &HashMap<String, Vec<RustFFIFunction>>) {
  let mut file_path = output_path.clone();
  file_path.push(crate_name);
  file_path.push("src");
  file_path.push("ffi.rs");
  let mut file = File::create(&file_path).unwrap();
  write!(file, "extern crate libc;\n\n").unwrap();
  write!(file, "#[link(name = \"Qt5Core\")]\n").unwrap();
  write!(file, "#[link(name = \"icui18n\")]\n").unwrap();
  write!(file, "#[link(name = \"icuuc\")]\n").unwrap();
  write!(file, "#[link(name = \"icudata\")]\n").unwrap();
  write!(file, "#[link(name = \"stdc++\")]\n").unwrap();
  write!(file, "#[link(name = \"qtcw\", kind = \"static\")]\n").unwrap();
  write!(file, "extern \"C\" {{\n").unwrap();

  for (include_file, functions) in functions {
    write!(file, "  // Header: {}\n", include_file).unwrap();
    for function in functions {
      file.write(rust_ffi_function_to_code(crate_name, function).as_bytes()).unwrap();
    }
    write!(file, "\n").unwrap();
  }
  write!(file, "}}\n").unwrap();
}
