use errors::{Result, ChainErr, unexpected};
use file_utils::{PathBufWithAdded, copy_recursively, file_to_string, copy_file, create_file,
                 path_to_str, create_dir_all, remove_file, read_dir, os_str_to_str,
                 os_string_into_string};
use log;
use rust_generator::RustGeneratorOutput;
use rust_info::{RustTypeDeclarationKind, RustTypeWrapperKind, RustModule, RustMethod,
                RustMethodArguments, RustMethodArgumentsVariant, RustMethodScope,
                RustMethodArgument, TraitImpl, TraitImplExtra, RustQtReceiverType};
use rust_type::{RustName, RustType, RustTypeIndirection, RustFFIFunction, RustToCTypeConversion,
                CompleteType};
use string_utils::{JoinWithString, CaseOperations};
use utils::{is_msvc, MapIfOk};
use doc_formatter;
use std::path::PathBuf;

extern crate rustfmt;
extern crate toml;

pub struct RustCodeGeneratorDependency {
  pub crate_name: String,
  pub crate_path: PathBuf,
}

pub enum RustLinkKind {
  SharedLibrary,
  Framework,
}
pub struct RustLinkItem {
  pub name: String,
  pub kind: RustLinkKind,
}

pub struct RustCodeGeneratorConfig {
  pub crate_name: String,
  pub crate_version: String,
  pub crate_authors: Vec<String>,
  pub output_path: PathBuf,
  pub final_output_path: PathBuf,
  pub template_path: PathBuf,
  pub c_lib_name: String,
  pub c_lib_is_shared: bool,
  pub link_items: Vec<RustLinkItem>,
  pub framework_dirs: Vec<String>,
  pub rustfmt_config_path: Option<PathBuf>,
  pub dependencies: Vec<RustCodeGeneratorDependency>,
}

