#![allow(dead_code)]

use crate::config::Config;
use crate::cpp_data::CppPath;
use crate::cpp_data::CppPathItem;
use crate::cpp_data::CppTypeDataKind;
use crate::cpp_ffi_data::CppCast;
use crate::cpp_ffi_data::CppFfiArgumentMeaning;
use crate::cpp_ffi_data::CppFfiFunction;
use crate::cpp_ffi_data::CppFfiFunctionKind;
use crate::cpp_ffi_data::CppFfiType;
use crate::cpp_ffi_data::CppFieldAccessorType;
use crate::cpp_ffi_data::CppTypeConversionToFfi;
use crate::cpp_function::ReturnValueAllocationPlace;
use crate::cpp_type::CppBuiltInNumericType;
use crate::cpp_type::CppFunctionPointerType;
use crate::cpp_type::CppPointerLikeTypeKind;
use crate::cpp_type::CppSpecificNumericType;
use crate::cpp_type::CppSpecificNumericTypeKind;
use crate::cpp_type::CppType;
use crate::database::CppDatabaseItem;
use crate::database::CppFfiItemKind;
use crate::database::CppItemData;
use crate::database::Database;
use crate::processor::ProcessingStep;
use crate::processor::ProcessorData;
use crate::rust_info::RustDatabase;
use crate::rust_info::RustDatabaseItem;
use crate::rust_info::RustEnumValue;
use crate::rust_info::RustEnumValueDoc;
use crate::rust_info::RustFFIArgument;
use crate::rust_info::RustFFIFunction;
use crate::rust_info::RustFfiClassTypeDoc;
use crate::rust_info::RustFfiWrapperData;
use crate::rust_info::RustFunctionArgument;
use crate::rust_info::RustFunctionKind;
use crate::rust_info::RustItemKind;
use crate::rust_info::RustModule;
use crate::rust_info::RustModuleDoc;
use crate::rust_info::RustModuleKind;
use crate::rust_info::RustPathScope;
use crate::rust_info::RustStruct;
use crate::rust_info::RustStructKind;
use crate::rust_info::RustTraitAssociatedType;
use crate::rust_info::RustTraitImpl;
use crate::rust_info::RustWrapperType;
use crate::rust_info::RustWrapperTypeDocData;
use crate::rust_info::RustWrapperTypeKind;
use crate::rust_info::UnnamedRustFunction;
use crate::rust_type::RustFinalType;
use crate::rust_type::RustPath;
use crate::rust_type::RustPointerLikeTypeKind;
use crate::rust_type::RustToFfiTypeConversion;
use crate::rust_type::RustType;
use log::{debug, trace};
use ritual_common::errors::*;
use ritual_common::string_utils::CaseOperations;
use ritual_common::utils::MapIfOk;
use std::collections::HashMap;
use std::ops::Deref;

/// Adds "_" to a string if it is a reserved word in Rust
fn sanitize_rust_identifier(name: &str, is_module: bool) -> String {
    match name {
        "abstract" | "alignof" | "as" | "become" | "box" | "break" | "const" | "continue"
        | "crate" | "do" | "else" | "enum" | "extern" | "false" | "final" | "fn" | "for" | "if"
        | "impl" | "in" | "let" | "loop" | "macro" | "match" | "mod" | "move" | "mut"
        | "offsetof" | "override" | "priv" | "proc" | "pub" | "pure" | "ref" | "return"
        | "Self" | "self" | "sizeof" | "static" | "struct" | "super" | "trait" | "true"
        | "type" | "typeof" | "unsafe" | "unsized" | "use" | "virtual" | "where" | "while"
        | "yield" => format!("{}_", name),
        "lib" | "main" if is_module => format!("{}_", name),
        _ => name.to_string(),
    }
}

#[test]
fn sanitize_rust_identifier_test() {
    assert_eq!(&sanitize_rust_identifier("good", false), "good");
    assert_eq!(&sanitize_rust_identifier("Self", false), "Self_");
    assert_eq!(&sanitize_rust_identifier("mod", false), "mod_");
    assert_eq!(&sanitize_rust_identifier("mod", true), "mod_");
    assert_eq!(&sanitize_rust_identifier("main", false), "main");
    assert_eq!(&sanitize_rust_identifier("main", true), "main_");
    assert_eq!(&sanitize_rust_identifier("lib", false), "lib");
    assert_eq!(&sanitize_rust_identifier("lib", true), "lib_");
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NameType<'a> {
    General,
    Module,
    FfiStruct,
    FfiFunction,
    ApiFunction(&'a CppFfiFunction),
    SizedItem,
    ClassPtr,
}

struct State<'a> {
    dep_databases: &'a [Database],
    rust_database: &'a mut RustDatabase,
    config: &'a Config,
    cpp_path_to_index: HashMap<CppPath, usize>,
}

