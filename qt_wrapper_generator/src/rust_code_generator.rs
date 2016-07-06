use rust_type::{RustName, RustType, CompleteType, RustTypeIndirection, RustFFIFunction,
                RustFFIArgument, RustToCTypeConversion};
use std::path::PathBuf;
use std::fs::File;
use std::io::{Write, Read};
use rust_info::{RustTypeDeclaration, RustTypeDeclarationKind, RustTypeWrapperKind, RustModule,
                RustMethod, RustMethodArguments, RustMethodArgumentsVariant};
use std::collections::{HashMap, HashSet};
use utils::JoinWithString;
use log;

extern crate rustfmt;

pub struct RustCodeGenerator {
  crate_name: String,
  output_path: PathBuf,
  rustfmt_config: rustfmt::config::Config,
}

impl RustCodeGenerator {
  pub fn new(crate_name: String, output_path: PathBuf) -> RustCodeGenerator {
    let mut rustfmt_config_path = output_path.clone();
    rustfmt_config_path.push("rustfmt.toml");
    log::info(format!("Using rustfmt config file: {:?}", rustfmt_config_path));
    let mut rustfmt_config_file = File::open(rustfmt_config_path).unwrap();
    let mut rustfmt_config_toml = String::new();
    rustfmt_config_file.read_to_string(&mut rustfmt_config_toml).unwrap();

    let rustfmt_config = rustfmt::config::Config::from_toml(&rustfmt_config_toml);
    RustCodeGenerator {
      crate_name: crate_name,
      output_path: output_path,
      rustfmt_config: rustfmt_config,
    }
  }

