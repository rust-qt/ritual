use cpp_ffi_data::{QtSlotWrapper, CppIndirectionChange, CppAndFfiMethod, CppFfiArgumentMeaning,
                   CppFfiHeaderData, CppFfiType, CppFieldAccessorType, CppFfiMethodKind};
use cpp_method::ReturnValueAllocationPlace;
use cpp_type::{CppTypeIndirection, CppTypeBase, CppType};
use common::errors::{Result, ChainErr, unexpected};
use common::file_utils::{PathBufWithAdded, create_dir_all, create_file, path_to_str};
use common::string_utils::JoinWithSeparator;
use common::utils::MapIfOk;
use common::utils::get_command_output;

use std::path::PathBuf;
use std::iter::once;
use std::process::Command;

/// Generates C++ code for the C wrapper library.
pub struct CppCodeGenerator {
  /// Library name
  lib_name: String,
  /// Uppercase library name (for optimization)
  lib_name_upper: String,
  /// Path to the directory where the library is generated
  lib_path: ::std::path::PathBuf,
}

impl CppCodeGenerator {
  /// Creates a generator for a library.
  /// lib_name: library name
  /// lib_path: path to the directory where the library is generated
  pub fn new(lib_name: String, lib_path: ::std::path::PathBuf) -> Self {
    CppCodeGenerator {
      lib_name: lib_name.clone(),
      lib_name_upper: lib_name.to_uppercase(),
      lib_path: lib_path,
    }
  }

  /// Generates function name, return type and arguments list
  /// as it appears in both function declaration and implementation.
  fn function_signature(&self, method: &CppAndFfiMethod) -> Result<String> {
    let mut arg_texts = Vec::new();
    for arg in &method.c_signature.arguments {
      arg_texts.push(arg.to_cpp_code()?);
    }
    let name_with_args = format!("{}({})", method.c_name, arg_texts.join(", "));
    let return_type = &method.c_signature.return_type.ffi_type;
    let r = if let CppTypeBase::FunctionPointer(..) = return_type.base {
      return_type.to_cpp_code(Some(&name_with_args))?
    } else {
      format!("{} {}", return_type.to_cpp_code(None)?, name_with_args)
    };
    Ok(r)
  }

  /// Generates method declaration for the header.
  fn function_declaration(&self, method: &CppAndFfiMethod) -> Result<String> {
    Ok(format!("{}_EXPORT {};\n",
               self.lib_name_upper,
               self.function_signature(method)?))
  }

  /// Generates code for a Qt slot wrapper
  fn qt_slot_wrapper(&self, wrapper: &QtSlotWrapper) -> Result<String> {
    let func_type = CppType {
      base: CppTypeBase::FunctionPointer(wrapper.function_type.clone()),
      indirection: CppTypeIndirection::None,
      is_const: false,
      is_const2: false,
    };
    let method_args = wrapper
      .arguments
      .iter()
      .enumerate()
      .map_if_ok(|(num, t)| -> Result<_> {
                   Ok(format!("{} arg{}", t.original_type.to_cpp_code(None)?, num))
                 })?
      .join(", ");
    let func_args = once("m_data".to_string())
      .chain(wrapper
               .arguments
               .iter()
               .enumerate()
               .map_if_ok(|(num, t)| self.convert_type_to_ffi(t, format!("arg{}", num)))?)
      .join(", ");
    Ok(format!(include_str!("../templates/c_lib/qt_slot_wrapper.h"),
               class_name = &wrapper.class_name,
               func_arg = func_type.to_cpp_code(Some("func"))?,
               func_field = func_type.to_cpp_code(Some("m_func"))?,
               method_args = method_args,
               func_args = func_args))



  }

  /// Generates code that wraps `expression` of type `type1.original_type` and
  /// converts it to type `type1.ffi_type`
  fn convert_type_to_ffi(&self, type1: &CppFfiType, expression: String) -> Result<String> {
    Ok(match type1.conversion {
         CppIndirectionChange::NoChange => expression,
         CppIndirectionChange::ValueToPointer => {
           format!("new {}({})",
                   type1.original_type.base.to_cpp_code(None)?,
                   expression)
         }
         CppIndirectionChange::ReferenceToPointer => format!("&{}", expression),
         CppIndirectionChange::QFlagsToUInt => format!("uint({})", expression),
       })
  }