impl State<'_> {
    /// Converts `CppType` to its exact Rust equivalent (FFI-compatible)
    fn ffi_type_to_rust_ffi_type(&self, cpp_ffi_type: &CppType) -> Result<RustType> {
        let rust_type = match cpp_ffi_type {
            CppType::PointerLike {
                ref kind,
                ref is_const,
                ref target,
            } => {
                let rust_target = if target.deref() == &CppType::Void {
                    RustType::Common {
                        path: RustPath::from_str_unchecked("std::ffi::c_void"),
                        generic_arguments: None,
                    }
                } else {
                    self.ffi_type_to_rust_ffi_type(target)?
                };
                RustType::PointerLike {
                    kind: match *kind {
                        CppPointerLikeTypeKind::Pointer => RustPointerLikeTypeKind::Pointer,
                        CppPointerLikeTypeKind::Reference
                        | CppPointerLikeTypeKind::RValueReference => {
                            bail!("references are not supported in FFI");
                        }
                    },
                    target: Box::new(rust_target),
                    is_const: *is_const,
                }
            }
            CppType::Void => RustType::Unit,

            CppType::BuiltInNumeric(ref numeric) => {
                let rust_path = if numeric == &CppBuiltInNumericType::Bool {
                    // TODO: bool may not be safe for FFI
                    RustPath::from_str_unchecked("bool")
                } else {
                    let own_name = match *numeric {
                        CppBuiltInNumericType::Bool => unreachable!(),
                        CppBuiltInNumericType::Char => "c_char",
                        CppBuiltInNumericType::SChar => "c_schar",
                        CppBuiltInNumericType::UChar => "c_uchar",
                        CppBuiltInNumericType::Short => "c_short",
                        CppBuiltInNumericType::UShort => "c_ushort",
                        CppBuiltInNumericType::Int => "c_int",
                        CppBuiltInNumericType::UInt => "c_uint",
                        CppBuiltInNumericType::Long => "c_long",
                        CppBuiltInNumericType::ULong => "c_ulong",
                        CppBuiltInNumericType::LongLong => "c_longlong",
                        CppBuiltInNumericType::ULongLong => "c_ulonglong",
                        CppBuiltInNumericType::Float => "c_float",
                        CppBuiltInNumericType::Double => "c_double",
                        _ => bail!("unsupported numeric type: {:?}", numeric),
                    };
                    RustPath::from_str_unchecked("std::os::raw").join(own_name)
                };

                RustType::Common {
                    path: rust_path,
                    generic_arguments: None,
                }
            }
            CppType::SpecificNumeric(CppSpecificNumericType {
                ref bits, ref kind, ..
            }) => {
                let letter = match *kind {
                    CppSpecificNumericTypeKind::Integer { ref is_signed } => {
                        if *is_signed {
                            "i"
                        } else {
                            "u"
                        }
                    }
                    CppSpecificNumericTypeKind::FloatingPoint => "f",
                };
                let path = RustPath::from_str_unchecked(&format!("{}{}", letter, bits));

                RustType::Common {
                    path,
                    generic_arguments: None,
                }
            }
            CppType::PointerSizedInteger { ref is_signed, .. } => {
                let name = if *is_signed { "isize" } else { "usize" };
                RustType::Common {
                    path: RustPath::from_str_unchecked(name),
                    generic_arguments: None,
                }
            }
            CppType::Enum { ref path } | CppType::Class(ref path) => {
                let rust_item = self.find_ffi_type(path)?;
                let path = rust_item
                    .path()
                    .ok_or_else(|| err_msg("RustDatabaseItem for class has no path"))?
                    .clone();

                RustType::Common {
                    path,
                    generic_arguments: None,
                }
            }
            CppType::FunctionPointer(CppFunctionPointerType {
                ref return_type,
                ref arguments,
                ref allows_variadic_arguments,
            }) => {
                if *allows_variadic_arguments {
                    bail!("function pointers with variadic arguments are not supported");
                }
                let rust_args = arguments
                    .iter()
                    .map_if_ok(|arg| self.ffi_type_to_rust_ffi_type(arg))?;
                let rust_return_type = self.ffi_type_to_rust_ffi_type(return_type)?;
                RustType::FunctionPointer {
                    arguments: rust_args,
                    return_type: Box::new(rust_return_type),
                }
            }
            CppType::TemplateParameter { .. } => bail!("invalid cpp type"),
        };

        Ok(rust_type)
    }

    /// Generates `CompleteType` from `CppFfiType`, adding
    /// Rust API type, Rust FFI type and conversion between them.
    fn rust_final_type(
        &self,
        cpp_ffi_type: &CppFfiType,
        argument_meaning: &CppFfiArgumentMeaning,
        is_template_argument: bool,
        allocation_place: &ReturnValueAllocationPlace,
    ) -> Result<RustFinalType> {
        let rust_ffi_type = self.ffi_type_to_rust_ffi_type(&cpp_ffi_type.ffi_type)?;
        let mut rust_api_type = rust_ffi_type.clone();
        let mut api_to_ffi_conversion = RustToFfiTypeConversion::None;
        if let RustType::PointerLike {
            ref mut kind,
            ref mut is_const,
            ref target,
        } = rust_api_type
        {
            match cpp_ffi_type.conversion {
                CppTypeConversionToFfi::NoChange => {
                    if argument_meaning == &CppFfiArgumentMeaning::This {
                        assert!(kind == &RustPointerLikeTypeKind::Pointer);
                        *kind = RustPointerLikeTypeKind::Reference { lifetime: None };
                        api_to_ffi_conversion = RustToFfiTypeConversion::RefToPtr;
                    }
                }
                CppTypeConversionToFfi::ValueToPointer => {
                    assert!(kind == &RustPointerLikeTypeKind::Pointer);
                    if argument_meaning == &CppFfiArgumentMeaning::ReturnValue {
                        // TODO: return error if this rust type is not deletable
                        match *allocation_place {
                            ReturnValueAllocationPlace::Stack => {
                                rust_api_type = (**target).clone();
                                api_to_ffi_conversion = RustToFfiTypeConversion::ValueToPtr;
                            }
                            ReturnValueAllocationPlace::Heap => {
                                rust_api_type = RustType::Common {
                                    path: RustPath::from_str_unchecked("cpp_utils::CppBox"),
                                    generic_arguments: Some(vec![(**target).clone()]),
                                };
                                api_to_ffi_conversion = RustToFfiTypeConversion::CppBoxToPtr;
                            }
                            ReturnValueAllocationPlace::NotApplicable => {
                                bail!("NotApplicable conflicts with ValueToPointer");
                            }
                        }
                    } else if is_template_argument {
                        rust_api_type = (**target).clone();
                        api_to_ffi_conversion = RustToFfiTypeConversion::ValueToPtr;
                    } else {
                        // there is no point in passing arguments by value because
                        // there will be an implicit copy in any case
                        *kind = RustPointerLikeTypeKind::Reference { lifetime: None };
                        *is_const = true;
                        api_to_ffi_conversion = RustToFfiTypeConversion::RefToPtr;
                    }
                }
                CppTypeConversionToFfi::ReferenceToPointer => {
                    assert!(kind == &RustPointerLikeTypeKind::Pointer);
                    *kind = RustPointerLikeTypeKind::Reference { lifetime: None };
                    api_to_ffi_conversion = RustToFfiTypeConversion::RefToPtr;
                }
                CppTypeConversionToFfi::QFlagsToUInt => unreachable!(),
            }
        }
        if cpp_ffi_type.conversion == CppTypeConversionToFfi::QFlagsToUInt {
            let qflags_type = match &cpp_ffi_type.original_type {
                CppType::PointerLike {
                    ref kind,
                    ref is_const,
                    ref target,
                } => {
                    if kind != &CppPointerLikeTypeKind::Reference {
                        bail!(
                            "unsupported indirection for QFlagsToUInt: {:?}",
                            cpp_ffi_type
                        );
                    }
                    if !*is_const {
                        bail!("unsupported is_const for QFlagsToUInt: {:?}", cpp_ffi_type);
                    }
                    &*target
                }
                a => a,
            };
            let enum_type = if let CppType::Class(path) = qflags_type {
                let template_arguments = path
                    .last()
                    .template_arguments
                    .as_ref()
                    .ok_or_else(|| err_msg("expected template arguments for QFlags"))?;
                if template_arguments.len() != 1 {
                    bail!("QFlags type must have exactly 1 template argument");
                }
                &template_arguments[0]
            } else {
                bail!("invalid original type for QFlagsToUInt: {:?}", cpp_ffi_type);
            };

            let enum_path = if let CppType::Enum { ref path } = enum_type {
                path
            } else {
                bail!("invalid QFlags argument type: {:?}", enum_type);
            };

            let rust_enum_type = self.find_wrapper_type(enum_path)?;
            let rust_enum_path = rust_enum_type.path().ok_or_else(|| {
                err_msg(format!(
                    "failed to get path from Rust enum type: {:?}",
                    rust_enum_type
                ))
            })?;

            rust_api_type = RustType::Common {
                path: RustPath::from_str_unchecked("qt_core::QFlags"),
                generic_arguments: Some(vec![RustType::Common {
                    path: rust_enum_path.clone(),
                    generic_arguments: None,
                }]),
            };

            api_to_ffi_conversion = RustToFfiTypeConversion::QFlagsToUInt;
        }

        Ok(RustFinalType {
            ffi_type: rust_ffi_type,
            api_type: rust_api_type,
            api_to_ffi_conversion,
        })
    }

    /// Generates exact (FFI-compatible) Rust equivalent of `CppAndFfiMethod` object.
    fn generate_ffi_function(&self, data: &CppFfiFunction) -> Result<RustFFIFunction> {
        let mut args = Vec::new();
        for arg in &data.arguments {
            let rust_type = self.ffi_type_to_rust_ffi_type(&arg.argument_type.ffi_type)?;
            args.push(RustFFIArgument {
                name: sanitize_rust_identifier(&arg.name, false),
                argument_type: rust_type,
            });
        }
        Ok(RustFFIFunction {
            return_type: self.ffi_type_to_rust_ffi_type(&data.return_type.ffi_type)?,
            path: self.generate_rust_path(&data.path, &NameType::FfiFunction)?,
            arguments: args,
        })
    }

    fn fix_cast_function(
        mut unnamed_function: UnnamedRustFunction,
        cast: &CppCast,
        is_const: bool,
    ) -> Result<UnnamedRustFunction> {
        let return_ref_type = unnamed_function.return_type.ptr_to_ref(is_const)?;
        match cast {
            CppCast::Static { .. } => {
                unnamed_function.return_type = return_ref_type;
            }
            CppCast::Dynamic | CppCast::QObject => {
                unnamed_function.return_type.api_to_ffi_conversion =
                    RustToFfiTypeConversion::OptionRefToPtr;
                unnamed_function.return_type.api_type = RustType::Common {
                    path: RustPath::from_str_unchecked("std::option::Option"),
                    generic_arguments: Some(vec![return_ref_type.api_type]),
                }
            }
        }

        unnamed_function.arguments[0].argument_type = unnamed_function.arguments[0]
            .argument_type
            .ptr_to_ref(is_const)?;
        unnamed_function.arguments[0].name = "self".to_string();
        Ok(unnamed_function)
    }

    fn process_cast(
        mut unnamed_function: UnnamedRustFunction,
        cast: &CppCast,
    ) -> Result<Vec<RustItemKind>> {
        let mut results = Vec::new();
        let args = &unnamed_function.arguments;
        if args.len() != 1 {
            bail!("1 argument expected");
        }

        let from_type = &args[0].argument_type;
        let to_type = &unnamed_function.return_type;

        let trait_path;
        let derived_type;
        let cast_function_name;
        let cast_function_name_mut;
        unnamed_function.is_unsafe = false;
        match cast {
            CppCast::Static { ref is_unsafe, .. } => {
                if *is_unsafe {
                    trait_path = RustPath::from_str_unchecked("cpp_utils::UnsafeStaticCast");
                    derived_type = to_type;
                    unnamed_function.is_unsafe = true;
                } else {
                    trait_path = RustPath::from_str_unchecked("cpp_utils::StaticCast");
                    derived_type = from_type;
                }
                cast_function_name = "static_cast";
                cast_function_name_mut = "static_cast_mut";
            }
            CppCast::Dynamic => {
                trait_path = RustPath::from_str_unchecked("cpp_utils::DynamicCast");
                derived_type = to_type;
                cast_function_name = "dynamic_cast";
                cast_function_name_mut = "dynamic_cast_mut";
            }
            CppCast::QObject => {
                trait_path = RustPath::from_str_unchecked("qt_core::qobject::Cast");
                derived_type = to_type;
                cast_function_name = "qobject_cast";
                cast_function_name_mut = "qobject_cast_mut";
            }
        };

        let fixed_function = State::fix_cast_function(unnamed_function.clone(), cast, true)?;
        let cast_function = fixed_function
            .clone()
            .with_path(trait_path.join(cast_function_name));

        let fixed_function_mut = State::fix_cast_function(unnamed_function.clone(), cast, false)?;
        let cast_function_mut = fixed_function_mut
            .clone()
            .with_path(trait_path.join(cast_function_name_mut));

        let parent_path = if let RustType::Common { ref path, .. } =
            derived_type.ffi_type.pointer_like_to_target()?
        {
            path.parent().expect("cast argument path must have parent")
        } else {
            bail!("can't get parent for derived_type: {:?}", derived_type);
        };

        let target_type = from_type.ptr_to_value()?.api_type;
        let to_type_value = to_type.ptr_to_value()?.api_type;
        results.push(RustItemKind::TraitImpl(RustTraitImpl {
            target_type: target_type.clone(),
            parent_path: parent_path.clone(),
            trait_type: RustType::Common {
                path: trait_path,
                generic_arguments: Some(vec![to_type_value.clone()]),
            },
            associated_types: Vec::new(),
            functions: vec![cast_function, cast_function_mut],
        }));

        if cast.is_first_static_cast() && !cast.is_unsafe_static_cast() {
            let deref_trait_path = RustPath::from_str_unchecked("std::ops::Deref");
            let deref_function = fixed_function.with_path(deref_trait_path.join("deref"));
            results.push(RustItemKind::TraitImpl(RustTraitImpl {
                target_type: target_type.clone(),
                parent_path: parent_path.clone(),
                trait_type: RustType::Common {
                    path: deref_trait_path,
                    generic_arguments: None,
                },
                associated_types: vec![RustTraitAssociatedType {
                    name: "Target".to_string(),
                    value: to_type_value,
                }],
                functions: vec![deref_function],
            }));

            let deref_mut_trait_path = RustPath::from_str_unchecked("std::ops::DerefMut");
            let deref_mut_function =
                fixed_function_mut.with_path(deref_mut_trait_path.join("deref_mut"));
            results.push(RustItemKind::TraitImpl(RustTraitImpl {
                target_type,
                parent_path,
                trait_type: RustType::Common {
                    path: deref_mut_trait_path,
                    generic_arguments: None,
                },
                associated_types: Vec::new(),
                functions: vec![deref_mut_function],
            }));
        }

        Ok(results)
    }

    /// Converts one function to a `RustSingleMethod`.
    fn generate_rust_function(
        &self,
        function: &CppFfiFunction,
        ffi_function_path: &RustPath,
    ) -> Result<Vec<RustItemKind>> {
        let mut arguments = Vec::new();
        for (arg_index, arg) in function.arguments.iter().enumerate() {
            if arg.meaning != CppFfiArgumentMeaning::ReturnValue {
                let arg_type = self.rust_final_type(
                    &arg.argument_type,
                    &arg.meaning,
                    false,
                    &function.allocation_place,
                )?;
                arguments.push(RustFunctionArgument {
                    ffi_index: arg_index,
                    argument_type: arg_type,
                    name: if arg.meaning == CppFfiArgumentMeaning::This {
                        "self".to_string()
                    } else {
                        sanitize_rust_identifier(&arg.name.to_snake_case(), false)
                    },
                });
            }
        }
        let (mut return_type, return_arg_index) = if let Some((arg_index, arg)) = function
            .arguments
            .iter()
            .enumerate()
            .find(|&(_arg_index, arg)| arg.meaning == CppFfiArgumentMeaning::ReturnValue)
        {
            // an argument has return value meaning, so
            // FFI return type must be void
            assert_eq!(function.return_type, CppFfiType::void());
            (
                self.rust_final_type(
                    &arg.argument_type,
                    &arg.meaning,
                    false,
                    &function.allocation_place,
                )?,
                Some(arg_index),
            )
        } else {
            // none of the arguments has return value meaning,
            // so FFI return value must be used
            let return_type = self.rust_final_type(
                &function.return_type,
                &CppFfiArgumentMeaning::ReturnValue,
                false,
                &function.allocation_place,
            )?;
            (return_type, None)
        };
        if return_type.api_type.is_ref() && return_type.api_type.lifetime().is_none() {
            let mut found = false;
            for arg in &arguments {
                if let Some(lifetime) = arg.argument_type.api_type.lifetime() {
                    return_type.api_type = return_type.api_type.with_lifetime(lifetime.to_string());
                    found = true;
                    break;
                }
            }
            if !found {
                let mut next_lifetime_num = 0;
                for arg in &mut arguments {
                    if arg.argument_type.api_type.is_ref()
                        && arg.argument_type.api_type.lifetime().is_none()
                    {
                        arg.argument_type.api_type = arg
                            .argument_type
                            .api_type
                            .with_lifetime(format!("l{}", next_lifetime_num));
                        next_lifetime_num += 1;
                    }
                }
                let return_lifetime = if next_lifetime_num == 0 {
                    debug!(
                            "Method returns a reference but doesn't receive a reference. Assuming static lifetime of return value: {}",
                            function.short_text()
                        );
                    "static".to_string()
                } else {
                    "l0".to_string()
                };
                return_type.api_type = return_type.api_type.with_lifetime(return_lifetime);
            }
        }

        let unnamed_function = UnnamedRustFunction {
            is_public: true,
            arguments: arguments.clone(),
            return_type,
            kind: RustFunctionKind::FfiWrapper(RustFfiWrapperData {
                cpp_ffi_function: function.clone(),
                ffi_function_path: ffi_function_path.clone(),
                return_type_ffi_index: return_arg_index,
            }),
            extra_doc: None,
            is_unsafe: true,
        };

        if let CppFfiFunctionKind::Function {
            ref cpp_function,
            ref cast,
            ..
        } = function.kind
        {
            if cpp_function.is_destructor() {
                if arguments.len() != 1 {
                    bail!("destructor must have one argument");
                }
                let target_type = arguments[0]
                    .argument_type
                    .api_type
                    .pointer_like_to_target()?;

                let parent_path = if let RustType::Common { ref path, .. } = target_type {
                    path.parent()
                        .expect("destructor argument path must have parent")
                } else {
                    bail!("can't get parent for target type: {:?}", target_type);
                };

                let function_name;
                let trait_path;
                let is_unsafe;
                match function.allocation_place {
                    ReturnValueAllocationPlace::Stack => {
                        function_name = "drop";
                        trait_path = RustPath::from_str_unchecked("std::ops::Drop");
                        is_unsafe = false;
                    }
                    ReturnValueAllocationPlace::Heap => {
                        function_name = "delete";
                        trait_path = RustPath::from_str_unchecked("cpp_utils::CppDeletable");
                        is_unsafe = true;
                    }
                    ReturnValueAllocationPlace::NotApplicable => {
                        bail!("invalid allocation_place for destructor");
                    }
                }
                let mut function = unnamed_function.with_path(trait_path.join(function_name));
                function.is_unsafe = is_unsafe;

                let rust_item = RustItemKind::TraitImpl(RustTraitImpl {
                    target_type,
                    parent_path,
                    trait_type: RustType::Common {
                        path: trait_path,
                        generic_arguments: None,
                    },
                    associated_types: Vec::new(),
                    functions: vec![function],
                });
                return Ok(vec![rust_item]);
            }
            if let Some(cast) = cast {
                return State::process_cast(unnamed_function, cast);
            }
        }

        let cpp_path = match function.kind {
            CppFfiFunctionKind::Function {
                ref cpp_function, ..
            } => &cpp_function.path,
            CppFfiFunctionKind::FieldAccessor { ref field, .. } => &field.path,
        };
        let path = self.generate_rust_path(cpp_path, &NameType::ApiFunction(function))?;
        let rust_item = RustItemKind::Function(unnamed_function.with_path(path));
        Ok(vec![rust_item])
    }

    fn find_rust_items(&self, cpp_path: &CppPath) -> Result<Vec<&RustDatabaseItem>> {
        if let Some(index) = self.cpp_path_to_index.get(cpp_path) {
            return Ok(self
                .rust_database
                .items
                .iter()
                .filter(|item| item.cpp_item_index == Some(*index))
                .collect());
        }

        for db in self.dep_databases {
            if let Some(index) = db
                .cpp_items
                .iter()
                .position(|cpp_item| cpp_item.cpp_data.path() == Some(cpp_path))
            {
                return Ok(db
                    .rust_database
                    .items
                    .iter()
                    .filter(|item| item.cpp_item_index == Some(index))
                    .collect());
            }
        }

        bail!("unknown cpp path: {}", cpp_path)
    }

    fn find_wrapper_type(&self, cpp_path: &CppPath) -> Result<&RustDatabaseItem> {
        let rust_items = self.find_rust_items(cpp_path)?;
        if rust_items.is_empty() {
            bail!("no Rust items for {}", cpp_path);
        }
        rust_items
            .into_iter()
            .find(|item| item.kind.is_wrapper_type())
            .ok_or_else(|| err_msg(format!("no Rust type wrapper for {}", cpp_path)))
    }

    fn find_ffi_type(&self, cpp_path: &CppPath) -> Result<&RustDatabaseItem> {
        let rust_items = self.find_rust_items(cpp_path)?;
        if rust_items.is_empty() {
            bail!("no Rust items for {}", cpp_path);
        }
        rust_items
            .into_iter()
            .find(|item| item.kind.is_ffi_type())
            .ok_or_else(|| err_msg(format!("no Rust FFI type for {}", cpp_path)))
    }

    fn get_strategy(&self, parent_path: &CppPath, name_type: &NameType) -> Result<RustPathScope> {
        let rust_items = self.find_rust_items(parent_path)?;
        if rust_items.is_empty() {
            bail!("no Rust items for {}", parent_path);
        }

        let rust_item = rust_items
            .into_iter()
            .find(|item| {
                let is_good_type = match name_type {
                    NameType::ApiFunction(_) => item.kind.is_ffi_type(),
                    _ => item.kind.is_wrapper_type(),
                };
                is_good_type || item.kind.is_module()
            })
            .ok_or_else(|| {
                err_msg(format!(
                    "no Rust type wrapper or module for {}",
                    parent_path
                ))
            })?;

        let rust_path = rust_item.path().ok_or_else(|| {
            err_msg(format!(
                "rust item doesn't have rust path (parent_path = {:?})",
                parent_path
            ))
        })?;

        let mut rust_path = rust_path.clone();
        let path_crate_name = rust_path
            .crate_name()
            .expect("rust item path must have crate name");
        let current_crate_name = self.config.crate_properties().name();

        if path_crate_name != current_crate_name {
            rust_path.parts[0] = current_crate_name.to_string();
        }

        Ok(RustPathScope {
            path: rust_path,
            prefix: None,
        })
    }

    fn default_strategy(&self) -> RustPathScope {
        RustPathScope {
            path: RustPath {
                parts: vec![self.config.crate_properties().name().into()],
            },
            prefix: None,
        }
    }

    fn generate_rust_path(&self, cpp_path: &CppPath, name_type: &NameType<'_>) -> Result<RustPath> {
        let strategy = match name_type {
            NameType::FfiFunction => {
                let ffi_module = self
                    .rust_database
                    .items
                    .iter()
                    .filter_map(|item| item.as_module_ref())
                    .find(|module| module.kind == RustModuleKind::Ffi)
                    .ok_or_else(|| err_msg("ffi module not found"))?;
                RustPathScope {
                    path: ffi_module.path.clone(),
                    prefix: None,
                }
            }
            NameType::SizedItem => {
                let sized_module = self
                    .rust_database
                    .items
                    .iter()
                    .filter_map(|item| item.as_module_ref())
                    .find(|module| module.kind == RustModuleKind::SizedTypes)
                    .ok_or_else(|| err_msg("sized_types module not found"))?;
                RustPathScope {
                    path: sized_module.path.clone(),
                    prefix: None,
                }
            }
            NameType::General
            | NameType::FfiStruct
            | NameType::Module
            | NameType::ClassPtr
            | NameType::ApiFunction { .. } => {
                if let Some(parent) = cpp_path.parent() {
                    self.get_strategy(&parent, name_type)?
                } else {
                    self.default_strategy()
                }
            }
        };

        let cpp_path_item_to_name = |item: &CppPathItem| {
            if item.template_arguments.is_some() {
                bail!("naming items with template arguments is not supported yet");
            }
            Ok(item.name.clone())
        };

        let full_last_name = match name_type {
            NameType::SizedItem => cpp_path
                .items
                .iter()
                .map_if_ok(|item| cpp_path_item_to_name(item))?
                .join("_"),
            NameType::ApiFunction(function) => {
                let s = if let Some(last_name_override) = special_function_rust_name(function)? {
                    last_name_override.clone()
                } else {
                    cpp_path_item_to_name(cpp_path.last())?
                };
                s.to_snake_case()
            }
            NameType::ClassPtr => format!("{}Ptr", cpp_path_item_to_name(&cpp_path.last())?),
            NameType::General | NameType::Module | NameType::FfiFunction | NameType::FfiStruct => {
                cpp_path_item_to_name(&cpp_path.last())?
            }
        };

        let mut number = None;
        if name_type == &NameType::FfiFunction {
            let rust_path = strategy.apply(&full_last_name);
            if self.rust_database.find(&rust_path).is_some() {
                bail!("ffi function path already taken: {:?}", rust_path);
            }
            return Ok(rust_path);
        }

        loop {
            let name_try = match number {
                None => full_last_name.clone(),
                Some(n) => format!("{}{}", full_last_name, n),
            };
            let sanitized_name =
                sanitize_rust_identifier(&name_try, name_type == &NameType::Module);
            let rust_path = strategy.apply(&sanitized_name);
            if self.rust_database.find(&rust_path).is_none() {
                return Ok(rust_path);
            }

            number = Some(number.unwrap_or(0) + 1);
        }

        // TODO: check for conflicts with types from crate template (how?)
    }

    fn generate_rust_items(
        &mut self,
        cpp_item: &mut CppDatabaseItem,
        cpp_item_index: usize,
        modified: &mut bool,
    ) -> Result<()> {
        if !cpp_item.is_rust_processed {
            match &cpp_item.cpp_data {
                CppItemData::Namespace(path) => {
                    let rust_path = self.generate_rust_path(path, &NameType::General)?;
                    let rust_item = RustDatabaseItem {
                        kind: RustItemKind::Module(RustModule {
                            path: rust_path,
                            doc: RustModuleDoc {
                                extra_doc: None,
                                cpp_path: Some(path.clone()),
                            },
                            kind: RustModuleKind::Normal,
                        }),
                        cpp_item_index: Some(cpp_item_index),
                    };
                    self.rust_database.items.push(rust_item);
                    *modified = true;
                    cpp_item.is_rust_processed = true;
                }
                CppItemData::Type(data) => {
                    match data.kind {
                        CppTypeDataKind::Class { is_movable } => {
                            // TODO: if the type is `QFlags<T>` or `QUrlTwoFlags<T>`,
                            //       generate `impl Flaggable` instead.
                            let internal_name_type = if is_movable {
                                NameType::SizedItem
                            } else {
                                NameType::FfiStruct
                            };
                            let public_name_type = if is_movable {
                                NameType::General
                            } else {
                                NameType::ClassPtr
                            };
                            let internal_path =
                                self.generate_rust_path(&data.path, &internal_name_type)?;
                            let public_path =
                                self.generate_rust_path(&data.path, &public_name_type)?;
                            if internal_path == public_path {
                                bail!(
                                    "internal path is the same as public path: {:?}",
                                    internal_path
                                );
                            }

                            let internal_wrapper_kind = if is_movable {
                                RustStructKind::SizedType(data.path.clone())
                            } else {
                                RustStructKind::FfiClassType(RustFfiClassTypeDoc {
                                    cpp_path: data.path.clone(),
                                    public_rust_path: public_path.clone(),
                                })
                            };

                            let internal_rust_item = RustDatabaseItem {
                                kind: RustItemKind::Struct(RustStruct {
                                    extra_doc: None,
                                    path: internal_path.clone(),
                                    kind: internal_wrapper_kind,
                                    is_public: true,
                                }),
                                cpp_item_index: Some(cpp_item_index),
                            };
                            self.rust_database.items.push(internal_rust_item);

                            let wrapper_kind = if is_movable {
                                RustWrapperTypeKind::MovableClassWrapper {
                                    sized_type_path: internal_path,
                                }
                            } else {
                                RustWrapperTypeKind::ImmovableClassWrapper {
                                    raw_type_path: internal_path,
                                }
                            };

                            let public_rust_item = RustDatabaseItem {
                                kind: RustItemKind::Struct(RustStruct {
                                    extra_doc: None,
                                    path: public_path,
                                    kind: RustStructKind::WrapperType(RustWrapperType {
                                        doc_data: RustWrapperTypeDocData {
                                            cpp_path: data.path.clone(),
                                            cpp_doc: data.doc.clone(),
                                            raw_qt_slot_wrapper: None, // TODO: fix this
                                        },
                                        kind: wrapper_kind,
                                    }),
                                    is_public: true,
                                }),
                                cpp_item_index: Some(cpp_item_index),
                            };
                            self.rust_database.items.push(public_rust_item);

                            *modified = true;
                            cpp_item.is_rust_processed = true;
                        }
                        CppTypeDataKind::Enum => {
                            let rust_path =
                                self.generate_rust_path(&data.path, &NameType::General)?;
                            let rust_item = RustDatabaseItem {
                                kind: RustItemKind::Struct(RustStruct {
                                    extra_doc: None,
                                    path: rust_path,
                                    kind: RustStructKind::WrapperType(RustWrapperType {
                                        doc_data: RustWrapperTypeDocData {
                                            cpp_path: data.path.clone(),
                                            cpp_doc: data.doc.clone(),
                                            raw_qt_slot_wrapper: None,
                                        },
                                        kind: RustWrapperTypeKind::EnumWrapper,
                                    }),
                                    is_public: true,
                                }),
                                cpp_item_index: Some(cpp_item_index),
                            };
                            self.rust_database.items.push(rust_item);
                            *modified = true;
                            cpp_item.is_rust_processed = true;
                        }
                    }
                }
                CppItemData::EnumValue(value) => {
                    let rust_path = self.generate_rust_path(&value.path, &NameType::General)?;

                    let rust_item = RustDatabaseItem {
                        kind: RustItemKind::EnumValue(RustEnumValue {
                            path: rust_path,
                            value: value.value,
                            doc: RustEnumValueDoc {
                                cpp_path: value.path.clone(),
                                cpp_doc: value.doc.clone(),
                                extra_doc: None,
                            },
                        }),
                        cpp_item_index: Some(cpp_item_index),
                    };
                    self.rust_database.items.push(rust_item);
                    *modified = true;
                    cpp_item.is_rust_processed = true;
                }
                CppItemData::Function(_)
                | CppItemData::ClassField(_)
                | CppItemData::ClassBase(_) => {
                    // only need to process FFI items
                    cpp_item.is_rust_processed = true;
                }
                _ => bail!("unimplemented"),
            }
        }
        for ffi_item in &mut cpp_item.ffi_items {
            if ffi_item.is_rust_processed {
                continue;
            }
            match &ffi_item.kind {
                CppFfiItemKind::Function(cpp_ffi_function) => {
                    let rust_ffi_function = self.generate_ffi_function(&cpp_ffi_function)?;
                    let rust_ffi_function_path = rust_ffi_function.path.clone();
                    let ffi_rust_item = RustDatabaseItem {
                        kind: RustItemKind::FfiFunction(rust_ffi_function),
                        cpp_item_index: Some(cpp_item_index),
                    };

                    for item in
                        self.generate_rust_function(cpp_ffi_function, &rust_ffi_function_path)?
                    {
                        let api_rust_item = RustDatabaseItem {
                            kind: item,
                            cpp_item_index: Some(cpp_item_index),
                        };
                        self.rust_database.items.push(api_rust_item);
                    }
                    self.rust_database.items.push(ffi_rust_item);

                    *modified = true;
                    ffi_item.is_rust_processed = true;
                }
                CppFfiItemKind::QtSlotWrapper(_) => {
                    bail!("not supported yet");
                }
            }
        }

        Ok(())
    }

    fn generate_special_module(&mut self, kind: RustModuleKind) -> Result<()> {
        if !self
            .rust_database
            .items
            .iter()
            .filter_map(|item| item.as_module_ref())
            .any(|module| module.kind == kind)
        {
            let crate_name = self.config.crate_properties().name().to_string();
            let rust_path_parts = match kind {
                RustModuleKind::CrateRoot => vec![crate_name],
                RustModuleKind::Ffi => vec![crate_name, "ffi".to_string()],
                RustModuleKind::SizedTypes => {
                    vec![crate_name, "ffi".to_string(), "sized_types".to_string()]
                }
                RustModuleKind::Normal => unreachable!(),
            };
            let rust_path = RustPath::from_parts(rust_path_parts);

            if self.rust_database.find(&rust_path).is_some() {
                bail!("special module path already taken: {:?}", rust_path);
            }

            let rust_item = RustDatabaseItem {
                kind: RustItemKind::Module(RustModule {
                    path: rust_path,
                    doc: RustModuleDoc {
                        extra_doc: None,
                        cpp_path: None,
                    },
                    kind,
                }),
                cpp_item_index: None,
            };
            self.rust_database.items.push(rust_item);
        }
        Ok(())
    }
}