  fn rust_type_to_code(&self, rust_type: &RustType) -> String {
    match rust_type {
      &RustType::Void => panic!("rust void can't be converted to code"),
      &RustType::NonVoid { ref base,
                           ref is_const,
                           ref indirection,
                           ref is_option,
                           ref generic_arguments,
                           .. } => {
        let mut base_s = base.full_name(Some(&self.crate_name));
        if let &Some(ref args) = generic_arguments {
          base_s = format!("{}<{}>",
                           base_s,
                           args.iter().map(|x| self.rust_type_to_code(x)).join(", "));
        }
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
          }&RustTypeIndirection::PtrPtr => {
            if *is_const {
              format!("*const *const {}", base_s)
            } else {
              format!("*mut *mut {}", base_s)
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

  fn rust_ffi_function_to_code(&self, func: &RustFFIFunction) -> String {
    let args = func.arguments
                   .iter()
                   .map(|arg| {
                     format!("{}: {}",
                             arg.name,
                             self.rust_type_to_code(&arg.argument_type))
                   });
    format!("  pub fn {}({}){};\n",
            func.name,
            args.join(", "),
            match func.return_type {
              RustType::Void => String::new(),
              RustType::NonVoid { .. } => {
                format!(" -> {}", self.rust_type_to_code(&func.return_type))
              }
            })
  }

  fn generate_ffi_call(&self, func: &RustMethod, variant: &RustMethodArgumentsVariant) -> String {
    let mut final_args = Vec::new();
    final_args.resize(variant.cpp_method.c_signature.arguments.len(), None);
    for arg in &variant.arguments {
      assert!(arg.ffi_index >= 0 && arg.ffi_index < final_args.len() as i32);
      let mut code = arg.name.clone();
      match arg.argument_type.rust_api_to_c_conversion {
        RustToCTypeConversion::None => {}
        RustToCTypeConversion::RefToPtr => {
          code = format!("{} as {}",
                         code,
                         self.rust_type_to_code(&arg.argument_type.rust_ffi_type));

        }
        RustToCTypeConversion::ValueToPtr => {
          let is_const = if let RustType::NonVoid { ref is_const, .. } = arg.argument_type
                                                                            .rust_ffi_type {
            *is_const
          } else {
            panic!("void is not expected here at all!")
          };
          code = format!("{}{} as {}",
                         if is_const {
                           "&"
                         } else {
                           "&mut "
                         },
                         code,
                         self.rust_type_to_code(&arg.argument_type.rust_ffi_type));
        }
        RustToCTypeConversion::QFlagsToUInt => {
          code = format!("{}.to_int() as libc::c_uint", code);
        }
      }
      final_args[arg.ffi_index as usize] = Some(code);
    }

    let mut result = Vec::new();
    let mut maybe_result_var_name = None;
    if let Some(ref i) = func.return_type_ffi_index {
      let mut return_var_name = "object".to_string();
      let mut ii = 1;
      while variant.arguments.iter().find(|x| &x.name == &return_var_name).is_some() {
        ii += 1;
        return_var_name = format!("object{}", ii);
      }
      result.push(format!("{{\nlet mut {} = unsafe {{ {}::new_uninitialized() }};\n",
                          return_var_name,
                          self.rust_type_to_code(&func.return_type.rust_api_type)));
      final_args[*i as usize] = Some(format!("&mut {}", return_var_name));
      maybe_result_var_name = Some(return_var_name);
    }
    for arg in &final_args {
      if arg.is_none() {
        println!("func: {:?}", func);
        panic!("ffi argument is missing");
      }
    }
    result.push(format!("unsafe {{ ::ffi::{}({}) }}",
                        variant.cpp_method.c_name,
                        final_args.into_iter().map(|x| x.unwrap()).join(", ")));
    if let Some(ref name) = maybe_result_var_name {
      result.push(format!("{}\n}}", name));
    }
    let mut code = result.join("");
    match func.return_type.rust_api_to_c_conversion {
      RustToCTypeConversion::None => {}
      RustToCTypeConversion::RefToPtr => {
        let is_const = if let RustType::NonVoid { ref is_const, .. } = func.return_type
                                                                           .rust_ffi_type {
          *is_const
        } else {
          panic!("void is not expected here at all!")
        };
        code = format!("let ffi_result = {};\nunsafe {{ {}*ffi_result }}",
                       code,
                       if is_const {
                         "& "
                       } else {
                         "&mut "
                       });
      }
      RustToCTypeConversion::ValueToPtr => {
        if maybe_result_var_name.is_none() {
          code = format!("let ffi_result = {};\nunsafe {{ *ffi_result }}", code);
        }
      }
      RustToCTypeConversion::QFlagsToUInt => {
        let mut qflags_type = func.return_type.rust_api_type.clone();
        if let RustType::NonVoid { ref mut generic_arguments, .. } = qflags_type {
          *generic_arguments = None;
        } else {
          unreachable!();
        }
        code = format!("let ffi_result = {};\n{}::from_int(ffi_result as i32)",
                       code,
                       self.rust_type_to_code(&qflags_type));
      }
    }
    return code;
  }

  fn generate_rust_final_function(&self, func: &RustMethod) -> String {
    //    if func.name == "q_uncompress" {
    //      println!("TEST: {:?}", func);
    //    }
    match func.arguments {
      RustMethodArguments::SingleVariant(ref variant) => {
        let body = self.generate_ffi_call(func, variant);

        let args = variant.arguments
                          .iter()
                          .map(|arg| {
                            format!("{}: {}",
                                    arg.name,
                                    self.rust_type_to_code(&arg.argument_type.rust_api_type))
                          });
        format!("pub fn {}({}){} {{\n{}}}\n\n",
                func.name.last_name(),
                args.join(", "),
                match func.return_type.rust_api_type {
                  RustType::Void => String::new(),
                  RustType::NonVoid { .. } => {
                    format!(" -> {}",
                            self.rust_type_to_code(&func.return_type.rust_api_type))
                  }
                },
                body)
      }
      RustMethodArguments::MultipleVariants { .. } => {
        unimplemented!();
      }
    }
  }

  pub fn generate_lib_file(&self, output_path: &PathBuf, modules: &Vec<String>) {
    let mut lib_file_path = output_path.clone();
    lib_file_path.push("qt_core");
    lib_file_path.push("src");
    lib_file_path.push("lib.rs");
    {
      let mut lib_file = File::create(&lib_file_path).unwrap();
      let built_in_modules = vec!["types", "flags", "ffi"];
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
    self.call_rustfmt(&lib_file_path);
  }

  fn generate_module_code(&self, data: &RustModule) -> String {
    let mut results = Vec::new();
    results.push("extern crate libc;\n#[allow(unused_imports)]\nuse std;\n\n".to_string());

    for type1 in &data.types {
      let r = match type1.kind {
        RustTypeDeclarationKind::CppTypeWrapper { ref kind, .. } => {
          match *kind {
            RustTypeWrapperKind::Enum { ref values, ref is_flaggable } => {
              let mut r = format!("#[derive(Debug, PartialEq, Eq, Clone)]\n#[repr(C)]\npub enum \
                                   {} {{\n{}\n}}\n\n",
                                  type1.name.last_name(),
                                  values.iter()
                                        .map(|item| format!("  {} = {}", item.name, item.value))
                                        .join(", \n"));
              if *is_flaggable {
                r = format!("{}impl ::flags::FlaggableEnum for {} {{\n
                           \
                             fn to_int(self) -> libc::c_int {{ unsafe {{ \
                             std::mem::transmute(self) }} }}\n
                           fn \
                             enum_name() -> &'static str {{ unimplemented!() }}\n
                        \
                             }}\n\n",
                            r,
                            type1.name.last_name());
              }
              r
            }
            RustTypeWrapperKind::Struct { ref size } => {
              format!("#[repr(C)]\npub struct {name} {{\n  _buffer: [u8; {size}],\n}}\n\n
                       impl {name} {{ pub unsafe fn new_uninitialized() -> {name} {{
                         {name} {{ _buffer: std::mem::uninitialized() }}
                      }} }}\n\n",
                      name = type1.name.last_name(),
                      size = size)
            }
          }
        }
        _ => unimplemented!(),
      };
      results.push(r);
      if !type1.methods.is_empty() {
        results.push(format!("impl {} {{\n{}}}\n\n",
                             type1.name.last_name(),
                             type1.methods
                                  .iter()
                                  .map(|method| self.generate_rust_final_function(method))
                                  .join("")));
      }
    }
    for method in &data.functions {
      results.push(self.generate_rust_final_function(method));
    }

    for submodule in &data.submodules {
      results.push(format!("pub mod {} {{\n{}}}\n\n",
                           submodule.name.last_name(),
                           self.generate_module_code(submodule)));
    }
    return results.join("");
  }

  fn call_rustfmt(&self, path: &PathBuf) {
    let rustfmt_result = rustfmt::run(rustfmt::Input::File(path.clone()), &self.rustfmt_config);
    if !rustfmt_result.has_no_errors() {
      log::warning(format!("rustfmt failed to format file: {:?}", path));
    }
  }

  pub fn generate_module_file(&self, data: &RustModule) {
    let mut file_path = self.output_path.clone();
    file_path.push(&self.crate_name);
    file_path.push("src");
    file_path.push(format!("{}.rs", &data.name.last_name()));
    {
      let mut file = File::create(&file_path).unwrap();
      file.write(self.generate_module_code(data).as_bytes()).unwrap();
    }
    self.call_rustfmt(&file_path);

  }

  pub fn generate_ffi_file(&self, functions: &HashMap<String, Vec<RustFFIFunction>>) {
    let mut file_path = self.output_path.clone();
    file_path.push(&self.crate_name);
    file_path.push("src");
    file_path.push("ffi.rs");
    {
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
          file.write(self.rust_ffi_function_to_code(function).as_bytes()).unwrap();
        }
        write!(file, "\n").unwrap();
      }
      write!(file, "}}\n").unwrap();
    }
    // self.call_rustfmt(&file_path);
  }
}