  /// Wraps `expression` returned by the original C++ method to
  /// convert it to return type of the FFI method.
  fn convert_return_type(&self, method: &CppAndFfiMethod, expression: String) -> Result<String> {
    let mut result = expression;
    match method.c_signature.return_type.conversion {
      CppIndirectionChange::NoChange => {}
      CppIndirectionChange::ValueToPointer => {
        match method.allocation_place {
          ReturnValueAllocationPlace::Stack => {
            return Err(unexpected("stack allocated wrappers are expected to return void").into());
          }
          ReturnValueAllocationPlace::NotApplicable => {
            return Err(unexpected("ValueToPointer conflicts with NotApplicable").into());
          }
          ReturnValueAllocationPlace::Heap => {
            // constructors are said to return values in parse result,
            // but in reality we use `new` which returns a pointer,
            // so no conversion is necessary for constructors.
            if !method.cpp_method.is_constructor() {
              result = format!("new {}({})",
                               method.cpp_method.return_type.base.to_cpp_code(None)?,
                               result);
            }
          }
        }
      }
      CppIndirectionChange::ReferenceToPointer => {
        result = format!("&{}", result);
      }
      CppIndirectionChange::QFlagsToUInt => {
        result = format!("uint({})", result);
      }
    }

    if method.allocation_place == ReturnValueAllocationPlace::Stack &&
       !method.cpp_method.is_constructor() {
      if let Some(arg) = method
           .c_signature
           .arguments
           .iter()
           .find(|x| x.meaning == CppFfiArgumentMeaning::ReturnValue) {
        result = format!("new({}) {}({})",
                         arg.name,
                         method.cpp_method.return_type.base.to_cpp_code(None)?,
                         result);
      }
    }
    Ok(result)
  }

  /// Generates code for values passed to the original C++ method.
  fn arguments_values(&self, method: &CppAndFfiMethod) -> Result<String> {
    let mut filled_arguments = vec![];
    for (i, cpp_argument) in method.cpp_method.arguments.iter().enumerate() {
      if let Some(c_argument) = method
           .c_signature
           .arguments
           .iter()
           .find(|x| x.meaning == CppFfiArgumentMeaning::Argument(i as i8)) {
        let mut result = c_argument.name.clone();
        match c_argument.argument_type.conversion {
          CppIndirectionChange::ValueToPointer |
          CppIndirectionChange::ReferenceToPointer => result = format!("*{}", result),
          CppIndirectionChange::NoChange => {}
          CppIndirectionChange::QFlagsToUInt => {
            let type_text = if cpp_argument.argument_type.indirection == CppTypeIndirection::Ref &&
                               cpp_argument.argument_type.is_const {
              let mut fake_type = cpp_argument.argument_type.clone();
              fake_type.is_const = false;
              fake_type.indirection = CppTypeIndirection::None;
              fake_type.to_cpp_code(None)?
            } else {
              cpp_argument.argument_type.to_cpp_code(None)?
            };
            result = format!("{}({})", type_text, result);
          }
        }
        filled_arguments.push(result);
      } else {
        panic!("Error: no positional argument found\n{:?}", method);
      }
    }
    Ok(filled_arguments.into_iter().join(", "))
  }

