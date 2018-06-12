use common::errors::{unexpected, Result};
use common::file_utils::{create_dir_all, create_file, path_to_str, PathBufWithAdded};
use common::string_utils::JoinWithSeparator;
use common::utils::MapIfOk;
use common::utils::get_command_output;
use cpp_ffi_data::{CppFfiArgumentMeaning, CppFfiMethodKind, CppFfiType, CppFieldAccessorType,
                   CppIndirectionChange, QtSlotWrapper};
use cpp_method::ReturnValueAllocationPlace;
use cpp_type::{CppType, CppTypeBase, CppTypeIndirection};

use cpp_ffi_data::CppFfiFileData;
use cpp_ffi_data::CppFfiMethod;
use std::iter::once;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// Generates C++ code for the C wrapper library.
pub struct CppCodeGenerator {
  /// Library name
  lib_name: String,
  /// Uppercase library name (for optimization)
  lib_name_upper: String,
  //  /// Path to the directory where the library is generated
  //  lib_path: ::std::path::PathBuf,
}

impl CppCodeGenerator {
  /// Creates a generator for a library.
  /// lib_name: library name
  /// lib_path: path to the directory where the library is generated
  pub fn new(lib_name: String) -> Self {
    CppCodeGenerator {
      lib_name: lib_name.clone(),
      lib_name_upper: lib_name.to_uppercase(),
    }
  }

  /// Generates function name, return type and arguments list
  /// as it appears in both function declaration and implementation.
  fn function_signature(&self, method: &CppFfiMethod) -> Result<String> {
    let mut arg_texts = Vec::new();
    for arg in &method.arguments {
      arg_texts.push(arg.to_cpp_code()?);
    }
    let name_with_args = format!("{}({})", method.name, arg_texts.join(", "));
    let return_type = &method.return_type.ffi_type;
    let r = if let CppTypeBase::FunctionPointer(..) = return_type.base {
      return_type.to_cpp_code(Some(&name_with_args))?
    } else {
      format!("{} {}", return_type.to_cpp_code(None)?, name_with_args)
    };
    Ok(r)
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
    Ok(format!(
      include_str!("../templates/c_lib/qt_slot_wrapper.h"),
      class_name = &wrapper.class_name,
      func_arg = func_type.to_cpp_code(Some("func"))?,
      func_field = func_type.to_cpp_code(Some("m_func"))?,
      method_args = method_args,
      func_args = func_args
    ))
  }

  /// Generates code that wraps `expression` of type `type1.original_type` and
  /// converts it to type `type1.ffi_type`
  fn convert_type_to_ffi(&self, type1: &CppFfiType, expression: String) -> Result<String> {
    Ok(match type1.conversion {
      CppIndirectionChange::NoChange => expression,
      CppIndirectionChange::ValueToPointer => format!(
        "new {}({})",
        type1.original_type.base.to_cpp_code(None)?,
        expression
      ),
      CppIndirectionChange::ReferenceToPointer => format!("&{}", expression),
      CppIndirectionChange::QFlagsToUInt => format!("uint({})", expression),
    })
  }