fn format_doc(doc: &str) -> String {
  if doc.is_empty() {
    return String::new();
  }
  doc.split('\n')
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

pub fn rust_type_to_code(rust_type: &RustType, crate_name: &str) -> String {
  match *rust_type {
    RustType::Void => "()".to_string(),
    RustType::Common { ref base,
                       ref is_const,
                       ref is_const2,
                       ref indirection,
                       ref generic_arguments,
                       .. } => {
      let mut base_s = base.full_name(Some(crate_name));
      if let Some(ref args) = *generic_arguments {
        base_s = format!("{}<{}>",
                         base_s,
                         args.iter().map(|x| rust_type_to_code(x, crate_name)).join(", "));
      }
      match *indirection {
        RustTypeIndirection::None => base_s,
        RustTypeIndirection::Ref { ref lifetime } => {
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
        RustTypeIndirection::Ptr => {
          if *is_const {
            format!("*const {}", base_s)
          } else {
            format!("*mut {}", base_s)
          }
        }
        RustTypeIndirection::PtrPtr => {
          let const_text1 = if *is_const { "*const " } else { "*mut " };
          let const_text2 = if *is_const2 { "*const " } else { "*mut " };
          format!("{}{}{}", const_text2, const_text1, base_s)
        }
        RustTypeIndirection::PtrRef { ref lifetime } => {
          let const_text1 = if *is_const { "*const " } else { "*mut " };
          let lifetime_text = match *lifetime {
            Some(ref lifetime) => format!("'{} ", lifetime),
            None => String::new(),
          };
          let const_text2 = if *is_const2 {
            format!("&{}", lifetime_text)
          } else {
            format!("&{}mut ", lifetime_text)
          };
          format!("{}{}{}", const_text2, const_text1, base_s)
        }
      }
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


pub fn run(config: RustCodeGeneratorConfig, data: &RustGeneratorOutput) -> Result<()> {
  let rustfmt_config_data = match config.rustfmt_config_path {
    Some(ref path) => {
      log::info(format!("Using rustfmt config file: {:?}", path));
      try!(file_to_string(path))
    }
    None => include_str!("../templates/crate/rustfmt.toml").to_string(),
  };
  let rustfmt_config = rustfmt::config::Config::from_toml(&rustfmt_config_data);
  let generator = RustCodeGenerator {
    config: config,
    rustfmt_config: rustfmt_config,
  };
  try!(generator.generate_template());
  for module in &data.modules {
    try!(generator.generate_module_file(module));
  }
  let mut module_names: Vec<_> = data.modules.iter().map(|x| &x.name).collect();
  module_names.sort();
  try!(generator.generate_ffi_file(&data.ffi_functions));
  try!(generator.generate_lib_file(&module_names));
  Ok(())
}

pub struct RustCodeGenerator {
  config: RustCodeGeneratorConfig,
  pub rustfmt_config: rustfmt::config::Config,
}

impl RustCodeGenerator {
  /// Generates cargo file and skeleton of the crate
  pub fn generate_template(&self) -> Result<()> {
    if let Some(ref path) = self.config.rustfmt_config_path {
      try!(copy_file(path, self.config.output_path.with_added("rustfmt.toml")));
    } else {
      let mut rustfmt_file = try!(create_file(self.config.output_path.with_added("rustfmt.toml")));
      try!(rustfmt_file.write(include_str!("../templates/crate/rustfmt.toml")));
    };

    {
      let mut build_rs_file = try!(create_file(self.config.output_path.with_added("build.rs")));
      let extra = self.config
        .framework_dirs
        .iter()
        .map(|x| {
          format!("  println!(\"cargo:rustc-link-search=framework={{}}\", \"{}\");",
                  x)
        })
        .join("\n");
      try!(build_rs_file.write(format!(include_str!("../templates/crate/build.rs"), extra = extra)));
    }

    let cargo_toml_data = toml::Value::Table({
      let package = toml::Value::Table({
        let mut table = toml::Table::new();
        table.insert("name".to_string(),
                     toml::Value::String(self.config.crate_name.clone()));
        table.insert("version".to_string(),
                     toml::Value::String(self.config.crate_version.clone()));
        let authors = self.config
          .crate_authors
          .iter()
          .map(|x| toml::Value::String(x.clone()))
          .collect();
        table.insert("authors".to_string(), toml::Value::Array(authors));
        table.insert("build".to_string(),
                     toml::Value::String("build.rs".to_string()));
        table
      });
      let dependencies = toml::Value::Table({
        let mut table = toml::Table::new();
        table.insert("libc".to_string(), toml::Value::String("0.2".to_string()));
        table.insert("cpp_utils".to_string(),
                     toml::Value::String("0.1".to_string()));
        for dep in &self.config.dependencies {
          let mut table_dep = toml::Table::new();
          table_dep.insert("path".to_string(),
                           toml::Value::String(try!(path_to_str(&dep.crate_path)).to_string()));
          table.insert(dep.crate_name.clone(), toml::Value::Table(table_dep));
        }
        table
      });
      let mut table = toml::Table::new();
      table.insert("package".to_string(), package);
      table.insert("dependencies".to_string(), dependencies);
      if is_msvc() {
        // LNK1189 (too many members) in MSVC with static linking,
        // so we use dynamic linking
        let lib = toml::Value::Table({
          let mut table = toml::Table::new();
          table.insert("crate-type".to_string(),
                       toml::Value::Array(vec![toml::Value::String("dylib".to_string())]));
          table
        });
        table.insert("lib".to_string(), lib);
      }
      table
    });
    let mut cargo_toml_file = try!(create_file(self.config.output_path.with_added("Cargo.toml")));
    try!(cargo_toml_file.write(cargo_toml_data.to_string()));

    for name in &["src", "tests"] {
      let template_item_path = self.config.template_path.with_added(&name);
      if template_item_path.as_path().exists() {
        try!(copy_recursively(&template_item_path,
                              &self.config.output_path.with_added(&name)));
      }
    }
    if !self.config.output_path.with_added("src").exists() {
      try!(create_dir_all(self.config.output_path.with_added("src")));
    }
    Ok(())
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

  fn convert_type_from_ffi(&self,
                           type1: &CompleteType,
                           expression: String,
                           in_unsafe_context: bool,
                           use_ffi_result_var: bool)
                           -> Result<String> {
    let (unsafe_start, unsafe_end) = if in_unsafe_context {
      ("", "")
    } else {
      ("unsafe { ", " }")
    };
    if type1.rust_api_to_c_conversion == RustToCTypeConversion::None {
      return Ok(expression);
    }

    let (code1, source_expr) = if use_ffi_result_var {
      (format!("let ffi_result = {};\n", expression), "ffi_result".to_string())
    } else {
      (String::new(), expression)
    };
    let code2 = match type1.rust_api_to_c_conversion {
      RustToCTypeConversion::None => unreachable!(),
      RustToCTypeConversion::RefToPtr |
      RustToCTypeConversion::OptionRefToPtr => {
        let api_is_const = if type1.rust_api_to_c_conversion ==
                              RustToCTypeConversion::OptionRefToPtr {
          if let RustType::Common { ref generic_arguments, .. } = type1.rust_api_type {
            let args = try!(generic_arguments.as_ref()
              .chain_err(|| "Option with no generic_arguments"));
            if args.len() != 1 {
              return Err("Option with invalid args count".into());
            }
            try!(args[0].last_is_const())
          } else {
            return Err("Option type expected".into());
          }
        } else {
          try!(type1.rust_api_type.last_is_const())
        };
        let unwrap_code = match type1.rust_api_to_c_conversion {
          RustToCTypeConversion::RefToPtr => {
            ".expect(\"Attempted to convert null pointer to reference\")"
          }
          RustToCTypeConversion::OptionRefToPtr => "",
          _ => unreachable!(),
        };
        format!("{unsafe_start}{}.{}(){unsafe_end}{}",
                source_expr,
                if api_is_const { "as_ref" } else { "as_mut" },
                unwrap_code,
                unsafe_start = unsafe_start,
                unsafe_end = unsafe_end)
      }
      RustToCTypeConversion::ValueToPtr => {
        format!("{unsafe_start}*{}{unsafe_end}",
                source_expr,
                unsafe_start = unsafe_start,
                unsafe_end = unsafe_end)
      }
      RustToCTypeConversion::CppBoxToPtr => {
        format!("{unsafe_start}::cpp_utils::CppBox::new({}){unsafe_end}",
                source_expr,
                unsafe_start = unsafe_start,
                unsafe_end = unsafe_end)
      }
      RustToCTypeConversion::QFlagsToUInt => {
        let mut qflags_type = type1.rust_api_type.clone();
        if let RustType::Common { ref mut generic_arguments, .. } = qflags_type {
          *generic_arguments = None;
        } else {
          unreachable!();
        }
        format!("{}::from_int({} as i32)",
                self.rust_type_to_code(&qflags_type),
                source_expr)
      }
    };
    Ok(code1 + &code2)
  }

  fn generate_ffi_call(&self,
                       variant: &RustMethodArgumentsVariant,
                       shared_arguments: &[RustMethodArgument],
                       in_unsafe_context: bool)
                       -> Result<String> {
    let (unsafe_start, unsafe_end) = if in_unsafe_context {
      ("", "")
    } else {
      ("unsafe { ", " }")
    };
    let mut final_args = Vec::new();
    final_args.resize(variant.cpp_method.c_signature.arguments.len(), None);
    let mut all_args: Vec<RustMethodArgument> = Vec::from(shared_arguments);
    for arg in &variant.arguments {
      all_args.push(arg.clone());
    }
    for arg in &all_args {
      if let Some(ffi_index) = arg.ffi_index {
        assert!(ffi_index >= 0 && ffi_index < final_args.len() as i32);
        let mut code = arg.name.clone();
        match arg.argument_type.rust_api_to_c_conversion {
          RustToCTypeConversion::None => {}
          RustToCTypeConversion::OptionRefToPtr => {
            return Err("OptionRefToPtr is not supported here yet".into());
          }
          RustToCTypeConversion::RefToPtr => {
            if try!(arg.argument_type.rust_api_type.is_const()) &&
               !try!(arg.argument_type.rust_ffi_type.is_const()) {
              let mut intermediate_type = arg.argument_type.rust_ffi_type.clone();
              try!(intermediate_type.set_const(true));
              code = format!("{} as {} as {}",
                             code,
                             self.rust_type_to_code(&intermediate_type),
                             self.rust_type_to_code(&arg.argument_type.rust_ffi_type));

            } else {
              code = format!("{} as {}",
                             code,
                             self.rust_type_to_code(&arg.argument_type.rust_ffi_type));
            }
          }
          RustToCTypeConversion::ValueToPtr |
          RustToCTypeConversion::CppBoxToPtr => {
            let is_const =
              if let RustType::Common { ref is_const, ref is_const2, ref indirection, .. } =
                     arg.argument_type
                .rust_ffi_type {
                match *indirection {
                  RustTypeIndirection::PtrPtr { .. } |
                  RustTypeIndirection::PtrRef { .. } => *is_const2,
                  _ => *is_const,
                }
              } else {
                return Err(unexpected("void is not expected here at all!").into());
              };
            if arg.argument_type.rust_api_to_c_conversion == RustToCTypeConversion::CppBoxToPtr {
              let method = if is_const { "as_ptr" } else { "as_mut_ptr" };
              code = format!("{}.{}()", code, method);
            } else {
              code = format!("{}{} as {}",
                             if is_const { "&" } else { "&mut " },
                             code,
                             self.rust_type_to_code(&arg.argument_type.rust_ffi_type));
            }
          }
          RustToCTypeConversion::QFlagsToUInt => {
            code = format!("{}.to_int() as ::libc::c_uint", code);
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
      while variant.arguments.iter().any(|x| &x.name == &return_var_name) {
        ii += 1;
        return_var_name = format!("object{}", ii);
      }
      let struct_name = if variant.return_type.rust_api_to_c_conversion ==
                           RustToCTypeConversion::CppBoxToPtr {
        if let RustType::Common { ref generic_arguments, .. } = variant.return_type.rust_api_type {
          let generic_arguments = try!(generic_arguments.as_ref()
            .chain_err(|| "CppBox must have generic_arguments"));
          let arg = try!(generic_arguments.get(0)
            .chain_err(|| "CppBox must have non-empty generic_arguments"));
          self.rust_type_to_code(arg)
        } else {
          return Err(unexpected("CppBox type expected").into());
        }
      } else {
        self.rust_type_to_code(&variant.return_type.rust_api_type)
      };
      result.push(format!("{{\nlet mut {var}: {t} = {unsafe_start}\
                           ::cpp_utils::new_uninitialized::NewUninitialized::new_uninitialized()\
                           {unsafe_end};\n",
                          var = return_var_name,
                          t = struct_name,
                          unsafe_start = unsafe_start,
                          unsafe_end = unsafe_end));
      final_args[*i as usize] = Some(format!("&mut {}", return_var_name));
      maybe_result_var_name = Some(return_var_name);
    }
    let final_args = try!(final_args.into_iter()
      .map_if_ok(|x| x.chain_err(|| "ffi argument is missing")));

    result.push(format!("{unsafe_start}::ffi::{}({}){maybe_semicolon}{unsafe_end}",
                        variant.cpp_method.c_name,
                        final_args.join(", "),
                        maybe_semicolon = if maybe_result_var_name.is_some() {
                          ";"
                        } else {
                          ""
                        },
                        unsafe_start = unsafe_start,
                        unsafe_end = unsafe_end));
    if let Some(ref name) = maybe_result_var_name {
      result.push(format!("{}\n}}", name));
    }
    let code = result.join("");
    if maybe_result_var_name.is_none() {
      self.convert_type_from_ffi(&variant.return_type, code, in_unsafe_context, true)
    } else {
      Ok(code)
    }
  }

  fn arg_texts(&self, args: &[RustMethodArgument], lifetime: Option<&String>) -> Vec<String> {
    args.iter()
      .map(|arg| {
        if &arg.name == "self" {
          let self_type = match lifetime {
            Some(lifetime) => arg.argument_type.rust_api_type.with_lifetime(lifetime.clone()),
            None => arg.argument_type.rust_api_type.clone(),
          };
          if let RustType::Common { ref indirection, ref is_const, .. } = self_type {
            let maybe_mut = if *is_const { "" } else { "mut " };
            match *indirection {
              RustTypeIndirection::None => "self".to_string(),
              RustTypeIndirection::Ref { ref lifetime } => {
                match *lifetime {
                  Some(ref lifetime) => format!("&'{} {}self", lifetime, maybe_mut),
                  None => format!("&{}self", maybe_mut),
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


  fn generate_rust_final_function(&self, func: &RustMethod) -> Result<String> {
    let maybe_pub = match func.scope {
      RustMethodScope::TraitImpl => "",
      _ => "pub ",
    };
    let maybe_unsafe = if func.is_unsafe { "unsafe " } else { "" };
    Ok(match func.arguments {
      RustMethodArguments::SingleVariant(ref variant) => {
        let body = try!(self.generate_ffi_call(variant, &Vec::new(), func.is_unsafe));
        let return_type_for_signature = if variant.return_type.rust_api_type == RustType::Void {
          String::new()
        } else {
          format!(" -> {}",
                  self.rust_type_to_code(&variant.return_type.rust_api_type))
        };
        let all_lifetimes: Vec<_> = variant.arguments
          .iter()
          .filter_map(|x| x.argument_type.rust_api_type.lifetime())
          .collect();
        let lifetimes_text = if all_lifetimes.is_empty() {
          String::new()
        } else {
          format!("<{}>",
                  all_lifetimes.iter().map(|x| format!("'{}", x)).join(", "))
        };

        format!("{doc}{maybe_pub}{maybe_unsafe}fn {name}{lifetimes_text}({args}){return_type} \
                 {{\n{body}}}\n\n",
                doc = format_doc(&doc_formatter::method_doc(&func)),
                maybe_pub = maybe_pub,
                maybe_unsafe = maybe_unsafe,
                lifetimes_text = lifetimes_text,
                name = try!(func.name.last_name()),
                args = self.arg_texts(&variant.arguments, None).join(", "),
                return_type = return_type_for_signature,
                body = body)
      }
      RustMethodArguments::MultipleVariants { ref params_trait_name,
                                              ref params_trait_lifetime,
                                              ref params_trait_return_type,
                                              ref shared_arguments,
                                              ref variant_argument_name,
                                              .. } => {
        let tpl_type = variant_argument_name.to_class_case();
        let body = format!("{}.exec({})",
                           variant_argument_name,
                           shared_arguments.iter().map(|arg| arg.name.clone()).join(", "));
        let mut all_lifetimes: Vec<_> = shared_arguments.iter()
          .filter_map(|x| x.argument_type.rust_api_type.lifetime())
          .collect();
        if let Some(ref params_trait_lifetime) = *params_trait_lifetime {
          if !all_lifetimes.iter().any(|x| x == &params_trait_lifetime) {
            all_lifetimes.push(params_trait_lifetime);
          }
        }
        let mut tpl_decl_texts: Vec<_> = all_lifetimes.iter().map(|x| format!("'{}", x)).collect();
        tpl_decl_texts.push(tpl_type.clone());
        let tpl_decl = tpl_decl_texts.join(", ");
        let trait_lifetime_arg = match *params_trait_lifetime {
          Some(ref lifetime) => format!("<'{}>", lifetime),
          None => String::new(),
        };
        let mut args = self.arg_texts(shared_arguments, None);
        args.push(format!("{}: {}", variant_argument_name, tpl_type));
        let return_type_string = if let Some(ref t) = *params_trait_return_type {
          self.rust_type_to_code(t)
        } else {
          format!("{}::ReturnType", tpl_type)
        };
        format!(include_str!("../templates/crate/overloaded_function.rs.in"),
                doc = format_doc(&doc_formatter::method_doc(&func)),
                maybe_pub = maybe_pub,
                tpl_decl = tpl_decl,
                trait_lifetime_arg = trait_lifetime_arg,
                name = try!(func.name.last_name()),
                trait_name = params_trait_name,
                tpl_type = tpl_type,
                args = args.join(", "),
                body = body,
                return_type_string = return_type_string)
      }
    })
  }

  #[cfg_attr(feature="clippy", allow(collapsible_if))]
  pub fn generate_lib_file(&self, modules: &[&String]) -> Result<()> {
    let src_path = self.config.output_path.with_added("src");
    let lib_file_path = src_path.with_added("lib.rs");
    if lib_file_path.as_path().exists() {
      try!(remove_file(&lib_file_path));
    }
    #[derive(PartialEq, Eq)]
    enum Mode {
      LibInRs,
      LibRs,
    }
    for mode in &[Mode::LibInRs, Mode::LibRs] {
      let lib_file_path = match *mode {
        Mode::LibInRs => self.config.output_path.with_added("lib.in.rs"),
        Mode::LibRs => src_path.with_added("lib.rs"),
      };
      {
        let mut lib_file = try!(create_file(&lib_file_path));
        try!(lib_file.write("pub extern crate libc;\n"));
        try!(lib_file.write("pub extern crate cpp_utils;\n\n"));
        for dep in &self.config.dependencies {
          try!(lib_file.write(format!("pub extern crate {};\n\n", &dep.crate_name)));
        }
        let mut extra_modules = vec!["ffi".to_string()];

        if mode == &Mode::LibRs {
          if self.config.template_path.with_added("src").exists() {
            for item in try!(read_dir(&self.config.template_path.with_added("src"))) {
              let item = try!(item);
              let path = item.path();
              let file_name = try!(os_string_into_string(item.file_name()));
              if file_name == "lib.rs" {
                continue;
              }
              if item.path().is_dir() {
                extra_modules.push(file_name.to_string());
              } else if let Some(ext) = item.path().extension() {
                if ext == "rs" {
                  let stem = try!(path.file_stem().chain_err(|| "file_stem() failed for .rs file"));
                  extra_modules.push(try!(os_str_to_str(stem)).to_string());
                }
              }
            }
          }
        }
        for module in &extra_modules {
          if modules.iter().any(|x| x.as_ref() as &str == module) {
            panic!("module name conflict");
          }
        }
        let all_modules = extra_modules.iter().chain(modules.iter().map(|x| *x));
        for module in all_modules {
          let mut maybe_pub = "pub ";
          if module == "ffi" {
            maybe_pub = "";
            // some ffi functions are not used because
            // some Rust methods are filtered
            try!(lib_file.write("#[allow(dead_code)]\n"));
          }
          match *mode {
            Mode::LibInRs => {
              let mut file_path = self.config.final_output_path.clone();
              file_path.push("src");
              file_path.push(format!("{}.rs", module));
              try!(lib_file.write(format!("{maybe_pub}mod {name} {{ \n  \
                                           include!(\"{path}\");\n}}\n",
                                          path = try!(path_to_str(&file_path)).replace("\\", "\\\\"),
                                          name = module,
                                          maybe_pub = maybe_pub)))
            }
            Mode::LibRs => try!(lib_file.write(format!("{}mod {};\n", maybe_pub, module))),
          }
        }
      }
      self.call_rustfmt(&lib_file_path);
    }
    Ok(())
  }

  fn generate_trait_impls(&self, trait_impls: &[TraitImpl]) -> Result<String> {
    let mut results = Vec::new();
    for trait1 in trait_impls {
      let trait_content = if let Some(TraitImplExtra::CppDeletable { ref deleter_name }) =
                                 trait1.extra {
        format!("fn deleter() -> ::cpp_utils::Deleter<Self> {{\n  ::ffi::{}\n}}\n",
                deleter_name)
      } else {
        try!(trait1.methods
            .iter()
            .map_if_ok(|method| self.generate_rust_final_function(method)))
          .join("")
      };
      results.push(format!("impl {} for {} {{\n{}}}\n\n",
                           self.rust_type_to_code(&trait1.trait_type),
                           self.rust_type_to_code(&trait1.target_type),
                           trait_content));
    }
    Ok(results.join(""))
  }

  #[cfg_attr(feature="clippy", allow(single_match_else))]
  fn generate_module_code(&self, data: &RustModule) -> Result<String> {
    let mut results = Vec::new();
    for type1 in &data.types {
      results.push(format_doc(&doc_formatter::type_doc(type1)));
      let maybe_pub = if type1.is_public { "pub " } else { "" };
      match type1.kind {
        RustTypeDeclarationKind::CppTypeWrapper { ref kind,
                                                  ref methods,
                                                  ref trait_impls,
                                                  ref qt_receivers,
                                                  .. } => {
          let r = match *kind {
            RustTypeWrapperKind::Enum { ref values, ref is_flaggable } => {
              let mut r = format!(include_str!("../templates/crate/enum_declaration.rs.in"),
                                  maybe_pub = maybe_pub,
                                  name = try!(type1.name.last_name()),
                                  variants = values.iter()
                                    .map(|item| {
                                      format!("{}  {} = {}",
                                              format_doc(&doc_formatter::enum_value_doc(&item)),
                                              item.name,
                                              item.value)
                                    })
                                    .join(", \n"));
              if *is_flaggable {
                r = r +
                    &format!(include_str!("../templates/crate/impl_flaggable.rs.in"),
                             name = try!(type1.name.last_name()),
                             trait_type = try!(RustName::new(vec!["qt_core".to_string(),
                                                                  "flags".to_string(),
                                                                  "FlaggableEnum".to_string()]))
                               .full_name(Some(&self.config.crate_name)));
              }
              r
            }
            RustTypeWrapperKind::Struct { ref size, .. } => {
              format!(include_str!("../templates/crate/struct_declaration.rs.in"),
                      maybe_pub = maybe_pub,
                      name = try!(type1.name.last_name()),
                      size = size)
            }
            RustTypeWrapperKind::EmptyEnum { ref slot_wrapper, .. } => {
              let mut r = format!("{maybe_pub}enum {} {{}}\n\n",
                                  try!(type1.name.last_name()),
                                  maybe_pub = maybe_pub);
              if let Some(ref slot_wrapper) = *slot_wrapper {
                let arg_texts: Vec<_> = slot_wrapper.arguments
                  .iter()
                  .map(|t| self.rust_type_to_code(&t.rust_api_type))
                  .collect();
                let args = arg_texts.join(", ");
                let args_tuple = format!("{}{}", args, if arg_texts.len() == 1 { "," } else { "" });
                let connections_mod = try!(RustName::new(vec!["qt_core".to_string(),
                                                              "connections".to_string()]))
                  .full_name(Some(&self.config.crate_name));
                let object_type_name = try!(RustName::new(vec!["qt_core".to_string(),
                                                               "object".to_string(),
                                                               "Object".to_string()]))
                  .full_name(Some(&self.config.crate_name));
                let callback_args = slot_wrapper.arguments
                  .iter()
                  .enumerate()
                  .map(|(num, t)| {
                    format!("arg{}: {}", num, self.rust_type_to_code(&t.rust_ffi_type))
                  })
                  .join(", ");
                let func_args = try!(slot_wrapper.arguments
                    .iter()
                    .enumerate()
                    .map_if_ok(|(num, t)| {
                      self.convert_type_from_ffi(t, format!("arg{}", num), false, false)
                    }))
                  .join(", ");
                r.push_str(&format!(include_str!("../templates/crate/slot_wrapper_extras.rs.in"),
                                    type_name = type1.name
                                      .full_name(Some(&self.config.crate_name)),
                                    pub_type_name = slot_wrapper.public_type_name,
                                    callback_name = slot_wrapper.callback_name,
                                    args = args,
                                    args_tuple = args_tuple,
                                    receiver_id = slot_wrapper.receiver_id,
                                    connections_mod = connections_mod,
                                    object_type_name = object_type_name,
                                    func_args = func_args,
                                    callback_args = callback_args));
              }
              r
            }
          };
          results.push(r);
          if !methods.is_empty() {
            results.push(format!("impl {} {{\n{}}}\n\n",
                                 try!(type1.name.last_name()),
                                 try!(methods.iter()
                                   .map_if_ok(|method| self.generate_rust_final_function(method)))
                                   .join("")));
          }
          results.push(try!(self.generate_trait_impls(trait_impls)));
          if !qt_receivers.is_empty() {
            let connections_mod = try!(RustName::new(vec!["qt_core".to_string(),
                                                          "connections".to_string()]))
              .full_name(Some(&self.config.crate_name));
            let object_type_name = try!(RustName::new(vec!["qt_core".to_string(),
                                                           "object".to_string(),
                                                           "Object".to_string()]))
              .full_name(Some(&self.config.crate_name));
            let mut content = Vec::new();
            let obj_name = type1.name.full_name(Some(&self.config.crate_name));
            content.push("use ::cpp_utils::StaticCast;\n".to_string());
            let mut type_impl_content = Vec::new();
            for receiver_type in &[RustQtReceiverType::Signal, RustQtReceiverType::Slot] {
              if qt_receivers.iter().any(|r| &r.receiver_type == receiver_type) {
                let (struct_method, struct_type) = match *receiver_type {
                  RustQtReceiverType::Signal => ("signals", "Signals"),
                  RustQtReceiverType::Slot => ("slots", "Slots"),
                };
                let mut struct_content = Vec::new();
                content.push(format!("pub struct {}<'a>(&'a {});\n", struct_type, obj_name));
                for receiver in qt_receivers {
                  if &receiver.receiver_type == receiver_type {
                    let arg_texts: Vec<_> = receiver.arguments
                      .iter()
                      .map(|t| self.rust_type_to_code(t))
                      .collect();
                    let args_tuple = arg_texts.join(", ") +
                                     if arg_texts.len() == 1 { "," } else { "" };
                    content.push(format!("pub struct {}<'a>(&'a {});\n", receiver.type_name, obj_name));
                    content.push(format!("\
impl<'a> {connections_mod}::Receiver for {type_name}<'a> {{
  type Arguments = ({arguments});
  fn object(&self) -> &{object_type_name} {{ self.0.static_cast() }}
  fn receiver_id() -> &'static [u8] {{ b\"{receiver_id}\\0\" }}
}}\n",
                                         type_name = receiver.type_name,
                                         arguments = args_tuple,
                                         connections_mod = connections_mod,
                                         object_type_name = object_type_name,
                                         receiver_id = receiver.receiver_id));
                    if *receiver_type == RustQtReceiverType::Signal {
                      content.push(format!("impl<'a> {connections_mod}::Signal for {}<'a> {{}}\n",
                                           receiver.type_name,
                                           connections_mod = connections_mod));
                    }
                    struct_content.push(format!("\
pub fn {method_name}(&self) -> {type_name} {{
  {type_name}(self.0)
}}\n",
                                                type_name = receiver.type_name,
                                                method_name = receiver.method_name));
                  }
                }
                content.push(format!("impl<'a> {}<'a> {{\n{}\n}}\n",
                                     struct_type,
                                     struct_content.join("")));
                type_impl_content.push(format!("\
pub fn {struct_method}(&self) -> {struct_type} {{
  {struct_type}(self)
}}\n",
                                               struct_method = struct_method,
                                               struct_type = struct_type));
              }
            }
            content.push(format!("impl {} {{\n{}\n}}\n", obj_name, type_impl_content.join("")));
            results.push(format!("pub mod connections {{\n{}\n}}\n\n", content.join("")));
          }
        }
        RustTypeDeclarationKind::MethodParametersTrait { ref shared_arguments,
                                                         ref impls,
                                                         ref lifetime,
                                                         ref return_type,
                                                         ref is_unsafe,
                                                         .. } => {
          let arg_list = self.arg_texts(shared_arguments, lifetime.as_ref()).join(", ");
          let trait_lifetime_specifier = match *lifetime {
            Some(ref lf) => format!("<'{}>", lf),
            None => String::new(),
          };
          if impls.is_empty() {
            return Err("MethodParametersTrait with empty impls".into());
          }
          let return_type_decl = if return_type.is_some() {
            ""
          } else {
            "type ReturnType;"
          };
          let return_type_string = if let Some(ref return_type) = *return_type {
            self.rust_type_to_code(return_type)
          } else {
            "Self::ReturnType".to_string()
          };
          results.push(format!("pub trait {name}{trait_lifetime_specifier} {{
              {return_type_decl}\
              fn exec(self, {arg_list}) -> {return_type_string};
            }}",
                               name = try!(type1.name.last_name()),
                               arg_list = arg_list,
                               trait_lifetime_specifier = trait_lifetime_specifier,
                               return_type_decl = return_type_decl,
                               return_type_string = return_type_string));
          for variant in impls {
            let final_lifetime =
              if lifetime.is_none() &&
                 (variant.arguments.iter().any(|t| t.argument_type.rust_api_type.is_ref()) ||
                  variant.return_type.rust_api_type.is_ref()) {
                Some("a".to_string())
              } else {
                lifetime.clone()
              };
            let lifetime_specifier = match final_lifetime {
              Some(ref lf) => format!("<'{}>", lf),
              None => String::new(),
            };
            let final_arg_list = self.arg_texts(shared_arguments, final_lifetime.as_ref())
              .join(", ");
            let tuple_item_types: Vec<_> = variant.arguments
              .iter()
              .map(|t| {
                if let Some(ref lifetime) = final_lifetime {
                  self.rust_type_to_code(&t.argument_type
                    .rust_api_type
                    .with_lifetime(lifetime.to_string()))
                } else {
                  self.rust_type_to_code(&t.argument_type
                    .rust_api_type)
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
            let return_type_string = match final_lifetime {
              Some(ref lifetime) => {
                self.rust_type_to_code(&variant.return_type
                  .rust_api_type
                  .with_lifetime(lifetime.to_string()))
              }
              None => {
                self.rust_type_to_code(&variant.return_type
                  .rust_api_type)
              }
            };
            let return_type_decl = if return_type.is_some() {
              String::new()
            } else {
              format!("type ReturnType = {};", return_type_string)
            };
            results.push(format!(include_str!("../templates/crate/impl_overloading_trait.rs.in"),
                            lifetime_specifier = lifetime_specifier,
                            trait_lifetime_specifier = trait_lifetime_specifier,
                            trait_name = try!(type1.name.last_name()),
                            final_arg_list = final_arg_list,
                            impl_type = if tuple_item_types.len() == 1 {
                              tuple_item_types[0].clone()
                            } else {
                              format!("({})", tuple_item_types.join(","))
                            },
                            return_type_decl = return_type_decl,
                            return_type_string = return_type_string,
                            tmp_vars = tmp_vars.join("\n"),
                            body =
                              try!(self.generate_ffi_call(variant, shared_arguments, *is_unsafe))));

          }
        }
      };
    }
    for method in &data.functions {
      results.push(try!(self.generate_rust_final_function(method)));
    }
    results.push(try!(self.generate_trait_impls(&data.trait_impls)));
    for submodule in &data.submodules {
      results.push(format!("pub mod {} {{\n{}}}\n\n",
                           submodule.name,
                           try!(self.generate_module_code(submodule))));
    }
    Ok(results.join(""))
  }

  fn call_rustfmt(&self, path: &PathBuf) {
    log::noisy(format!("Formatting {}", path.display()));
    let result = ::std::panic::catch_unwind(|| {
      rustfmt::format_input(rustfmt::Input::File(path.clone()),
                            &self.rustfmt_config,
                            Some(&mut ::std::io::stdout()))
    });
    match result {
      Ok(rustfmt_result) => {
        if rustfmt_result.is_err() {
          log::warning(format!("rustfmt returned Err on file: {:?}", path));
        }
      }
      Err(cause) => {
        log::warning(format!("rustfmt paniced on file: {:?}: {:?}", path, cause));
      }
    }
    assert!(path.as_path().is_file());
  }

  pub fn generate_module_file(&self, data: &RustModule) -> Result<()> {
    let mut file_path = self.config.output_path.clone();
    file_path.push("src");
    file_path.push(format!("{}.rs", &data.name));
    {
      let mut file = try!(create_file(&file_path));
      try!(file.write(try!(self.generate_module_code(data))));
    }
    self.call_rustfmt(&file_path);
    Ok(())
  }

  pub fn generate_ffi_file(&self, functions: &[(String, Vec<RustFFIFunction>)]) -> Result<()> {
    let mut file_path = self.config.output_path.clone();
    file_path.push("src");
    file_path.push("ffi.rs");
    {
      let mut file = try!(create_file(&file_path));
      for item in &self.config.link_items {
        match item.kind {
          RustLinkKind::SharedLibrary => {
            try!(file.write(format!("#[link(name = \"{}\")]\n", item.name)));
          }
          RustLinkKind::Framework => {
            try!(file.write(format!("#[link(name = \"{}\", kind = \"framework\")]\n", item.name)));
          }
        }
      }
      if !is_msvc() {
        try!(file.write("#[link(name = \"stdc++\")]\n"));
      }
      if self.config.c_lib_is_shared {
        try!(file.write(format!("#[link(name = \"{}\")]\n", &self.config.c_lib_name)));
      } else {
        try!(file.write(format!("#[link(name = \"{}\", kind = \"static\")]\n",
                                &self.config.c_lib_name)));
      }
      try!(file.write("extern \"C\" {\n"));

      for &(ref include_file, ref functions) in functions {
        try!(file.write(format!("  // Header: {}\n", include_file)));
        for function in functions {
          try!(file.write(self.rust_ffi_function_to_code(function)));
        }
        try!(file.write("\n"));
      }
      try!(file.write("}\n"));
    }
    // no rustfmt for ffi file
    Ok(())
  }
}