fn run(data: &mut ProcessorData) -> Result<()> {
    let mut state = State {
        dep_databases: data.dep_databases,
        rust_database: &mut data.current_database.rust_database,
        config: data.config,
        cpp_path_to_index: data
            .current_database
            .cpp_items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| item.cpp_data.path().map(|path| (path.clone(), index)))
            .collect(),
    };
    state.generate_special_module(RustModuleKind::CrateRoot)?;
    state.generate_special_module(RustModuleKind::Ffi)?;
    state.generate_special_module(RustModuleKind::SizedTypes)?;

    let cpp_items = &mut data.current_database.cpp_items;

    loop {
        let mut something_changed = false;

        for (index, mut cpp_item) in cpp_items.iter_mut().enumerate() {
            if cpp_item.is_all_rust_processed() {
                continue;
            }

            let _ = state.generate_rust_items(&mut cpp_item, index, &mut something_changed);
        }

        if !something_changed {
            break;
        }
    }

    for (index, mut cpp_item) in cpp_items.iter_mut().enumerate() {
        if cpp_item.is_all_rust_processed() {
            continue;
        }

        match state.generate_rust_items(&mut cpp_item, index, &mut true) {
            Ok(_) => {
                bail!(
                    "previous iteration had no success, so fail is expected! item: {:?}",
                    cpp_item
                );
            }
            Err(err) => {
                trace!("skipping item: {}: {}", &cpp_item.cpp_data, err);
            }
        }
    }
    Ok(())
}

