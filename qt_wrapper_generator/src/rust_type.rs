use c_type::{CType, CTypeExtended, CppToCTypeConversion};
use cpp_type::CppType;

extern crate inflector;
use self::inflector::Inflector;

enum RustTypeIndirection {
  None,
  Ptr,
  Ref,
}

enum RustType {
  Void,
  NonVoid {
    base: String,
    is_const: bool,
    indirection: RustTypeIndirection,
    is_option: bool,
  },
}

enum RustToCTypeConversion {
  None,
  RefToPtr,
  ValueToPtr,
}

struct CompleteType {
  pub c_type: CType,
  pub cpp_type: CppType,
  pub cpp_to_c_conversion: CppToCTypeConversion,
  pub rust_ffi_type: RustType,
  //pub rust_api_type: RustType,
  //pub rust_api_to_c_conversion: RustToCTypeConversion,
}

impl CompleteType {
  pub fn from_c_type(c_type_ex: CTypeExtended) -> Self {
    let rust_ffi_type = if c_type_ex.c_type.base == "void" {
      if c_type_ex.c_type.is_pointer {
        RustType::NonVoid {
          base: "::c_void".to_string(),
          is_const: c_type_ex.c_type.is_const,
          indirection: RustTypeIndirection::Ptr,
          is_option: true,
        }
      } else {
        RustType::Void
      }
    } else {
      RustType::NonVoid {
        base: match c_type_ex.c_type.base.as_ref() {
          "qint8" => "i8".to_string(),
          "quint8" => "u8".to_string(),
          "qint16" => "i16".to_string(),
          "quint16" => "u16".to_string(),
          "qint32" => "i32".to_string(),
          "quint32" => "u32".to_string(),
          "qint64" => "i64".to_string(),
          "quint64" => "u64".to_string(),
          "qintptr" | "qptrdiff" | "QList_difference_type" => "isize".to_string(),
          "quintptr" => "usize".to_string(),
          "qreal" => "::qreal".to_string(),
          "float" => "f32".to_string(),
          "double" => "f64".to_string(),
          "bool" => "bool".to_string(),
          "char" => "::c_char".to_string(),
          "signed char" => "::c_schar".to_string(),
          "unsigned char" => "::c_uchar".to_string(),
          "short" => "::c_short".to_string(),
          "unsigned short" => "::c_ushort".to_string(),
          "int" => "::c_int".to_string(),
          "unsigned int" => "::c_uint".to_string(),
          "long" => "::c_long".to_string(),
          "unsigned long" => "::c_ulong".to_string(),
          "long long" => "::c_longlong".to_string(),
          "unsigned long long" => "::c_ulonglong".to_string(),
          "wchar_t" => "::wchar_t".to_string(),
          "size_t" => "::size_t".to_string(),
          c_type_base => format!("::{}", c_type_base.to_camel_case()),
        },
        is_const: c_type_ex.c_type.is_const,
        indirection: if c_type_ex.c_type.is_pointer {
          RustTypeIndirection::Ptr
        } else {
          RustTypeIndirection::None
        },
        is_option: c_type_ex.c_type.is_pointer
      }
    };

    CompleteType {
      c_type: c_type_ex.c_type,
      cpp_type: c_type_ex.cpp_type,
      cpp_to_c_conversion: c_type_ex.conversion,
      rust_ffi_type: rust_ffi_type,
      //rust_api_type: rust_api_type,
      //rust_api_to_c_conversion: rust_api_to_c_conversion,
    }
  }
}
