use crate::cpp_data::CppPath;
use crate::cpp_data::CppPathItem;
use crate::cpp_data::CppTypeDeclarationKind;
use crate::cpp_ffi_data::CppCast;
use crate::cpp_ffi_data::CppFfiArgumentMeaning;
use crate::cpp_ffi_data::CppFfiFunction;
use crate::cpp_ffi_data::CppFfiFunctionKind;
use crate::cpp_ffi_data::CppFfiType;
use crate::cpp_ffi_data::CppFieldAccessorType;
use crate::cpp_ffi_data::CppToFfiTypeConversion;
use crate::cpp_ffi_generator::ffi_type;
use crate::cpp_function::{CppFunction, CppOperator, ReturnValueAllocationPlace};
use crate::cpp_type::is_qflags;
use crate::cpp_type::CppBuiltInNumericType;
use crate::cpp_type::CppFunctionPointerType;
use crate::cpp_type::CppPointerLikeTypeKind;
use crate::cpp_type::CppSpecificNumericType;
use crate::cpp_type::CppSpecificNumericTypeKind;
use crate::cpp_type::CppType;
use crate::cpp_type::CppTypeRole;
use crate::database::CppDatabaseItem;
use crate::database::CppFfiItem;
use crate::database::CppFfiItemKind;
use crate::database::CppItemData;
use crate::processor::ProcessorData;
use crate::rust_info::RustEnumValueDoc;
use crate::rust_info::RustFFIArgument;
use crate::rust_info::RustFFIFunction;
use crate::rust_info::RustFfiWrapperData;
use crate::rust_info::RustFunction;
use crate::rust_info::RustFunctionArgument;
use crate::rust_info::RustFunctionKind;
use crate::rust_info::RustItemKind;
use crate::rust_info::RustModule;
use crate::rust_info::RustModuleDoc;
use crate::rust_info::RustModuleKind;
use crate::rust_info::RustPathScope;
use crate::rust_info::RustQtReceiverType;
use crate::rust_info::RustStruct;
use crate::rust_info::RustStructKind;
use crate::rust_info::RustTraitAssociatedType;
use crate::rust_info::RustTraitImpl;
use crate::rust_info::RustWrapperType;
use crate::rust_info::RustWrapperTypeDocData;
use crate::rust_info::RustWrapperTypeKind;
use crate::rust_info::UnnamedRustFunction;
use crate::rust_info::{RustDatabaseItem, RustExtraImpl, RustExtraImplKind, RustRawSlotReceiver};
use crate::rust_info::{RustEnumValue, RustQtSlotWrapper};
use crate::rust_type::RustCommonType;
use crate::rust_type::RustFinalType;
use crate::rust_type::RustPath;
use crate::rust_type::RustPointerLikeTypeKind;
use crate::rust_type::RustToFfiTypeConversion;
use crate::rust_type::RustType;
use itertools::Itertools;
use log::{debug, trace};
use ritual_common::errors::{bail, err_msg, format_err, print_trace, Result};
use ritual_common::string_utils::CaseOperations;
use ritual_common::utils::MapIfOk;
use std::iter::once;
use std::ops::Deref;

pub fn qt_core_path(crate_name: &str) -> RustPath {
    if crate_name.starts_with("moqt_") {
        RustPath::from_good_str("moqt_core")
    } else {
        RustPath::from_good_str("qt_core")
    }
}

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
    Type,
    EnumValue,
    Module,
    FfiFunction,
    ApiFunction(&'a CppFfiFunction),
    ReceiverFunction,
    SizedItem,
    QtSlotWrapper {
        signal_arguments: Vec<CppType>,
        is_public: bool,
    },
}

struct State<'b, 'a: 'b>(&'b mut ProcessorData<'a>);