pub fn rust_generator_step() -> ProcessingStep {
    ProcessingStep::new("rust_generator", run)
}

pub fn clear_rust_info(data: &mut ProcessorData) -> Result<()> {
    data.current_database.rust_database.items.clear();
    for item in &mut data.current_database.cpp_items {
        item.is_rust_processed = false;
        for item in &mut item.ffi_items {
            item.is_rust_processed = false;
        }
    }
    Ok(())
}

pub fn clear_rust_info_step() -> ProcessingStep {
    ProcessingStep::new("clear_rust_info", clear_rust_info)
}

/// Returns method name. For class member functions, the name doesn't
/// include class name and scope. For free functions, the name includes
/// modules.
fn special_function_rust_name(function: &CppFfiFunction) -> Result<Option<String>> {
    let r = match function.kind {
        CppFfiFunctionKind::Function {
            ref cpp_function, ..
        } => {
            if cpp_function.is_constructor() {
                Some("new".to_string())
            } else if let Some(ref operator) = cpp_function.operator {
                Some(format!("operator_{}", operator.ascii_name()?))
            } else {
                None
            }
        }
        CppFfiFunctionKind::FieldAccessor {
            ref accessor_type,
            ref field,
        } => {
            let name = &field.path.last().name;
            let function_name = match accessor_type {
                CppFieldAccessorType::CopyGetter => name.to_string(),
                CppFieldAccessorType::ConstRefGetter => name.to_string(),
                CppFieldAccessorType::MutRefGetter => format!("{}_mut", name),
                CppFieldAccessorType::Setter => format!("set_{}", name),
            };
            Some(function_name)
        }
    };

    Ok(r)
}

