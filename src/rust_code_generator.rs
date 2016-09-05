use rust_type::{RustType, RustTypeIndirection, RustFFIFunction, RustToCTypeConversion};
use std::path::PathBuf;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use rust_info::{RustTypeDeclarationKind, RustTypeWrapperKind, RustModule, RustMethod,
                RustMethodArguments, RustMethodArgumentsVariant, RustMethodScope,
                RustMethodArgument, TraitName};
use utils::{JoinWithString, copy_recursively};
use log;
use utils::PathBufPushTweak;
use std::panic;
use utils::CaseOperations;
use rust_generator::RustGeneratorOutput;

extern crate rustfmt;
extern crate toml;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvokationMethod {
  CommandLine,
  BuildScript,
}

pub struct RustCodeGeneratorConfig {
  pub invokation_method: InvokationMethod,
  pub crate_name: String,
  pub crate_version: String,
  pub crate_authors: Vec<String>,
  pub output_path: PathBuf,
  pub template_path: PathBuf,
  pub c_lib_name: String,
  pub cpp_lib_name: String,
  pub rustfmt_config_path: Option<PathBuf>,
}

fn format_doc(doc: &String) -> String {
  doc.split("\n")
    .map(|x| {
      let mut line = format!("/// {}\n", x);
      if line.starts_with("///     ") {
        // block doc tests
        line = line.replace("///     ", "/// &#32;   ");
      }
      line
    })
    .join("")
}

pub fn rust_type_to_code(rust_type: &RustType, crate_name: &String) -> String {
  match *rust_type {
    RustType::Void => "()".to_string(),
    RustType::Common { ref base, ref is_const, ref indirection, ref generic_arguments, .. } => {
      let mut base_s = base.full_name(Some(crate_name));
      if let &Some(ref args) = generic_arguments {
        base_s = format!("{}<{}>",
                         base_s,
                         args.iter().map(|x| rust_type_to_code(x, crate_name)).join(", "));
      }
      let s = match indirection {
        &RustTypeIndirection::None => base_s,
        &RustTypeIndirection::Ref { ref lifetime } => {
          let lifetime_text = match *lifetime {
            Some(ref lifetime) => format!("'{} ", lifetime),
            None => String::new(),
          };
          if *is_const {
            format!("&{}{}", lifetime_text, base_s)
          } else {
            format!("&{}mut {}", lifetime_text, base_s)
          }
        }
        &RustTypeIndirection::Ptr => {
          if *is_const {
            format!("*const {}", base_s)
          } else {
            format!("*mut {}", base_s)
          }
        }
        &RustTypeIndirection::PtrPtr => {
          if *is_const {
            format!("*const *const {}", base_s)
          } else {
            format!("*mut *mut {}", base_s)
          }
        }
      };
      s
    }
    RustType::FunctionPointer { ref return_type, ref arguments } => {
      format!("extern \"C\" fn({}){}",
              arguments.iter().map(|arg| rust_type_to_code(arg, crate_name)).join(", "),
              match return_type.as_ref() {
                &RustType::Void => String::new(),
                return_type => format!(" -> {}", rust_type_to_code(return_type, crate_name)),
              })
    }
  }
}


pub fn run(config: RustCodeGeneratorConfig, data: &RustGeneratorOutput) {
  let rustfmt_config_data = match config.rustfmt_config_path {
    Some(ref path) => {
      log::info(format!("Using rustfmt config file: {:?}", path));
      let mut rustfmt_config_file = File::open(path).unwrap();
      let mut rustfmt_config_toml = String::new();
      rustfmt_config_file.read_to_string(&mut rustfmt_config_toml).unwrap();
      rustfmt_config_toml
    }
    None => include_str!("../templates/crate/rustfmt.toml").to_string(),
  };
  let rustfmt_config = rustfmt::config::Config::from_toml(&rustfmt_config_data);
  let generator = RustCodeGenerator {
    config: config,
    rustfmt_config: rustfmt_config,
  };
  generator.generate_template();
  for module in &data.modules {
    generator.generate_module_file(module);
  }
  let mut module_names: Vec<_> = data.modules.iter().map(|x| &x.name).collect();
  module_names.sort();
  generator.generate_lib_file(&module_names);

  generator.generate_ffi_file(&data.ffi_functions);

}