  /// Generates code for the value returned by the FFI method.
  #[cfg_attr(feature="clippy", allow(collapsible_if))]
  fn returned_expression(&self, method: &CppAndFfiMethod) -> Result<String> {
    let result = if method.cpp_method.is_destructor() {
      if let Some(arg) = method
           .c_signature
           .arguments
           .iter()
           .find(|x| x.meaning == CppFfiArgumentMeaning::This) {
        format!("{}_call_destructor({})", self.lib_name, arg.name)
      } else {
        return Err(unexpected("no this arg in destructor").into());
      }
    } else {
      let mut is_field_accessor = false;
      let result_without_args = if let Some(info) = method.cpp_method.class_info_if_constructor() {
        let class_type = &info.class_type;
        match method.allocation_place {
          ReturnValueAllocationPlace::Stack => {
            if let Some(arg) = method
                 .c_signature
                 .arguments
                 .iter()
                 .find(|x| x.meaning == CppFfiArgumentMeaning::ReturnValue) {
              format!("new({}) {}", arg.name, class_type.to_cpp_code()?)
            } else {
              return Err(unexpected(format!("return value argument not found\n{:?}", method))
                           .into());
            }
          }
          ReturnValueAllocationPlace::Heap => format!("new {}", class_type.to_cpp_code()?),
          ReturnValueAllocationPlace::NotApplicable => {
            return Err(unexpected("NotApplicable in constructor").into());
          }
        }
      } else {
        let scope_specifier = if let Some(ref class_membership) =
          method.cpp_method.class_membership {
          if class_membership.is_static {
            format!("{}::", class_membership.class_type.to_cpp_code()?)
          } else {
            if let Some(arg) = method
                 .c_signature
                 .arguments
                 .iter()
                 .find(|x| x.meaning == CppFfiArgumentMeaning::This) {
              format!("{}->", arg.name)
            } else {
              return Err(unexpected("no this arg in non-static method").into());
            }
          }
        } else {
          "".to_string()
        };
        let template_args = match method.cpp_method.template_arguments_values {
          Some(ref args) => {
            let mut texts = Vec::new();
            for arg in args {
              texts.push(arg.to_cpp_code(None)?);
            }
            format!("<{}>", texts.join(", "))
          }
          None => String::new(),
        };
        if let CppFfiMethodKind::FieldAccessor {
                 ref accessor_type,
                 ref field_name,
               } = method.kind {
          is_field_accessor = true;
          if accessor_type == &CppFieldAccessorType::Setter {
            format!("{}{} = {}",
                    scope_specifier,
                    field_name,
                    self.arguments_values(method)?)
          } else {
            format!("{}{}", scope_specifier, field_name)
          }
        } else {
          format!("{}{}{}",
                  scope_specifier,
                  method.cpp_method.name,
                  template_args)
        }
      };
      if is_field_accessor {
        result_without_args
      } else {
        format!("{}({})",
                result_without_args,
                self.arguments_values(method)?)
      }
    };
    self.convert_return_type(method, result)
  }

  /// Generates body of the FFI method implementation.
  fn source_body(&self, method: &CppAndFfiMethod) -> Result<String> {
    if method.cpp_method.is_destructor() &&
       method.allocation_place == ReturnValueAllocationPlace::Heap {
      if let Some(arg) = method
           .c_signature
           .arguments
           .iter()
           .find(|x| x.meaning == CppFfiArgumentMeaning::This) {
        Ok(format!("delete {};\n", arg.name))
      } else {
        panic!("Error: no this argument found\n{:?}", method);
      }
    } else {
      Ok(format!("{}{};\n",
                 if method.c_signature.return_type.ffi_type.is_void() {
                   ""
                 } else {
                   "return "
                 },
                 self.returned_expression(&method)?))
    }
  }

  /// Generates implementation of the FFI method for the source file.
  fn function_implementation(&self, method: &CppAndFfiMethod) -> Result<String> {
    Ok(format!("{} {{\n  {}}}\n\n",
               self.function_signature(method)?,
               self.source_body(&method)?))
  }

  /// Generates main files and directories of the library.
  pub fn generate_template_files(&self, include_directives: &[PathBuf]) -> Result<()> {
    let name_upper = self.lib_name.to_uppercase();
    let cmakelists_path = self.lib_path.with_added("CMakeLists.txt");
    let mut cmakelists_file = create_file(&cmakelists_path)?;

    cmakelists_file
      .write(format!(include_str!("../templates/c_lib/CMakeLists.txt"),
                     lib_name_lowercase = &self.lib_name,
                     lib_name_uppercase = name_upper))?;
    let src_dir = self.lib_path.with_added("src");
    create_dir_all(&src_dir)?;

    let include_dir = self.lib_path.with_added("include");
    create_dir_all(&include_dir)?;
    let exports_file_path = include_dir.with_added(format!("{}_exports.h", &self.lib_name));
    let mut exports_file = create_file(&exports_file_path)?;
    exports_file
      .write(format!(include_str!("../templates/c_lib/exports.h"),
                     lib_name_uppercase = name_upper))?;

    let include_directives_code = include_directives
      .map_if_ok(|d| -> Result<_> { Ok(format!("#include \"{}\"", path_to_str(d)?)) })?
      .join("\n");

    let global_file_path = include_dir.with_added(format!("{}_global.h", &self.lib_name));
    let mut global_file = create_file(&global_file_path)?;
    global_file
      .write(format!(include_str!("../templates/c_lib/global.h"),
                     lib_name_lowercase = &self.lib_name,
                     lib_name_uppercase = name_upper,
                     include_directives_code = include_directives_code))?;
    Ok(())
  }