#[allow(dead_code)]
mod ported {
    //use ritual_common::errors::Result;
    use ritual_common::string_utils::CaseOperations;
    use ritual_common::string_utils::WordIterator;

    /// Mode of case conversion
    #[derive(Clone, Copy)]
    enum Case {
        /// Class case: "OneTwo"
        Class,
        /// Snake case: "one_two"
        Snake,
    }

    // TODO: implement removal of arbitrary prefixes (#25)

    /// If `remove_qt_prefix` is true, removes "Q" or "Qt"
    /// if it is first word of the string and not the only one word.
    /// Also converts case of the words.
    #[allow(clippy::collapsible_if)]
    fn remove_qt_prefix_and_convert_case(s: &str, case: Case, remove_qt_prefix: bool) -> String {
        let mut parts: Vec<_> = WordIterator::new(s).collect();
        if remove_qt_prefix && parts.len() > 1 {
            if (parts[0] == "Q" || parts[0] == "q" || parts[0] == "Qt")
                && !parts[1].starts_with(|c: char| c.is_digit(10))
            {
                parts.remove(0);
            }
        }
        match case {
            Case::Snake => parts.to_snake_case(),
            Case::Class => parts.to_class_case(),
        }
    }

    #[test]
    fn remove_qt_prefix_and_convert_case_test() {
        assert_eq!(
            remove_qt_prefix_and_convert_case(&"OneTwo".to_string(), Case::Class, false),
            "OneTwo"
        );
        assert_eq!(
            remove_qt_prefix_and_convert_case(&"OneTwo".to_string(), Case::Snake, false),
            "one_two"
        );
        assert_eq!(
            remove_qt_prefix_and_convert_case(&"OneTwo".to_string(), Case::Class, true),
            "OneTwo"
        );
        assert_eq!(
            remove_qt_prefix_and_convert_case(&"OneTwo".to_string(), Case::Snake, true),
            "one_two"
        );
        assert_eq!(
            remove_qt_prefix_and_convert_case(&"QDirIterator".to_string(), Case::Class, false),
            "QDirIterator"
        );
        assert_eq!(
            remove_qt_prefix_and_convert_case(&"QDirIterator".to_string(), Case::Snake, false),
            "q_dir_iterator"
        );
        assert_eq!(
            remove_qt_prefix_and_convert_case(&"QDirIterator".to_string(), Case::Class, true),
            "DirIterator"
        );
        assert_eq!(
            remove_qt_prefix_and_convert_case(&"QDirIterator".to_string(), Case::Snake, true),
            "dir_iterator"
        );
        assert_eq!(
            remove_qt_prefix_and_convert_case(&"Qt3DWindow".to_string(), Case::Class, false),
            "Qt3DWindow"
        );
        assert_eq!(
            remove_qt_prefix_and_convert_case(&"Qt3DWindow".to_string(), Case::Snake, false),
            "qt_3d_window"
        );
        assert_eq!(
            remove_qt_prefix_and_convert_case(&"Qt3DWindow".to_string(), Case::Class, true),
            "Qt3DWindow"
        );
        assert_eq!(
            remove_qt_prefix_and_convert_case(&"Qt3DWindow".to_string(), Case::Snake, true),
            "qt_3d_window"
        );
    }
}