pub struct RustCodeGenerator {
  config: RustCodeGeneratorConfig,
  pub rustfmt_config: rustfmt::config::Config,
}

impl RustCodeGenerator {
  /// Generates cargo file and skeleton of the crate
  pub fn generate_template(&self) {
    match self.config.rustfmt_config_path {
      Some(ref path) => {
        fs::copy(path, self.config.output_path.with_added("rustfmt.toml")).unwrap();
      }
      None => {
        let mut rustfmt_file = File::create(self.config.output_path.with_added("rustfmt.toml"))
          .unwrap();
        rustfmt_file.write(include_bytes!("../templates/crate/rustfmt.toml")).unwrap();
      }
    };

    let mut build_rs_file = File::create(self.config.output_path.with_added("build.rs")).unwrap();
    build_rs_file.write(include_bytes!("../templates/crate/build.rs")).unwrap();

    let cargo_toml_data = toml::Value::Table({
      let package = toml::Value::Table({
        let mut table = toml::Table::new();
        table.insert("name".to_string(),
                     toml::Value::String(self.config.crate_name.clone()));
        table.insert("version".to_string(),
                     toml::Value::String(self.config.crate_version.clone()));
        let mut authors: Vec<_> = self.config
          .crate_authors
          .iter()
          .map(|x| toml::Value::String(x.clone()))
          .collect();
        authors.push(toml::Value::String("cpp_to_rust generator \
                                                       (https://github.com/rust-qt/cpp_to_rust)"
          .to_string()));
        table.insert("authors".to_string(), toml::Value::Array(authors));
        table.insert("build".to_string(),
                     toml::Value::String("build.rs".to_string()));
        table
      });
      let dependencies = toml::Value::Table({
        let mut table = toml::Table::new();
        table.insert("libc".to_string(), toml::Value::String("0.2".to_string()));
        let cpp_box = toml::Value::Table({
          let mut table = toml::Table::new();
          table.insert("git".to_string(),
                       toml::Value::String("https://github.com/rust-qt/cpp_box.git".to_string()));
          table
        });
        table.insert("cpp_box".to_string(), cpp_box);
        table
      });
      let mut table = toml::Table::new();
      table.insert("package".to_string(), package);
      table.insert("dependencies".to_string(), dependencies);
      table
    });
    let mut cargo_toml_file = File::create(self.config.output_path.with_added("Cargo.toml"))
      .unwrap();
    write!(cargo_toml_file, "{}", cargo_toml_data).unwrap();

    if self.config.invokation_method == InvokationMethod::CommandLine {
      for name in &["src", "tests"] {
        let template_item_path = self.config.template_path.with_added(&name);
        if template_item_path.as_path().exists() {
          copy_recursively(&template_item_path,
                           &self.config.output_path.with_added(&name))
            .unwrap();
        }
      }
    } else {
      fs::create_dir_all(self.config.output_path.with_added("src")).unwrap();
    }
  }