  /// Wraps `expression` returned by the original C++ method to
  /// convert it to return type of the FFI method.
  fn convert_return_type(&self, method: &CppFfiMethod, expression: String) -> Result<String> {
    let mut result = expression;
    match method.return_type.conversion {
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
            if !method
              .kind
              .cpp_method()
              .map(|m| m.is_constructor())
              .unwrap_or(false)
            {
              result = format!(
                "new {}({})",
                method.return_type.original_type.base.to_cpp_code(None)?,
                result
              );
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

    if method.allocation_place == ReturnValueAllocationPlace::Stack
      && !method
        .kind
        .cpp_method()
        .map(|m| m.is_constructor())
        .unwrap_or(false)
    {
      if let Some(arg) = method
        .arguments
        .iter()
        .find(|x| x.meaning == CppFfiArgumentMeaning::ReturnValue)
      {
        result = format!(
          "new({}) {}({})",
          arg.name,
          method.return_type.original_type.base.to_cpp_code(None)?,
          result
        );
      }
    }
    Ok(result)
  }

  /// Generates code for values passed to the original C++ method.
  fn arguments_values(&self, method: &CppFfiMethod) -> Result<String> {
    let r: Vec<_> = method.arguments.iter().map_if_ok(|argument| -> Result<_> {
      let mut result = argument.name.clone();
      match argument.argument_type.conversion {
        CppIndirectionChange::ValueToPointer | CppIndirectionChange::ReferenceToPointer => {
          result = format!("*{}", result)
        }
        CppIndirectionChange::NoChange => {}
        CppIndirectionChange::QFlagsToUInt => {
          let type_text = if argument.argument_type.original_type.indirection
            == CppTypeIndirection::Ref
            && argument.argument_type.original_type.is_const
          {
            let mut fake_type = argument.argument_type.original_type.clone();
            fake_type.is_const = false;
            fake_type.indirection = CppTypeIndirection::None;
            fake_type.to_cpp_code(None)?
          } else {
            argument.argument_type.original_type.to_cpp_code(None)?
          };
          result = format!("{}({})", type_text, result);
        }
      }
      Ok(result)
    })?;
    Ok(r.join(", "))
  }

  /// Generates code for the value returned by the FFI method.
  #[cfg_attr(feature = "clippy", allow(collapsible_if))]
  fn returned_expression(&self, method: &CppFfiMethod) -> Result<String> {
    let result = if method
      .kind
      .cpp_method()
      .map(|m| m.is_destructor())
      .unwrap_or(false)
    {
      if let Some(arg) = method
        .arguments
        .iter()
        .find(|x| x.meaning == CppFfiArgumentMeaning::This)
      {
        format!("{}_call_destructor({})", self.lib_name, arg.name)
      } else {
        return Err(unexpected("no this arg in destructor").into());
      }
    } else {
      let mut is_field_accessor = false;
      let result_without_args = if let Some(info) = method
        .kind
        .cpp_method()
        .and_then(|m| m.class_info_if_constructor())
      {
        let class_type = &info.class_type;
        match method.allocation_place {
          ReturnValueAllocationPlace::Stack => {
            if let Some(arg) = method
              .arguments
              .iter()
              .find(|x| x.meaning == CppFfiArgumentMeaning::ReturnValue)
            {
              format!("new({}) {}", arg.name, class_type.to_cpp_code()?)
            } else {
              return Err(
                unexpected(format!("return value argument not found\n{:?}", method)).into(),
              );
            }
          }
          ReturnValueAllocationPlace::Heap => format!("new {}", class_type.to_cpp_code()?),
          ReturnValueAllocationPlace::NotApplicable => {
            return Err(unexpected("NotApplicable in constructor").into());
          }
        }
      } else {
        // TODO: scope specifier should probably be stored in a field `cpp_full_name` of `CppFFiMethod`
        let scope_specifier = if let Some(ref class_membership) = method
          .kind
          .cpp_method()
          .and_then(|m| m.class_membership.as_ref())
          .and_then(|cm| if cm.is_static { Some(cm) } else { None })
        {
          // static method
          format!("{}::", class_membership.class_type.to_cpp_code()?)
        } else if let Some(ref field) =
          method
            .kind
            .cpp_field()
            .and_then(|f| if f.is_static { Some(f) } else { None })
        {
          // static field
          format!("{}::", field.class_type.to_cpp_code()?)
        } else {
          // regular member method/field or a free function
          if let Some(arg) = method
            .arguments
            .iter()
            .find(|x| x.meaning == CppFfiArgumentMeaning::This)
          {
            format!("{}->", arg.name)
          } else {
            "".to_string()
          }
        };
        let template_args = if let Some(cpp_method) = method.kind.cpp_method() {
          match cpp_method.template_arguments {
            Some(ref args) => {
              let mut texts = Vec::new();
              for arg in args {
                texts.push(arg.to_cpp_code(None)?);
              }
              format!("<{}>", texts.join(", "))
            }
            None => String::new(),
          }
        } else {
          String::new()
        };
        match method.kind {
          CppFfiMethodKind::FieldAccessor {
            ref accessor_type,
            ref field,
          } => {
            is_field_accessor = true;
            if accessor_type == &CppFieldAccessorType::Setter {
              format!(
                "{}{} = {}",
                scope_specifier,
                field.name,
                self.arguments_values(method)?
              )
            } else {
              format!("{}{}", scope_specifier, field.name)
            }
          }
          CppFfiMethodKind::Method { ref cpp_method, .. } => {
            format!("{}{}{}", scope_specifier, cpp_method.name, template_args)
          }
        }
      };
      if is_field_accessor {
        result_without_args
      } else {
        format!(
          "{}({})",
          result_without_args,
          self.arguments_values(method)?
        )
      }
    };
    self.convert_return_type(method, result)
  }

  /// Generates body of the FFI method implementation.
  fn source_body(&self, method: &CppFfiMethod) -> Result<String> {
    if method
      .kind
      .cpp_method()
      .map(|m| m.is_destructor())
      .unwrap_or(false) && method.allocation_place == ReturnValueAllocationPlace::Heap
    {
      if let Some(arg) = method
        .arguments
        .iter()
        .find(|x| x.meaning == CppFfiArgumentMeaning::This)
      {
        Ok(format!("delete {};\n", arg.name))
      } else {
        panic!("Error: no this argument found\n{:?}", method);
      }
    } else {
      Ok(format!(
        "{}{};\n",
        if method.return_type.ffi_type.is_void() {
          ""
        } else {
          "return "
        },
        self.returned_expression(&method)?
      ))
    }
  }

  /// Generates implementation of the FFI method for the source file.
  fn function_implementation(&self, method: &CppFfiMethod) -> Result<String> {
    Ok(format!(
      "{}_EXPORT {} {{\n  {}}}\n\n",
      self.lib_name_upper,
      self.function_signature(method)?,
      self.source_body(&method)?
    ))
  }

  /// Generates main files and directories of the library.
  pub fn generate_template_files(
    &self,
    lib_path: &Path,
    include_directives: &[PathBuf],
  ) -> Result<()> {
    let name_upper = self.lib_name.to_uppercase();
    let cmakelists_path = lib_path.with_added("CMakeLists.txt");
    let mut cmakelists_file = create_file(&cmakelists_path)?;

    cmakelists_file.write(format!(
      include_str!("../templates/c_lib/CMakeLists.txt"),
      lib_name_lowercase = &self.lib_name,
      lib_name_uppercase = name_upper
    ))?;
    let src_dir = lib_path.with_added("src");
    create_dir_all(&src_dir)?;

    let include_dir = lib_path.with_added("include");
    create_dir_all(&include_dir)?;
    let exports_file_path = include_dir.with_added(format!("{}_exports.h", &self.lib_name));
    let mut exports_file = create_file(&exports_file_path)?;
    exports_file.write(format!(
      include_str!("../templates/c_lib/exports.h"),
      lib_name_uppercase = name_upper
    ))?;

    let include_directives_code = include_directives
      .map_if_ok(|d| -> Result<_> { Ok(format!("#include \"{}\"", path_to_str(d)?)) })?
      .join("\n");

    let global_file_path = include_dir.with_added(format!("{}_global.h", &self.lib_name));
    let mut global_file = create_file(&global_file_path)?;
    global_file.write(format!(
      include_str!("../templates/c_lib/global.h"),
      lib_name_lowercase = &self.lib_name,
      lib_name_uppercase = name_upper,
      include_directives_code = include_directives_code
    ))?;
    Ok(())
  }

  /// Generates a source file with the specified FFI methods.
  fn generate_cpp_file(&self, data: &CppFfiFileData, file_path: &Path) -> Result<()> {
    //    let cpp_path = self
    //      .lib_path
    //      .with_added("src")
    //      .with_added(format!("{}_{}.cpp", &self.lib_name, data.name));

    let mut cpp_file = create_file(file_path)?;
    {
      for wrapper in &data.qt_slot_wrappers {
        cpp_file.write(self.qt_slot_wrapper(wrapper)?)?;
      }
      cpp_file.write("extern \"C\" {\n\n")?;
      for method in &data.methods {
        cpp_file.write(self.function_implementation(method)?)?;
      }
      cpp_file.write("\n} // extern \"C\"\n\n")?;
    }
    if !data.qt_slot_wrappers.is_empty() {
      let moc_output = get_command_output(Command::new("moc").arg("-i").arg(file_path))?;
      cpp_file.write(format!(
        "// start of MOC generated code\n{}\n// end of MOC generated code\n",
        moc_output
      ))?;
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
pub fn generate_cpp_type_size_requester(
  requests: &[CppTypeSizeRequest],
  include_directives: &[PathBuf],
) -> Result<String> {
  let mut result = Vec::new();
  for dir in include_directives {
    result.push(format!("#include <{}>\n", path_to_str(dir)?));
  }
  result.push("#include <iostream>\n\nint main() {\n".to_string());
  for request in requests {
    result.push(format!(
      "  std::cout << \"pub const {}: usize = \" << sizeof({}) << \";\\n\";\n",
      request.size_const_name, request.cpp_code
    ));
  }
  result.push("}\n".to_string());
  Ok(result.join(""))
}