impl State<'_, '_> {
    /// Converts `CppType` to its exact Rust equivalent (FFI-compatible)
    fn ffi_type_to_rust_ffi_type(&self, cpp_ffi_type: &CppType) -> Result<RustType> {
        let rust_type = match &cpp_ffi_type {
            CppType::PointerLike {
                kind,
                is_const,
                target,
            } => {
                let rust_target = if target.deref() == &CppType::Void {
                    RustType::Common(RustCommonType {
                        path: RustPath::from_good_str("std::ffi::c_void"),
                        generic_arguments: None,
                    })
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
            CppType::Void => RustType::unit(),

            CppType::BuiltInNumeric(numeric) => {
                let rust_path = if numeric == &CppBuiltInNumericType::Bool {
                    // TODO: bool may not be safe for FFI
                    RustPath::from_good_str("bool")
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
                    RustPath::from_good_str("std::os::raw").join(own_name)
                };

                RustType::Common(RustCommonType {
                    path: rust_path,
                    generic_arguments: None,
                })
            }
            CppType::SpecificNumeric(CppSpecificNumericType { bits, kind, .. }) => {
                let letter = match kind {
                    CppSpecificNumericTypeKind::Integer { is_signed } => {
                        if *is_signed {
                            "i"
                        } else {
                            "u"
                        }
                    }
                    CppSpecificNumericTypeKind::FloatingPoint => "f",
                };
                let path = RustPath::from_good_str(&format!("{}{}", letter, bits));

                RustType::Common(RustCommonType {
                    path,
                    generic_arguments: None,
                })
            }
            CppType::PointerSizedInteger { is_signed, .. } => {
                let name = if *is_signed { "isize" } else { "usize" };
                RustType::Common(RustCommonType {
                    path: RustPath::from_good_str(name),
                    generic_arguments: None,
                })
            }
            CppType::Enum { path } | CppType::Class(path) => {
                let rust_item = self.find_wrapper_type(path)?;
                let path = rust_item
                    .path()
                    .ok_or_else(|| err_msg("RustDatabaseItem for class has no path"))?
                    .clone();

                RustType::Common(RustCommonType {
                    path,
                    generic_arguments: None,
                })
            }
            CppType::FunctionPointer(CppFunctionPointerType {
                return_type,
                arguments,
                allows_variadic_arguments,
            }) => {
                if *allows_variadic_arguments {
                    bail!("function pointers with variadic arguments are not supported");
                }
                let rust_args = arguments
                    .iter()
                    .map_if_ok(|arg| self.ffi_type_to_rust_ffi_type(arg))?;
                let rust_return_type = self.ffi_type_to_rust_ffi_type(return_type)?;
                let pointer = RustType::FunctionPointer {
                    arguments: rust_args,
                    return_type: Box::new(rust_return_type),
                };
                RustType::Common(RustCommonType {
                    path: RustPath::from_good_str("std::option::Option"),
                    generic_arguments: Some(vec![pointer]),
                })
            }
            CppType::TemplateParameter { .. } => bail!("invalid cpp type"),
        };

        Ok(rust_type)
    }

    fn qt_core_path(&self) -> RustPath {
        qt_core_path(self.0.config.crate_properties().name())
    }

    fn create_qflags(&self, arg: &RustPath) -> RustType {
        let path = self.qt_core_path().join("QFlags");

        RustType::Common(RustCommonType {
            path,
            generic_arguments: Some(vec![RustType::Common(RustCommonType {
                path: arg.clone(),
                generic_arguments: None,
            })]),
        })
    }

    /// Generates `CompleteType` from `CppFfiType`, adding
    /// Rust API type, Rust FFI type and conversion between them.
    #[allow(clippy::collapsible_if)]
    fn rust_final_type(
        &self,
        cpp_ffi_type: &CppFfiType,
        argument_meaning: &CppFfiArgumentMeaning,
        naming_mode: bool,
        allocation_place: ReturnValueAllocationPlace,
    ) -> Result<RustFinalType> {
        let rust_ffi_type = self.ffi_type_to_rust_ffi_type(cpp_ffi_type.ffi_type())?;
        let mut api_to_ffi_conversion = RustToFfiTypeConversion::None;
        if let RustType::PointerLike { .. } = &rust_ffi_type {
            if let CppToFfiTypeConversion::ValueToPointer { .. } = cpp_ffi_type.conversion() {
                if naming_mode {
                    api_to_ffi_conversion = RustToFfiTypeConversion::ValueToPtr;
                } else if argument_meaning == &CppFfiArgumentMeaning::ReturnValue {
                    // TODO: return error if this rust type is not deletable
                    match allocation_place {
                        ReturnValueAllocationPlace::Stack => {
                            api_to_ffi_conversion = RustToFfiTypeConversion::ValueToPtr;
                        }
                        ReturnValueAllocationPlace::Heap => {
                            api_to_ffi_conversion = RustToFfiTypeConversion::CppBoxToPtr;
                        }
                        ReturnValueAllocationPlace::NotApplicable => {
                            bail!("NotApplicable conflicts with ValueToPointer");
                        }
                    }
                } else {
                    if argument_meaning == &CppFfiArgumentMeaning::This {
                        api_to_ffi_conversion = RustToFfiTypeConversion::RefToPtr {
                            force_api_is_const: None,
                            lifetime: None,
                        };
                    } else {
                        api_to_ffi_conversion = RustToFfiTypeConversion::UtilsPtrToPtr {
                            force_api_is_const: None,
                        };
                    }
                }
            } else {
                if argument_meaning == &CppFfiArgumentMeaning::This {
                    api_to_ffi_conversion = RustToFfiTypeConversion::RefToPtr {
                        force_api_is_const: None,
                        lifetime: None,
                    };
                } else if !naming_mode {
                    api_to_ffi_conversion = RustToFfiTypeConversion::UtilsPtrToPtr {
                        force_api_is_const: None,
                    };
                }
            }
        }
        if cpp_ffi_type.conversion() == CppToFfiTypeConversion::QFlagsToInt {
            let qflags_type = match cpp_ffi_type.original_type() {
                CppType::PointerLike {
                    kind,
                    is_const,
                    target,
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

            let enum_path = if let CppType::Enum { path } = &enum_type {
                path
            } else {
                bail!("invalid QFlags argument type: {:?}", enum_type);
            };

            let rust_enum_type = self.find_wrapper_type(enum_path)?;
            let rust_enum_path = rust_enum_type.path().ok_or_else(|| {
                format_err!(
                    "failed to get path from Rust enum type: {:?}",
                    rust_enum_type
                )
            })?;

            api_to_ffi_conversion = RustToFfiTypeConversion::QFlagsToUInt {
                api_type: self.create_qflags(rust_enum_path),
            };
        };

        RustFinalType::new(rust_ffi_type, api_to_ffi_conversion)
    }

    /// Generates exact (FFI-compatible) Rust equivalent of `CppAndFfiMethod` object.
    fn generate_ffi_function(&self, data: &CppFfiFunction) -> Result<RustFFIFunction> {
        let mut args = Vec::new();
        for arg in &data.arguments {
            let rust_type = self.ffi_type_to_rust_ffi_type(arg.argument_type.ffi_type())?;
            args.push(RustFFIArgument {
                name: sanitize_rust_identifier(&arg.name, false),
                argument_type: rust_type,
            });
        }
        Ok(RustFFIFunction {
            return_type: self.ffi_type_to_rust_ffi_type(data.return_type.ffi_type())?,
            path: self.generate_rust_path(&data.path, &NameType::FfiFunction)?,
            arguments: args,
        })
    }

    fn fix_cast_function(
        mut unnamed_function: UnnamedRustFunction,
        cast: &CppCast,
        is_const: bool,
    ) -> Result<UnnamedRustFunction> {
        let force_const = if is_const { Some(true) } else { None };
        let return_type_conversion = match cast {
            CppCast::Dynamic | CppCast::QObject => RustToFfiTypeConversion::OptionUtilsPtrToPtr {
                force_api_is_const: force_const,
            },
            CppCast::Static { .. } => RustToFfiTypeConversion::UtilsPtrToPtr {
                force_api_is_const: force_const,
            },
        };
        unnamed_function.return_type = RustFinalType::new(
            unnamed_function.return_type.ffi_type().clone(),
            return_type_conversion,
        )?;

        unnamed_function.arguments[0].argument_type = RustFinalType::new(
            unnamed_function.arguments[0]
                .argument_type
                .ffi_type()
                .clone(),
            RustToFfiTypeConversion::RefToPtr {
                force_api_is_const: force_const,
                lifetime: None,
            },
        )?;
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

        let from_type = args[0].argument_type.ffi_type();
        let to_type = unnamed_function.return_type.ffi_type();

        let trait_path;
        let derived_type;
        let cast_function_name;
        let cast_function_name_mut;
        unnamed_function.is_unsafe = true;
        match &cast {
            CppCast::Static { is_unsafe, .. } => {
                if *is_unsafe {
                    trait_path = RustPath::from_good_str("cpp_utils::StaticDowncast");
                    derived_type = to_type;
                    cast_function_name = "static_downcast";
                    cast_function_name_mut = "static_downcast_mut";
                } else {
                    trait_path = RustPath::from_good_str("cpp_utils::StaticUpcast");
                    derived_type = from_type;
                    cast_function_name = "static_upcast";
                    cast_function_name_mut = "static_upcast_mut";
                }
            }
            CppCast::Dynamic => {
                trait_path = RustPath::from_good_str("cpp_utils::DynamicCast");
                derived_type = to_type;
                cast_function_name = "dynamic_cast";
                cast_function_name_mut = "dynamic_cast_mut";
            }
            CppCast::QObject => {
                trait_path = RustPath::from_good_str("qt_core::QObjectCast");
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

        let parent_path = if let RustType::Common(RustCommonType { path, .. }) =
            &derived_type.pointer_like_to_target()?
        {
            path.parent().expect("cast argument path must have parent")
        } else {
            bail!("can't get parent for derived_type: {:?}", derived_type);
        };

        let target_type = from_type.pointer_like_to_target()?;
        let to_type_value = to_type.pointer_like_to_target()?;
        results.push(RustItemKind::TraitImpl(RustTraitImpl {
            target_type: target_type.clone(),
            parent_path: parent_path.clone(),
            trait_type: RustType::Common(RustCommonType {
                path: trait_path,
                generic_arguments: Some(vec![to_type_value.clone()]),
            }),
            associated_types: Vec::new(),
            functions: vec![cast_function, cast_function_mut],
        }));

        if cast.is_first_static_cast() && !cast.is_unsafe_static_cast() {
            let fix_return_type = |type1: &mut RustFinalType, is_const: bool| -> Result<()> {
                *type1 = RustFinalType::new(
                    type1.ffi_type().clone(),
                    RustToFfiTypeConversion::RefToPtr {
                        force_api_is_const: if is_const { Some(true) } else { None },
                        lifetime: None,
                    },
                )?;
                Ok(())
            };

            let deref_trait_path = RustPath::from_good_str("std::ops::Deref");
            let mut deref_function = fixed_function.with_path(deref_trait_path.join("deref"));
            deref_function.is_unsafe = false;
            fix_return_type(&mut deref_function.return_type, true)?;
            results.push(RustItemKind::TraitImpl(RustTraitImpl {
                target_type: target_type.clone(),
                parent_path: parent_path.clone(),
                trait_type: RustType::Common(RustCommonType {
                    path: deref_trait_path,
                    generic_arguments: None,
                }),
                associated_types: vec![RustTraitAssociatedType {
                    name: "Target".to_string(),
                    value: to_type_value,
                }],
                functions: vec![deref_function],
            }));

            let deref_mut_trait_path = RustPath::from_good_str("std::ops::DerefMut");
            let mut deref_mut_function =
                fixed_function_mut.with_path(deref_mut_trait_path.join("deref_mut"));
            deref_mut_function.is_unsafe = false;
            fix_return_type(&mut deref_mut_function.return_type, false)?;
            results.push(RustItemKind::TraitImpl(RustTraitImpl {
                target_type,
                parent_path,
                trait_type: RustType::Common(RustCommonType {
                    path: deref_mut_trait_path,
                    generic_arguments: None,
                }),
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
                    function.allocation_place,
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
                    function.allocation_place,
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
                function.allocation_place,
            )?;
            (return_type, None)
        };
        if return_type.api_type().is_ref() && return_type.api_type().lifetime().is_none() {
            let mut found = false;
            for arg in &arguments {
                if let Some(lifetime) = arg.argument_type.api_type().lifetime() {
                    return_type = return_type.with_lifetime(lifetime.to_string())?;
                    found = true;
                    break;
                }
            }
            if !found {
                let mut next_lifetime_num = 0;
                for arg in &mut arguments {
                    if arg.argument_type.api_type().is_ref()
                        && arg.argument_type.api_type().lifetime().is_none()
                    {
                        arg.argument_type = arg
                            .argument_type
                            .with_lifetime(format!("l{}", next_lifetime_num))?;
                        next_lifetime_num += 1;
                    }
                }
                let return_lifetime = if next_lifetime_num == 0 {
                    debug!(
                        "Method returns a reference but doesn't receive a reference. \
                         Assuming static lifetime of return value: {}",
                        function.short_text()
                    );
                    "static".to_string()
                } else {
                    "l0".to_string()
                };
                return_type = return_type.with_lifetime(return_lifetime)?;
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
            cpp_function, cast, ..
        } = &function.kind
        {
            if cpp_function.is_destructor() {
                if arguments.len() != 1 {
                    bail!("destructor must have one argument");
                }
                let target_type = arguments[0]
                    .argument_type
                    .api_type()
                    .pointer_like_to_target()?;

                let parent_path =
                    if let RustType::Common(RustCommonType { path, .. }) = &target_type {
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
                        trait_path = RustPath::from_good_str("std::ops::Drop");
                        is_unsafe = false;
                    }
                    ReturnValueAllocationPlace::Heap => {
                        function_name = "delete";
                        trait_path = RustPath::from_good_str("cpp_utils::CppDeletable");
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
                    trait_type: RustType::Common(RustCommonType {
                        path: trait_path,
                        generic_arguments: None,
                    }),
                    associated_types: Vec::new(),
                    functions: vec![function],
                });
                return Ok(vec![rust_item]);
            }
            if let Some(cast) = cast {
                return State::process_cast(unnamed_function, cast);
            }
        }

        let cpp_path = match &function.kind {
            CppFfiFunctionKind::Function { cpp_function, .. } => &cpp_function.path,
            CppFfiFunctionKind::FieldAccessor { field, .. } => &field.path,
        };
        let path = self.generate_rust_path(cpp_path, &NameType::ApiFunction(function))?;
        let rust_item = RustItemKind::Function(unnamed_function.with_path(path));
        Ok(vec![rust_item])
    }

    fn find_rust_items(
        &self,
        cpp_path: &CppPath,
    ) -> Result<impl Iterator<Item = &RustDatabaseItem>> {
        for db in self.0.all_databases() {
            if let Some(index) = db
                .cpp_items()
                .iter()
                .position(|cpp_item| cpp_item.cpp_data.path() == Some(cpp_path))
            {
                return Ok(db
                    .rust_items()
                    .iter()
                    .filter(move |item| item.cpp_item_index == Some(index)));
            }
        }

        bail!("unknown cpp path: {}", cpp_path.to_cpp_pseudo_code())
    }

    fn find_wrapper_type(&self, cpp_path: &CppPath) -> Result<&RustDatabaseItem> {
        self.find_rust_items(cpp_path)?
            .find(|item| item.kind.is_wrapper_type())
            .ok_or_else(|| {
                format_err!("no Rust type wrapper for {}", cpp_path.to_cpp_pseudo_code())
            })
    }

    fn get_strategy(
        &self,
        parent_path: &CppPath,
        name_type: &NameType<'_>,
    ) -> Result<RustPathScope> {
        let allow_parent_type = if let NameType::Type = name_type {
            false
        } else {
            true
        };

        let rust_item = self
            .find_rust_items(parent_path)?
            .find(|item| {
                (allow_parent_type && item.kind.is_wrapper_type()) || item.kind.is_module()
            })
            .ok_or_else(|| {
                format_err!(
                    "no Rust type wrapper for {}",
                    parent_path.to_cpp_pseudo_code()
                )
            })?;

        let rust_path = rust_item.path().ok_or_else(|| {
            format_err!(
                "rust item doesn't have rust path (parent_path = {:?})",
                parent_path
            )
        })?;

        let mut rust_path = rust_path.clone();
        let path_crate_name = rust_path
            .crate_name()
            .expect("rust item path must have crate name");
        let current_crate_name = self.0.config.crate_properties().name();

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
                parts: vec![self.0.config.crate_properties().name().into()],
            },
            prefix: None,
        }
    }

    fn type_list_caption(&self, types: &[CppType], context: &RustPath) -> Result<String> {
        let mut captions = Vec::new();
        for arg in types {
            let rust_type = self.rust_final_type(
                &CppFfiType::new(arg.clone(), CppToFfiTypeConversion::NoChange)?,
                &CppFfiArgumentMeaning::Argument(0),
                true,
                ReturnValueAllocationPlace::NotApplicable,
            )?;
            captions.push(rust_type.api_type().caption(context)?);
        }
        Ok(captions.join("_"))
    }

    /// Returns method name. For class member functions, the name doesn't
    /// include class name and scope. For free functions, the name includes
    /// modules.
    fn special_function_rust_name(
        &self,
        function: &CppFfiFunction,
        context: &RustPath,
    ) -> Result<Option<String>> {
        let r = match &function.kind {
            CppFfiFunctionKind::Function { cpp_function, .. } => {
                if cpp_function.is_constructor() {
                    Some("new".to_string())
                } else if let Some(operator) = &cpp_function.operator {
                    if let CppOperator::Conversion(type1) = operator {
                        let rust_type = self.rust_final_type(
                            &ffi_type(type1, CppTypeRole::ReturnType)?,
                            &CppFfiArgumentMeaning::ReturnValue,
                            true,
                            function.allocation_place,
                        )?;
                        Some(format!("to_{}", rust_type.api_type().caption(context)?))
                    } else {
                        Some(format!("operator_{}", operator.ascii_name()?))
                    }
                } else {
                    None
                }
            }
            CppFfiFunctionKind::FieldAccessor {
                accessor_type,
                field,
            } => {
                let name = &field.path.last().name;
                let function_name = match accessor_type {
                    CppFieldAccessorType::CopyGetter | CppFieldAccessorType::ConstRefGetter => {
                        name.to_string()
                    }
                    CppFieldAccessorType::MutRefGetter => format!("{}_mut", name),
                    CppFieldAccessorType::Setter => format!("set_{}", name),
                };
                Some(function_name)
            }
        };

        Ok(r)
    }

    fn cpp_path_item_to_name(&self, item: &CppPathItem, context: &RustPath) -> Result<String> {
        if let Some(template_arguments) = &item.template_arguments {
            let captions = self.type_list_caption(template_arguments, context)?;
            Ok(format!("{}_of_{}", item.name, captions))
        } else {
            Ok(item.name.clone())
        }
    }

    fn generate_rust_path(&self, cpp_path: &CppPath, name_type: &NameType<'_>) -> Result<RustPath> {
        let strategy = match name_type {
            NameType::FfiFunction => {
                let ffi_module = self
                    .0
                    .current_database
                    .rust_items()
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
                    .0
                    .current_database
                    .rust_items()
                    .iter()
                    .filter_map(|item| item.as_module_ref())
                    .find(|module| module.kind == RustModuleKind::SizedTypes)
                    .ok_or_else(|| err_msg("sized_types module not found"))?;
                RustPathScope {
                    path: sized_module.path.clone(),
                    prefix: None,
                }
            }
            NameType::QtSlotWrapper { .. } => {
                // crate root
                self.default_strategy()
            }
            NameType::Type
            | NameType::Module
            | NameType::EnumValue
            | NameType::ApiFunction { .. }
            | NameType::ReceiverFunction => {
                if let Ok(parent) = cpp_path.parent() {
                    self.get_strategy(&parent, name_type)?
                } else {
                    self.default_strategy()
                }
            }
        };

        let full_last_name = match name_type {
            NameType::SizedItem => cpp_path
                .items()
                .iter()
                .map_if_ok(|item| self.cpp_path_item_to_name(item, &strategy.path))?
                .join("_"),
            NameType::ApiFunction(function) => {
                let s = if let Some(last_name_override) =
                    self.special_function_rust_name(function, &strategy.path)?
                {
                    last_name_override.clone()
                } else {
                    self.cpp_path_item_to_name(cpp_path.last(), &strategy.path)?
                };
                s.to_snake_case()
            }
            NameType::ReceiverFunction => self
                .cpp_path_item_to_name(cpp_path.last(), &strategy.path)?
                .to_snake_case(),
            NameType::Type | NameType::EnumValue => self
                .cpp_path_item_to_name(&cpp_path.last(), &strategy.path)?
                .to_class_case(),
            NameType::Module => self
                .cpp_path_item_to_name(&cpp_path.last(), &strategy.path)?
                .to_snake_case(),
            NameType::FfiFunction => cpp_path.last().name.clone(),
            NameType::QtSlotWrapper {
                signal_arguments,
                is_public,
            } => {
                let name = if *is_public { "Slot" } else { "RawSlot" };
                if signal_arguments.is_empty() {
                    name.to_string()
                } else {
                    let captions = self.type_list_caption(signal_arguments, &strategy.path)?;
                    format!("{}_Of_{}", name, captions).to_class_case()
                }
            }
        };

        let mut number = None;
        if name_type == &NameType::FfiFunction {
            let rust_path = strategy.apply(&full_last_name);
            if self
                .0
                .current_database
                .rust_database()
                .find(&rust_path)
                .is_some()
            {
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
            if self
                .0
                .current_database
                .rust_database()
                .find(&rust_path)
                .is_none()
            {
                return Ok(rust_path);
            }

            number = Some(number.unwrap_or(1) + 1);
        }

        // TODO: check for conflicts with types from crate template (how?)
    }

    fn process_ffi_item(&self, ffi_item: &CppFfiItem) -> Result<Vec<RustItemKind>> {
        if !ffi_item.checks.any_success() {
            bail!("cpp checks failed");
        }
        match &ffi_item.kind {
            CppFfiItemKind::Function(cpp_ffi_function) => {
                let rust_ffi_function = self.generate_ffi_function(&cpp_ffi_function)?;
                let public_items =
                    self.generate_rust_function(cpp_ffi_function, &rust_ffi_function.path)?;
                let ffi_rust_item = RustItemKind::FfiFunction(rust_ffi_function);

                Ok(once(ffi_rust_item).chain(public_items).collect_vec())
            }
            CppFfiItemKind::QtSlotWrapper(_) => {
                bail!("slot wrappers do not need to be processed here");
            }
        }
    }

    #[allow(clippy::useless_let_if_seq)]
    fn process_cpp_item(&self, cpp_item: &CppDatabaseItem) -> Result<Vec<RustItemKind>> {
        match &cpp_item.cpp_data {
            CppItemData::Namespace(path) => {
                let rust_path = self.generate_rust_path(path, &NameType::Module)?;
                let rust_item = RustItemKind::Module(RustModule {
                    is_public: true,
                    path: rust_path,
                    doc: RustModuleDoc {
                        extra_doc: None,
                        cpp_path: Some(path.clone()),
                    },
                    kind: RustModuleKind::CppNamespace,
                });
                Ok(vec![rust_item])
            }
            CppItemData::Type(data) => {
                match data.kind {
                    CppTypeDeclarationKind::Class { is_movable } => {
                        // TODO: do something about `QUrlTwoFlags<T1, T2>`
                        if is_qflags(&data.path) {
                            let argument =
                                &data.path.last().template_arguments.as_ref().unwrap()[0];
                            if !argument.is_template_parameter() {
                                if let CppType::Enum { path } = &argument {
                                    let rust_type = self.find_wrapper_type(path)?;
                                    let rust_type_path =
                                        rust_type.path().expect("enum rust item must have path");
                                    let rust_item = RustItemKind::ExtraImpl(RustExtraImpl {
                                        parent_path: rust_type_path.parent()?,
                                        kind: RustExtraImplKind::FlagEnum {
                                            enum_path: rust_type_path.clone(),
                                        },
                                    });
                                    return Ok(vec![rust_item]);
                                }
                            }
                        }

                        let mut qt_slot_wrapper = None;
                        if let Some(ffi_index) = cpp_item.source_ffi_item {
                            let ffi_item = self
                                .0
                                .current_database
                                .ffi_items()
                                .get(ffi_index)
                                .ok_or_else(|| err_msg("cpp item references invalid ffi index"))?;
                            if let CppFfiItemKind::QtSlotWrapper(wrapper) = &ffi_item.kind {
                                qt_slot_wrapper = Some(wrapper);
                            }
                        }

                        let public_name_type = if let Some(wrapper) = qt_slot_wrapper {
                            NameType::QtSlotWrapper {
                                signal_arguments: wrapper.signal_arguments.clone(),
                                is_public: false,
                            }
                        } else {
                            NameType::Type
                        };

                        let public_path = self.generate_rust_path(&data.path, &public_name_type)?;

                        let mut rust_items = Vec::new();

                        let wrapper_kind;
                        if is_movable {
                            let internal_path =
                                self.generate_rust_path(&data.path, &NameType::SizedItem)?;

                            if internal_path == public_path {
                                bail!(
                                    "internal path is the same as public path: {:?}",
                                    internal_path
                                );
                            }

                            let internal_rust_item = RustItemKind::Struct(RustStruct {
                                extra_doc: None,
                                path: internal_path.clone(),
                                kind: RustStructKind::SizedType(data.path.clone()),
                                is_public: true,
                            });

                            rust_items.push(internal_rust_item);

                            wrapper_kind = RustWrapperTypeKind::MovableClassWrapper {
                                sized_type_path: internal_path,
                            };
                        } else {
                            wrapper_kind = RustWrapperTypeKind::ImmovableClassWrapper;
                        }

                        let public_rust_item = RustItemKind::Struct(RustStruct {
                            extra_doc: None,
                            path: public_path.clone(),
                            kind: RustStructKind::WrapperType(RustWrapperType {
                                doc_data: RustWrapperTypeDocData {
                                    cpp_path: data.path.clone(),
                                    cpp_doc: data.doc.clone(),
                                    raw_qt_slot_wrapper: None, // TODO: fix this
                                },
                                kind: wrapper_kind,
                            }),
                            is_public: true,
                        });
                        rust_items.push(public_rust_item);

                        let nested_types_path =
                            self.generate_rust_path(&data.path, &NameType::Module)?;

                        let nested_types_rust_item = RustItemKind::Module(RustModule {
                            is_public: true,
                            path: nested_types_path,
                            doc: RustModuleDoc {
                                extra_doc: None,
                                cpp_path: Some(data.path.clone()),
                            },
                            kind: RustModuleKind::CppNestedType,
                        });
                        rust_items.push(nested_types_rust_item);

                        if let Some(wrapper) = qt_slot_wrapper {
                            let arg_types = wrapper
                                .signal_arguments
                                .iter()
                                .map_if_ok(|t| self.ffi_type_to_rust_ffi_type(t))?;

                            let receiver_id = CppFunction::receiver_id_from_data(
                                RustQtReceiverType::Slot,
                                "custom_slot",
                                &wrapper.signal_arguments,
                            )?;

                            let impl_item = RustItemKind::ExtraImpl(RustExtraImpl {
                                parent_path: public_path.parent()?,
                                kind: RustExtraImplKind::RawSlotReceiver(RustRawSlotReceiver {
                                    receiver_id,
                                    target_path: public_path.clone(),
                                    arguments: RustType::Tuple(arg_types),
                                }),
                            });
                            rust_items.push(impl_item);

                            let closure_item_path = self.generate_rust_path(
                                &data.path,
                                &NameType::QtSlotWrapper {
                                    signal_arguments: wrapper.signal_arguments.clone(),
                                    is_public: true,
                                },
                            )?;

                            let public_args = wrapper.arguments.iter().map_if_ok(|arg| {
                                self.rust_final_type(
                                    arg,
                                    // TODO: this is kind of a return type but not quite
                                    &CppFfiArgumentMeaning::Argument(0),
                                    false,
                                    ReturnValueAllocationPlace::NotApplicable,
                                )
                            })?;

                            let public_item = RustItemKind::Struct(RustStruct {
                                is_public: true,
                                kind: RustStructKind::QtSlotWrapper(RustQtSlotWrapper {
                                    arguments: public_args,
                                    signal_arguments: wrapper.signal_arguments.clone(),
                                    raw_slot_wrapper: public_path,
                                }),
                                path: closure_item_path,
                                extra_doc: None,
                            });
                            rust_items.push(public_item);
                        }

                        Ok(rust_items)
                    }
                    CppTypeDeclarationKind::Enum => {
                        let rust_path = self.generate_rust_path(&data.path, &NameType::Type)?;
                        let rust_item = RustItemKind::Struct(RustStruct {
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
                        });

                        Ok(vec![rust_item])
                    }
                }
            }
            CppItemData::EnumValue(value) => {
                let rust_path = self.generate_rust_path(&value.path, &NameType::EnumValue)?;

                let rust_item = RustItemKind::EnumValue(RustEnumValue {
                    path: rust_path,
                    value: value.value,
                    doc: RustEnumValueDoc {
                        cpp_path: value.path.clone(),
                        cpp_doc: value.doc.clone(),
                        extra_doc: None,
                    },
                });

                Ok(vec![rust_item])
            }
            CppItemData::Function(cpp_function) => {
                let receiver_type = if let Some(member_data) = &cpp_function.member {
                    if member_data.is_signal {
                        RustQtReceiverType::Signal
                    } else if member_data.is_slot {
                        RustQtReceiverType::Slot
                    } else {
                        return Ok(Vec::new());
                    }
                } else {
                    return Ok(Vec::new());
                };
                let receiver_id = cpp_function.receiver_id()?;
                let function_kind = RustFunctionKind::SignalOrSlotGetter {
                    cpp_path: cpp_function.path.clone(),
                    receiver_type,
                    receiver_id,
                    qobject_path: self.qt_core_path().join("QObject"),
                };

                let path =
                    self.generate_rust_path(&cpp_function.path, &NameType::ReceiverFunction)?;

                let class_type = self.find_wrapper_type(&cpp_function.path.parent()?)?;
                let self_type = RustType::PointerLike {
                    kind: RustPointerLikeTypeKind::Reference { lifetime: None },
                    is_const: true,
                    target: Box::new(RustType::Common(RustCommonType {
                        path: class_type.path().unwrap().clone(),
                        generic_arguments: None,
                    })),
                };

                let self_type = RustFinalType::new(self_type, RustToFfiTypeConversion::None)?;

                let return_type_path = match receiver_type {
                    RustQtReceiverType::Signal => self.qt_core_path().join("Signal"),
                    RustQtReceiverType::Slot => self.qt_core_path().join("Receiver"),
                };

                let arguments = cpp_function.arguments.iter().enumerate().map_if_ok(
                    |(index, arg)| -> Result<_> {
                        // TODO: rust generator shouldn't know about cpp ffi types
                        let ffi_type = ffi_type(&arg.argument_type, CppTypeRole::NotReturnType)?;
                        let rust_type = self.rust_final_type(
                            &ffi_type,
                            &CppFfiArgumentMeaning::Argument(index),
                            false,
                            ReturnValueAllocationPlace::NotApplicable,
                        )?;
                        Ok(rust_type.api_type().clone())
                    },
                )?;

                let return_type = RustType::Common(RustCommonType {
                    path: return_type_path,
                    generic_arguments: Some(vec![RustType::Tuple(arguments)]),
                });

                let return_type = RustFinalType::new(return_type, RustToFfiTypeConversion::None)?;

                let rust_function = RustFunction {
                    is_public: true,
                    is_unsafe: false,
                    path,
                    kind: function_kind,
                    arguments: vec![RustFunctionArgument {
                        argument_type: self_type,
                        name: "self".to_string(),
                        ffi_index: 42,
                    }],
                    return_type,
                    extra_doc: None,
                };
                Ok(vec![RustItemKind::Function(rust_function)])
            }
            CppItemData::ClassField(_) | CppItemData::ClassBase(_) => {
                // only need to process FFI items
                Ok(Vec::new())
            }
        }
    }

    fn generate_special_module(&mut self, kind: RustModuleKind) -> Result<()> {
        if !self
            .0
            .current_database
            .rust_items()
            .iter()
            .filter_map(|item| item.as_module_ref())
            .any(|module| module.kind == kind)
        {
            let crate_name = self.0.config.crate_properties().name().to_string();
            let rust_path_parts = match kind {
                RustModuleKind::CrateRoot => vec![crate_name],
                RustModuleKind::Ffi => vec![crate_name, "__ffi".to_string()],
                RustModuleKind::SizedTypes => vec![crate_name, "__sized_types".to_string()],
                RustModuleKind::CppNamespace | RustModuleKind::CppNestedType => unreachable!(),
            };
            let rust_path = RustPath::from_parts(rust_path_parts);

            if self
                .0
                .current_database
                .rust_database()
                .find(&rust_path)
                .is_some()
            {
                bail!("special module path already taken: {:?}", rust_path);
            }

            let rust_item = RustDatabaseItem {
                kind: RustItemKind::Module(RustModule {
                    is_public: false,
                    path: rust_path,
                    doc: RustModuleDoc {
                        extra_doc: None,
                        cpp_path: None,
                    },
                    kind,
                }),
                cpp_item_index: None,
                ffi_item_index: None,
            };
            self.0.current_database.add_rust_item(rust_item);
        }
        Ok(())
    }
}

pub fn run(data: &mut ProcessorData<'_>) -> Result<()> {
    let mut state = State(data);
    state.generate_special_module(RustModuleKind::CrateRoot)?;
    state.generate_special_module(RustModuleKind::Ffi)?;
    state.generate_special_module(RustModuleKind::SizedTypes)?;

    loop {
        let mut any_processed = false;
        for cpp_item_index in 0..state.0.current_database.cpp_items().len() {
            let cpp_item = &state.0.current_database.cpp_items()[cpp_item_index];
            if cpp_item.is_rust_processed {
                continue;
            }
            if let Ok(rust_items) = state.process_cpp_item(cpp_item) {
                let cpp_item_text = cpp_item.cpp_data.to_string();
                for rust_item in rust_items {
                    let item = RustDatabaseItem {
                        kind: rust_item,
                        cpp_item_index: Some(cpp_item_index),
                        ffi_item_index: None,
                    };
                    debug!(
                        "added rust item: {} (cpp item: {})",
                        item.kind.short_text(),
                        cpp_item_text
                    );
                    trace!("rust item data: {:?}", item);
                    state.0.current_database.add_rust_item(item);
                }
                state.0.current_database.cpp_items_mut()[cpp_item_index].is_rust_processed = true;
                any_processed = true;
            }
        }

        for ffi_item_index in 0..state.0.current_database.ffi_items().len() {
            let ffi_item = &state.0.current_database.ffi_items()[ffi_item_index];
            if ffi_item.is_rust_processed {
                continue;
            }
            if let Ok(rust_items) = state.process_ffi_item(ffi_item) {
                let ffi_item_text = ffi_item.kind.short_text();
                for rust_item in rust_items {
                    let item = RustDatabaseItem {
                        kind: rust_item,
                        cpp_item_index: None,
                        ffi_item_index: Some(ffi_item_index),
                    };
                    debug!(
                        "added rust item: {} (ffi item: {})",
                        item.kind.short_text(),
                        ffi_item_text
                    );
                    trace!("rust item data: {:?}", item);
                    state.0.current_database.add_rust_item(item);
                }
                state.0.current_database.ffi_items_mut()[ffi_item_index].is_rust_processed = true;
                any_processed = true;
            }
        }

        if !any_processed {
            break;
        }
    }

    for cpp_item in state.0.current_database.cpp_items() {
        if cpp_item.is_rust_processed {
            continue;
        }

        if let Err(err) = state.process_cpp_item(cpp_item) {
            debug!(
                "failed to process cpp item: {}: {}",
                &cpp_item.cpp_data, err
            );
            print_trace(&err, log::Level::Trace);
        }
    }

    for ffi_item in state.0.current_database.ffi_items() {
        if ffi_item.is_rust_processed {
            continue;
        }

        if let Err(err) = state.process_ffi_item(ffi_item) {
            debug!(
                "failed to process ffi item: {}: {}",
                ffi_item.kind.short_text(),
                err
            );
            print_trace(&err, log::Level::Trace);
        }
    }

    Ok(())
}

#[allow(dead_code)]
mod ported {
    use itertools::Itertools;
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
        let mut parts = WordIterator::new(s).collect_vec();
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