  fn rust_type_to_code(&self, rust_type: &RustType) -> String {
    rust_type_to_code(rust_type, &self.config.crate_name)
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
              _ => format!(" -> {}", self.rust_type_to_code(&func.return_type)),
            })
  }

  fn generate_ffi_call(&self,
                       variant: &RustMethodArgumentsVariant,
                       shared_arguments: &Vec<RustMethodArgument>)
                       -> String {
    let mut final_args = Vec::new();
    final_args.resize(variant.cpp_method.c_signature.arguments.len(), None);
    let mut all_args = shared_arguments.clone();
    for arg in &variant.arguments {
      all_args.push(arg.clone());
    }
    for arg in &all_args {
      if let Some(ffi_index) = arg.ffi_index {
        assert!(ffi_index >= 0 && ffi_index < final_args.len() as i32);
        let mut code = arg.name.clone();
        match arg.argument_type.rust_api_to_c_conversion {
          RustToCTypeConversion::None => {}
          RustToCTypeConversion::RefToPtr => {
            code = format!("{} as {}",
                           code,
                           self.rust_type_to_code(&arg.argument_type.rust_ffi_type));

          }
          RustToCTypeConversion::ValueToPtr => {
            let is_const = if let RustType::Common { ref is_const, .. } = arg.argument_type
              .rust_ffi_type {
              *is_const
            } else {
              panic!("void is not expected here at all!")
            };
            code = format!("{}{} as {}",
                           if is_const { "&" } else { "&mut " },
                           code,
                           self.rust_type_to_code(&arg.argument_type.rust_ffi_type));
          }
          RustToCTypeConversion::QFlagsToUInt => {
            code = format!("{}.to_int() as libc::c_uint", code);
          }
        }
        final_args[ffi_index as usize] = Some(code);
      }
    }

    let mut result = Vec::new();
    let mut maybe_result_var_name = None;
    if let Some(ref i) = variant.return_type_ffi_index {
      let mut return_var_name = "object".to_string();
      let mut ii = 1;
      while variant.arguments.iter().find(|x| &x.name == &return_var_name).is_some() {
        ii += 1;
        return_var_name = format!("object{}", ii);
      }
      result.push(format!("{{\nlet mut {} = unsafe {{ {}::new_uninitialized() }};\n",
                          return_var_name,
                          self.rust_type_to_code(&variant.return_type.rust_api_type)));
      final_args[*i as usize] = Some(format!("&mut {}", return_var_name));
      maybe_result_var_name = Some(return_var_name);
    }
    for arg in &final_args {
      if arg.is_none() {
        println!("variant: {:?}", variant);
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
    match variant.return_type.rust_api_to_c_conversion {
      RustToCTypeConversion::None => {}
      RustToCTypeConversion::RefToPtr => {
        let is_const = if let RustType::Common { ref is_const, .. } = variant.return_type
          .rust_ffi_type {
          *is_const
        } else {
          panic!("void is not expected here at all!")
        };
        code = format!("let ffi_result = {};\nunsafe {{ {}*ffi_result }}",
                       code,
                       if is_const { "& " } else { "&mut " });
      }
      RustToCTypeConversion::ValueToPtr => {
        if maybe_result_var_name.is_none() {
          code = format!("let ffi_result = {};\nunsafe {{ *ffi_result }}", code);
        }
      }
      RustToCTypeConversion::QFlagsToUInt => {
        let mut qflags_type = variant.return_type.rust_api_type.clone();
        if let RustType::Common { ref mut generic_arguments, .. } = qflags_type {
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

  fn arg_texts(&self, args: &Vec<RustMethodArgument>, lifetime: Option<&String>) -> Vec<String> {
    args.iter()
      .map(|arg| {
        if &arg.name == "self" {
          let self_type = match lifetime {
            Some(lifetime) => arg.argument_type.rust_api_type.with_lifetime(lifetime.clone()),
            None => arg.argument_type.rust_api_type.clone(),
          };
          if let RustType::Common { ref indirection, ref is_const, .. } = self_type {
            let maybe_mut = if *is_const { "" } else { "mut " };
            match indirection {
              &RustTypeIndirection::None => format!("self"),
              &RustTypeIndirection::Ref { ref lifetime } => {
                match lifetime {
                  &Some(ref lifetime) => format!("&'{} {}self", lifetime, maybe_mut),
                  &None => format!("&{}self", maybe_mut),
                }
              }
              _ => panic!("invalid self argument type (indirection)"),
            }
          } else {
            panic!("invalid self argument type (not Common)");
          }
        } else {
          let mut maybe_mut_declaration = "";
          if let RustType::Common { ref indirection, .. } = arg.argument_type
            .rust_api_type {
            if *indirection == RustTypeIndirection::None &&
               arg.argument_type.rust_api_to_c_conversion == RustToCTypeConversion::ValueToPtr {
              if let RustType::Common { ref is_const, .. } = arg.argument_type
                .rust_ffi_type {
                if !is_const {
                  maybe_mut_declaration = "mut ";
                }
              }
            }
          }

          format!("{}{}: {}",
                  maybe_mut_declaration,
                  arg.name,
                  match lifetime {
                    Some(lifetime) => {
                      self.rust_type_to_code(&arg.argument_type
                        .rust_api_type
                        .with_lifetime(lifetime.clone()))
                    }
                    None => self.rust_type_to_code(&arg.argument_type.rust_api_type),
                  })
        }
      })
      .collect()
  }


  fn generate_rust_final_function(&self, func: &RustMethod) -> String {
    let maybe_pub = match func.scope {
      RustMethodScope::TraitImpl { .. } => "",
      _ => "pub ",
    };
    match func.arguments {
      RustMethodArguments::SingleVariant(ref variant) => {
        let body = self.generate_ffi_call(variant, &Vec::new());
        let return_type_for_signature = match variant.return_type.rust_api_type {
          RustType::Void => String::new(),
          _ => {
            format!(" -> {}",
                    self.rust_type_to_code(&variant.return_type.rust_api_type))
          }
        };

        format!("{doc}{maybe_pub}fn {name}({args}){return_type} {{\n{body}}}\n\n",
                doc = format_doc(&func.doc),
                maybe_pub = maybe_pub,
                name = func.name.last_name(),
                args = self.arg_texts(&variant.arguments, None).join(", "),
                return_type = return_type_for_signature,
                body = body)
      }
      RustMethodArguments::MultipleVariants { ref params_trait_name,
                                              ref params_trait_lifetime,
                                              ref shared_arguments,
                                              ref variant_argument_name } => {
        let tpl_type = variant_argument_name.to_class_case();
        let body = format!("{}.exec({})",
                           variant_argument_name,
                           shared_arguments.iter().map(|arg| arg.name.clone()).join(", "));
        let lifetime_arg = match *params_trait_lifetime {
          Some(ref lifetime) => format!("'{}, ", lifetime),
          None => format!(""),
        };
        let lifetime_specifier = match *params_trait_lifetime {
          Some(ref lifetime) => format!("<'{}>", lifetime),
          None => format!(""),
        };
        let mut args = self.arg_texts(shared_arguments, params_trait_lifetime.as_ref());
        args.push(format!("{}: {}", variant_argument_name, tpl_type));
        format!(include_str!("../templates/crate/overloaded_function.rs.in"),
                doc = format_doc(&func.doc),
                maybe_pub = maybe_pub,
                lifetime_arg = lifetime_arg,
                lifetime = lifetime_specifier,
                name = func.name.last_name(),
                trait_name = params_trait_name,
                tpl_type = tpl_type,
                args = args.join(", "),
                body = body)
      }
    }
  }

  pub fn generate_lib_file(&self, modules: &Vec<&String>) {
    let mut lib_file_path = self.config.output_path.clone();
    lib_file_path.push("src");
    lib_file_path.push("lib.rs");
    if lib_file_path.as_path().exists() {
      fs::remove_file(&lib_file_path).unwrap();
    }
    {
      let mut lib_file = File::create(&lib_file_path).unwrap();
      if self.config.invokation_method == InvokationMethod::CommandLine {
        write!(lib_file, "#![allow(drop_with_repr_extern)]\n\n").unwrap();
      }
      write!(lib_file, "pub extern crate libc;\n").unwrap();
      write!(lib_file, "pub extern crate cpp_box;\n\n").unwrap();

      let mut extra_modules = vec!["ffi".to_string()];
      if self.config.invokation_method == InvokationMethod::CommandLine {
        for item in fs::read_dir(&self.config.template_path.with_added("src")).unwrap() {
          let item = item.unwrap();
          if item.file_name().to_str().unwrap() == "lib.rs" {
            continue;
          }
          if item.path().is_dir() {
            extra_modules.push(item.file_name().into_string().unwrap());
          } else if item.path().extension().is_some() && item.path().extension().unwrap() == "rs" {
            extra_modules.push(item.path().file_stem().unwrap().to_str().unwrap().to_string());
          }
        }
      }
      for module in &extra_modules {
        if modules.iter().find(|x| x.as_ref() as &str == module).is_some() {
          panic!("module name conflict");
        }
      }

      let all_modules = extra_modules.iter().chain(modules.clone());
      for module in all_modules {
        if module == &"ffi" {
          // TODO: remove allow directive for ffi?
          // TODO: ffi should be a private mod
          write!(lib_file, "#[allow(dead_code)]\n").unwrap();
        }
        match self.config.invokation_method {
          InvokationMethod::CommandLine => write!(lib_file, "pub mod {};\n", module).unwrap(),
          InvokationMethod::BuildScript => {
            write!(lib_file,
                   "pub mod {0} {{ \n  include!(\"{0}.rs\");\n}}\n",
                   module)
              .unwrap()
          }
        }
      }
    }
    self.call_rustfmt(&lib_file_path);
  }

  fn generate_module_code(&self, data: &RustModule) -> String {
    let mut results = Vec::new();
    results.push("#[allow(unused_imports)]\nuse {libc, cpp_box, std};\n\n".to_string());

    for type1 in &data.types {
      results.push(format_doc(&type1.doc));
      match type1.kind {
        RustTypeDeclarationKind::CppTypeWrapper { ref kind, ref methods, ref traits, .. } => {
          let r = match *kind {
            RustTypeWrapperKind::Enum { ref values, ref is_flaggable } => {
              let mut r =
                format!(include_str!("../templates/crate/enum_declaration.rs.in"),
                        name = type1.name,
                        variants = values.iter()
                          .map(|item| {
                            format!("{}  {} = {}", format_doc(&item.doc), item.name, item.value)
                          })
                          .join(", \n"));
              if *is_flaggable {
                r = r +
                    &format!(include_str!("../templates/crate/impl_flaggable.rs.in"),
                             name = type1.name);
              }
              r
            }
            RustTypeWrapperKind::Struct { ref size } => {
              format!(include_str!("../templates/crate/struct_declaration.rs.in"),
                      name = type1.name,
                      size = size)
            }
          };
          results.push(r);
          if !methods.is_empty() {
            results.push(format!("impl {} {{\n{}}}\n\n",
                                 type1.name,
                                 methods.iter()
                                   .map(|method| self.generate_rust_final_function(method))
                                   .join("")));
          }
          for trait1 in traits {
            let trait_content = match trait1.trait_name {
              TraitName::CppDeletable { ref deleter_name } => {
                format!("fn deleter() -> cpp_box::Deleter<Self> {{\n  ::ffi::{}\n}}\n",
                        deleter_name)
              }
              _ => {
                trait1.methods
                  .iter()
                  .map(|method| self.generate_rust_final_function(method))
                  .join("")
              }
            };

            results.push(format!("impl {} for {} {{\n{}}}\n\n",
                                 trait1.trait_name.to_string(),
                                 type1.name,
                                 trait_content));
          }
        }
        RustTypeDeclarationKind::MethodParametersTrait { ref shared_arguments,
                                                         ref impls,
                                                         ref lifetime } => {
          let arg_list = self.arg_texts(shared_arguments, lifetime.as_ref()).join(", ");
          let trait_lifetime_specifier = match *lifetime {
            Some(ref lf) => format!("<'{}>", lf),
            None => format!(""),
          };
          results.push(format!("pub trait {name}{trait_lifetime_specifier} {{
              type ReturnType;
              fn exec(self, {arg_list}) -> Self::ReturnType;
            }}",
                               name = type1.name,
                               arg_list = arg_list,
                               trait_lifetime_specifier = trait_lifetime_specifier));
          for variant in impls {
            let mut final_lifetime = lifetime.clone();
            if lifetime.is_none() &&
               (variant.arguments
              .iter()
              .find(|t| {
                t.argument_type
                  .rust_api_type
                  .is_ref()
              })
              .is_some() || variant.return_type.rust_api_type.is_ref()) {
              final_lifetime = Some("a".to_string());
            }
            let lifetime_specifier = match final_lifetime {
              Some(ref lf) => format!("<'{}>", lf),
              None => format!(""),
            };
            let final_arg_list = self.arg_texts(shared_arguments, final_lifetime.as_ref())
              .join(", ");
            let tuple_item_types: Vec<_> = variant.arguments
              .iter()
              .map(|t| {
                match final_lifetime {
                  Some(ref lifetime) => {
                    self.rust_type_to_code(&t.argument_type
                      .rust_api_type
                      .with_lifetime(lifetime.to_string()))
                  }
                  None => {
                    self.rust_type_to_code(&t.argument_type
                      .rust_api_type)
                  }
                }
              })
              .collect();
            let mut tmp_vars = Vec::new();
            if variant.arguments.len() == 1 {
              if variant.arguments[0].ffi_index.is_some() {
                tmp_vars.push(format!("let {} = self;", variant.arguments[0].name));
              }
            } else {
              for (index, arg) in variant.arguments.iter().enumerate() {
                if arg.ffi_index.is_some() {
                  tmp_vars.push(format!("let {} = self.{};", arg.name, index));
                }
              }
            }

            results.push(format!(include_str!("../templates/crate/impl_overloading_trait.rs.in"),
                                 lifetime_specifier = lifetime_specifier,
                                 trait_lifetime_specifier = trait_lifetime_specifier,
                                 trait_name = type1.name,
                                 final_arg_list = final_arg_list,
                                 impl_type = if tuple_item_types.len() == 1 {
                                   tuple_item_types[0].clone()
                                 } else {
                                   format!("({})", tuple_item_types.join(","))
                                 },
                                 return_type = match final_lifetime {
                                   Some(ref lifetime) => {
                                     self.rust_type_to_code(&variant.return_type
                                       .rust_api_type
                                       .with_lifetime(lifetime.to_string()))
                                   }
                                   None => {
                                     self.rust_type_to_code(&variant.return_type
                                       .rust_api_type)
                                   }
                                 },
                                 tmp_vars = tmp_vars.join("\n"),
                                 body = self.generate_ffi_call(variant, shared_arguments)));

          }
        }
      };
    }
    for method in &data.functions {
      results.push(self.generate_rust_final_function(method));
    }

    for submodule in &data.submodules {
      results.push(format!("pub mod {} {{\n{}}}\n\n",
                           submodule.name,
                           self.generate_module_code(submodule)));
    }
    return results.join("");
  }

  fn call_rustfmt(&self, path: &PathBuf) {
    let result = panic::catch_unwind(|| {
      rustfmt::run(rustfmt::Input::File(path.clone()), &self.rustfmt_config)
    });
    match result {
      Ok(rustfmt_result) => {
        if !rustfmt_result.has_no_errors() {
          log::warning(format!("rustfmt failed to format file: {:?}", path));
        }
      }
      Err(cause) => {
        log::warning(format!("rustfmt failed to format file: {:?} (panic: {:?})",
                             path,
                             cause));
      }
    }
    assert!(path.as_path().is_file());
  }

  pub fn generate_module_file(&self, data: &RustModule) {
    let mut file_path = self.config.output_path.clone();
    file_path.push("src");
    file_path.push(format!("{}.rs", &data.name));
    {
      let mut file = File::create(&file_path).unwrap();
      file.write(self.generate_module_code(data).as_bytes()).unwrap();
    }
    self.call_rustfmt(&file_path);

  }

  pub fn generate_ffi_file(&self, functions: &Vec<(String, Vec<RustFFIFunction>)>) {
    let mut file_path = self.config.output_path.clone();
    file_path.push("src");
    file_path.push("ffi.rs");
    {
      let mut file = File::create(&file_path).unwrap();
      write!(file, "use libc;\n\n").unwrap();

      write!(file, "#[link(name = \"{}\")]\n", &self.config.cpp_lib_name).unwrap();
      write!(file, "#[link(name = \"stdc++\")]\n").unwrap();
      write!(file,
             "#[link(name = \"{}\", kind = \"static\")]\n",
             &self.config.c_lib_name)
        .unwrap();
      write!(file, "extern \"C\" {{\n").unwrap();

      for &(ref include_file, ref functions) in functions {
        write!(file, "  // Header: {}\n", include_file).unwrap();
        for function in functions {
          file.write(self.rust_ffi_function_to_code(function).as_bytes()).unwrap();
        }
        write!(file, "\n").unwrap();
      }
      write!(file, "}}\n").unwrap();
    }
    // no rustfmt for ffi file
  }
}