  /// Generates all regular files of the C++ wrapper library
  pub fn generate_files(&self, data: &[CppFfiHeaderData]) -> Result<()> {
    self
      .generate_all_headers_file(data.iter().map(|x| &x.include_file_base_name))?;
    for item in data {
      self
        .generate_one(item)
        .chain_err(|| "C++ code generator failed")?;
    }
    Ok(())
  }

  /// Generates the header file that includes all other headers of the library.
  fn generate_all_headers_file<'a, I: Iterator<Item = &'a String>>(&self, names: I) -> Result<()> {
    let mut h_path = self.lib_path.clone();
    h_path.push("include");
    h_path.push(format!("{}.h", &self.lib_name));
    let mut all_header_file = create_file(&h_path)?;
    all_header_file
      .write(format!("#ifndef {0}_H\n#define {0}_H\n\n", &self.lib_name_upper))?;
    for name in names {
      all_header_file
        .write(format!("#include \"{}_{}.h\"\n", &self.lib_name, name))?;
    }
    all_header_file
      .write(format!("#endif // {}_H\n", &self.lib_name_upper))?;
    Ok(())
  }

  /// Generates a header file and a source file for a portion of data
  /// corresponding to a header file of original C++ library.
  fn generate_one(&self, data: &CppFfiHeaderData) -> Result<()> {
    let ffi_include_file = format!("{}_{}.h", &self.lib_name, data.include_file_base_name);

    let cpp_path = self
      .lib_path
      .with_added("src")
      .with_added(format!("{}_{}.cpp", &self.lib_name, data.include_file_base_name));

    let h_path = self
      .lib_path
      .with_added("include")
      .with_added(&ffi_include_file);

    let mut cpp_file = create_file(&cpp_path)?;
    {
      let mut h_file = create_file(&h_path)?;

      cpp_file
        .write(format!("#include \"{}\"\n\n", ffi_include_file))?;
      let include_guard_name = ffi_include_file.replace(".", "_").to_uppercase();
      h_file
        .write(format!("#ifndef {}\n#define {}\n\n",
                       include_guard_name,
                       include_guard_name))?;

      h_file
        .write(format!("#include \"{}_global.h\"\n\n", &self.lib_name))?;
      for wrapper in &data.qt_slot_wrappers {
        h_file.write(self.qt_slot_wrapper(wrapper)?)?;
      }
      h_file.write("extern \"C\" {\n\n")?;
      for method in &data.methods {
        h_file.write(self.function_declaration(method)?)?;
        cpp_file.write(self.function_implementation(method)?)?;
      }

      h_file.write("\n} // extern \"C\"\n\n")?;

      h_file
        .write(format!("#endif // {}\n", include_guard_name))?;
    }
    if !data.qt_slot_wrappers.is_empty() {
      let moc_output = get_command_output(Command::new("moc").arg("-i").arg(&h_path))?;
      cpp_file
        .write(format!("// start of MOC generated code\n{}\n// end of MOC generated code\n",
                       moc_output))?;
    }
    Ok(())
  }
}

/// Entry about a Rust struct with a buffer that must have the exact same size
/// as its corresponding C++ class. This information is required for the C++ program
/// that is launched by the build script to determine type sizes and generate `type_sizes.rs`.
#[derive(Debug, Clone)]
pub struct CppTypeSizeRequest {
  /// C++ code representing the type. Used as argument to `sizeof`.
  pub cpp_code: String,
  /// Name of the constant in `type_sizes.rs`.
  pub size_const_name: String,
}

/// Generates a C++ program that determines sizes of target C++ types
/// on the current platform and outputs the Rust code for `type_sizes.rs` module
/// to the standard output.
pub fn generate_cpp_type_size_requester(requests: &[CppTypeSizeRequest],
                                        include_directives: &[PathBuf])
                                        -> Result<String> {
  let mut result = Vec::new();
  for dir in include_directives {
    result.push(format!("#include <{}>\n", path_to_str(dir)?));
  }
  result.push("#include <iostream>\n\nint main() {\n".to_string());
  for request in requests {
    result.push(format!("  std::cout << \"pub const {}: usize = \" << sizeof({}) << \";\\n\";\n",
                        request.size_const_name,
                        request.cpp_code));
  }
  result.push("}\n".to_string());
  Ok(result.join(""))
}
