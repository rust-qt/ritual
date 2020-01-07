use crate::config::CrateDependencyKind;
use crate::cpp_checks::CppChecks;
use crate::cpp_data::{CppItem, CppPath, CppPathItem, CppTypeDeclaration, CppTypeDeclarationKind};
use crate::cpp_ffi_data::{
    CppCast, CppFfiArgumentMeaning, CppFfiFunction, CppFfiFunctionKind, CppFfiItem, CppFfiType,
    CppFieldAccessorType, CppToFfiTypeConversion,
};
use crate::cpp_ffi_generator::ffi_type;
use crate::cpp_function::{CppFunction, CppOperator, ReturnValueAllocationPlace};
use crate::cpp_type::{
    is_qflags, CppBuiltInNumericType, CppFunctionPointerType, CppPointerLikeTypeKind,
    CppSpecificNumericType, CppSpecificNumericTypeKind, CppType, CppTypeRole,
};
use crate::database::{DbItem, ItemId, ItemWithSource};
use crate::processor::ProcessorData;
use crate::rust_info::{
    NameType, RustEnumValue, RustExtraImpl, RustExtraImplKind, RustFfiWrapperData,
    RustFlagEnumImpl, RustFunction, RustFunctionArgument, RustFunctionCaptionStrategy,
    RustFunctionKind, RustFunctionSelfArgKind, RustItem, RustModule, RustModuleKind, RustPathScope,
    RustQtReceiverType, RustRawQtSlotWrapperData, RustRawSlotReceiver, RustReexport,
    RustReexportSource, RustSignalOrSlotGetter, RustSizedType, RustSpecialModuleKind, RustStruct,
    RustStructKind, RustTraitAssociatedType, RustTraitImpl, RustTraitImplExtraKind,
    RustTypeCaptionStrategy, RustWrapperTypeKind, UnnamedRustFunction,
};
use crate::rust_type::{
    RustClosureToCallbackConversion, RustCommonType, RustFinalType, RustFunctionPointerType,
    RustPath, RustPointerLikeTypeKind, RustToFfiTypeConversion, RustType,
};
use itertools::Itertools;
use log::{debug, trace};
use ritual_common::errors::{bail, err_msg, format_err, print_trace, Result};
use ritual_common::string_utils::CaseOperations;
use ritual_common::utils::MapIfOk;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::iter::Iterator;
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

#[derive(Debug)]
struct FunctionWithDesiredPath {
    function: UnnamedRustFunction,
    desired_path: RustPath,
}

enum ProcessedFfiItem {
    Item(RustItem),
    Function(FunctionWithDesiredPath),
}

#[derive(Debug, Clone, Copy)]
enum ReturnTypeConstraint {
    Bool,
    Usize,
    Unit,
    Any,
}

#[derive(Debug, Clone, Copy)]
struct TraitImplInfo {
    trait_path: &'static str,
    function_name: &'static str,
    is_unsafe: bool,
    is_inherent: bool,
    has_output_associated_type: bool,
    trait_arg_is_second_arg_type: bool,
    second_arg_is_reference: bool,
    return_type_constraint: ReturnTypeConstraint,
    self_arg_kind: RustFunctionSelfArgKind,
    target_is_reference: bool,
}

impl TraitImplInfo {
    fn from_operator(operator: &CppOperator) -> Option<TraitImplInfo> {
        Some(match operator {
            CppOperator::Addition => TraitImplInfo {
                trait_path: "std::ops::Add",
                function_name: "add",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::Value,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                target_is_reference: true,
            },
            CppOperator::Subtraction => TraitImplInfo {
                trait_path: "std::ops::Sub",
                function_name: "sub",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::Value,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                target_is_reference: true,
            },
            CppOperator::Multiplication => TraitImplInfo {
                trait_path: "std::ops::Mul",
                function_name: "mul",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::Value,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                target_is_reference: true,
            },
            CppOperator::Division => TraitImplInfo {
                trait_path: "std::ops::Div",
                function_name: "div",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::Value,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                target_is_reference: true,
            },
            CppOperator::Modulo => TraitImplInfo {
                trait_path: "std::ops::Rem",
                function_name: "rem",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::Value,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                target_is_reference: true,
            },
            CppOperator::BitwiseAnd => TraitImplInfo {
                trait_path: "std::ops::BitAnd",
                function_name: "bitand",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::Value,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                target_is_reference: true,
            },
            CppOperator::BitwiseOr => TraitImplInfo {
                trait_path: "std::ops::BitOr",
                function_name: "bitor",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::Value,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                target_is_reference: true,
            },
            CppOperator::BitwiseXor => TraitImplInfo {
                trait_path: "std::ops::BitXor",
                function_name: "bitxor",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::Value,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                target_is_reference: true,
            },
            CppOperator::BitwiseLeftShift => TraitImplInfo {
                trait_path: "std::ops::Shl",
                function_name: "shl",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::Value,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                target_is_reference: true,
            },
            CppOperator::BitwiseRightShift => TraitImplInfo {
                trait_path: "std::ops::Shr",
                function_name: "shr",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::Value,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                target_is_reference: true,
            },
            CppOperator::EqualTo => TraitImplInfo {
                trait_path: "std::cmp::PartialEq",
                function_name: "eq",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::ConstRef,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: true,
                return_type_constraint: ReturnTypeConstraint::Bool,
                target_is_reference: false,
            },
            CppOperator::GreaterThan => TraitImplInfo {
                trait_path: "cpp_core::cmp::Gt",
                function_name: "gt",
                is_unsafe: true,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::ConstRef,
                second_arg_is_reference: true,
                return_type_constraint: ReturnTypeConstraint::Bool,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                target_is_reference: false,
            },
            CppOperator::LessThan => TraitImplInfo {
                trait_path: "cpp_core::cmp::Lt",
                function_name: "lt",
                is_unsafe: true,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::ConstRef,
                second_arg_is_reference: true,
                return_type_constraint: ReturnTypeConstraint::Bool,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                target_is_reference: false,
            },
            CppOperator::GreaterThanOrEqualTo => TraitImplInfo {
                trait_path: "cpp_core::cmp::Ge",
                function_name: "ge",
                is_unsafe: true,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::ConstRef,
                second_arg_is_reference: true,
                return_type_constraint: ReturnTypeConstraint::Bool,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                target_is_reference: false,
            },
            CppOperator::LessThanOrEqualTo => TraitImplInfo {
                trait_path: "cpp_core::cmp::Le",
                function_name: "le",
                is_unsafe: true,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::ConstRef,
                second_arg_is_reference: true,
                return_type_constraint: ReturnTypeConstraint::Bool,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                target_is_reference: false,
            },
            CppOperator::LogicalNot => TraitImplInfo {
                trait_path: "std::ops::Not",
                function_name: "not",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::Value,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: false,
                target_is_reference: true,
            },
            CppOperator::UnaryMinus => TraitImplInfo {
                trait_path: "std::ops::Neg",
                function_name: "neg",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::Value,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                target_is_reference: true,
            },
            CppOperator::AdditionAssignment => TraitImplInfo {
                trait_path: "std::ops::AddAssign",
                function_name: "add_assign",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::MutRef,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Unit,
                target_is_reference: false,
            },
            CppOperator::SubtractionAssignment => TraitImplInfo {
                trait_path: "std::ops::SubAssign",
                function_name: "sub_assign",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::MutRef,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Unit,
                target_is_reference: false,
            },
            CppOperator::MultiplicationAssignment => TraitImplInfo {
                trait_path: "std::ops::MulAssign",
                function_name: "mul_assign",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::MutRef,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Unit,
                target_is_reference: false,
            },
            CppOperator::DivisionAssignment => TraitImplInfo {
                trait_path: "std::ops::DivAssign",
                function_name: "div_assign",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::MutRef,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Unit,
                target_is_reference: false,
            },
            CppOperator::ModuloAssignment => TraitImplInfo {
                trait_path: "std::ops::RemAssign",
                function_name: "rem_assign",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::MutRef,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Unit,
                target_is_reference: false,
            },
            CppOperator::BitwiseAndAssignment => TraitImplInfo {
                trait_path: "std::ops::BitAndAssign",
                function_name: "bitand_assign",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::MutRef,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Unit,
                target_is_reference: false,
            },
            CppOperator::BitwiseOrAssignment => TraitImplInfo {
                trait_path: "std::ops::BitOrAssign",
                function_name: "bitor_assign",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::MutRef,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Unit,
                target_is_reference: false,
            },
            CppOperator::BitwiseXorAssignment => TraitImplInfo {
                trait_path: "std::ops::BitXorAssign",
                function_name: "bitxor_assign",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::MutRef,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Unit,
                target_is_reference: false,
            },
            CppOperator::BitwiseLeftShiftAssignment => TraitImplInfo {
                trait_path: "std::ops::ShlAssign",
                function_name: "shl_assign",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::MutRef,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Unit,
                target_is_reference: false,
            },
            CppOperator::BitwiseRightShiftAssignment => TraitImplInfo {
                trait_path: "std::ops::ShrAssign",
                function_name: "shr_assign",
                is_unsafe: false,
                is_inherent: false,
                self_arg_kind: RustFunctionSelfArgKind::MutRef,
                has_output_associated_type: false,
                trait_arg_is_second_arg_type: true,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Unit,
                target_is_reference: false,
            },
            CppOperator::PrefixIncrement => TraitImplInfo {
                trait_path: "cpp_core::ops::Increment",
                function_name: "inc",
                is_unsafe: true,
                is_inherent: true,
                self_arg_kind: RustFunctionSelfArgKind::MutRef,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: false,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                target_is_reference: false,
            },
            CppOperator::PrefixDecrement => TraitImplInfo {
                trait_path: "cpp_core::ops::Decrement",
                function_name: "dec",
                is_unsafe: true,
                is_inherent: true,
                self_arg_kind: RustFunctionSelfArgKind::MutRef,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: false,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                target_is_reference: false,
            },
            CppOperator::Indirection => TraitImplInfo {
                trait_path: "cpp_core::ops::Indirection",
                function_name: "indirection",
                is_unsafe: true,
                is_inherent: true,
                self_arg_kind: RustFunctionSelfArgKind::Value,
                has_output_associated_type: true,
                trait_arg_is_second_arg_type: false,
                second_arg_is_reference: false,
                return_type_constraint: ReturnTypeConstraint::Any,
                target_is_reference: false,
            },
            CppOperator::Conversion(_)
            | CppOperator::Assignment
            | CppOperator::UnaryPlus
            | CppOperator::PostfixIncrement
            | CppOperator::PostfixDecrement
            | CppOperator::NotEqualTo
            | CppOperator::LogicalAnd
            | CppOperator::LogicalOr
            | CppOperator::BitwiseNot
            | CppOperator::Subscript
            | CppOperator::AddressOf
            | CppOperator::StructureDereference
            | CppOperator::PointerToMember
            | CppOperator::FunctionCall
            | CppOperator::Comma
            | CppOperator::New
            | CppOperator::NewArray
            | CppOperator::Delete
            | CppOperator::DeleteArray => return None,
        })
    }

    fn new(function: &CppFunction) -> Option<TraitImplInfo> {
        if let Some(operator) = &function.operator {
            return Self::from_operator(operator);
        }
        if let Some(member) = &function.member {
            if !member.is_static
                && function.arguments.is_empty()
                && function.path.last().template_arguments.is_none()
            {
                match function.path.last().name.as_str() {
                    "begin" => {
                        let info = if member.is_const {
                            TraitImplInfo {
                                trait_path: "cpp_core::ops::Begin",
                                function_name: "begin",
                                is_unsafe: true,
                                is_inherent: true,
                                self_arg_kind: RustFunctionSelfArgKind::ConstRef,
                                has_output_associated_type: true,
                                trait_arg_is_second_arg_type: false,
                                second_arg_is_reference: false,
                                return_type_constraint: ReturnTypeConstraint::Any,
                                target_is_reference: false,
                            }
                        } else {
                            TraitImplInfo {
                                trait_path: "cpp_core::ops::BeginMut",
                                function_name: "begin_mut",
                                is_unsafe: true,
                                is_inherent: true,
                                self_arg_kind: RustFunctionSelfArgKind::MutRef,
                                has_output_associated_type: true,
                                trait_arg_is_second_arg_type: false,
                                second_arg_is_reference: false,
                                return_type_constraint: ReturnTypeConstraint::Any,
                                target_is_reference: false,
                            }
                        };
                        return Some(info);
                    }
                    "end" => {
                        let info = if member.is_const {
                            TraitImplInfo {
                                trait_path: "cpp_core::ops::End",
                                function_name: "end",
                                is_unsafe: true,
                                is_inherent: true,
                                self_arg_kind: RustFunctionSelfArgKind::ConstRef,
                                has_output_associated_type: true,
                                trait_arg_is_second_arg_type: false,
                                second_arg_is_reference: false,
                                return_type_constraint: ReturnTypeConstraint::Any,
                                target_is_reference: false,
                            }
                        } else {
                            TraitImplInfo {
                                trait_path: "cpp_core::ops::EndMut",
                                function_name: "end_mut",
                                is_unsafe: true,
                                is_inherent: true,
                                self_arg_kind: RustFunctionSelfArgKind::MutRef,
                                has_output_associated_type: true,
                                trait_arg_is_second_arg_type: false,
                                second_arg_is_reference: false,
                                return_type_constraint: ReturnTypeConstraint::Any,
                                target_is_reference: false,
                            }
                        };
                        return Some(info);
                    }
                    "data" => {
                        if function.return_type.is_pointer() {
                            let info = if member.is_const {
                                TraitImplInfo {
                                    trait_path: "cpp_core::vector_ops::Data",
                                    function_name: "data",
                                    is_unsafe: true,
                                    is_inherent: true,
                                    self_arg_kind: RustFunctionSelfArgKind::ConstRef,
                                    has_output_associated_type: true,
                                    trait_arg_is_second_arg_type: false,
                                    second_arg_is_reference: false,
                                    return_type_constraint: ReturnTypeConstraint::Any,
                                    target_is_reference: false,
                                }
                            } else {
                                TraitImplInfo {
                                    trait_path: "cpp_core::vector_ops::DataMut",
                                    function_name: "data_mut",
                                    is_unsafe: true,
                                    is_inherent: true,
                                    self_arg_kind: RustFunctionSelfArgKind::MutRef,
                                    has_output_associated_type: true,
                                    trait_arg_is_second_arg_type: false,
                                    second_arg_is_reference: false,
                                    return_type_constraint: ReturnTypeConstraint::Any,
                                    target_is_reference: false,
                                }
                            };
                            return Some(info);
                        }
                    }
                    "size" => {
                        return Some(TraitImplInfo {
                            trait_path: "cpp_core::vector_ops::Size",
                            function_name: "size",
                            is_unsafe: true,
                            is_inherent: true,
                            self_arg_kind: RustFunctionSelfArgKind::ConstRef,
                            has_output_associated_type: false,
                            trait_arg_is_second_arg_type: false,
                            second_arg_is_reference: false,
                            return_type_constraint: ReturnTypeConstraint::Usize,
                            target_is_reference: false,
                        });
                    }
                    _ => {}
                }
            }
        }
        None
    }
}

#[derive(Debug)]
struct TraitTypes {
    target_type: RustType,
    trait_type: RustCommonType,
}

impl From<&RustTraitImpl> for TraitTypes {
    fn from(trait_impl: &RustTraitImpl) -> Self {
        Self {
            target_type: trait_impl.target_type.clone(),
            trait_type: trait_impl.trait_type.clone(),
        }
    }
}

fn check_trait_impl_uniqueness(
    trait_types: &[TraitTypes],
    target_type: &RustType,
    trait_type: &RustCommonType,
) -> Result<()> {
    let conflict = trait_types.iter().find(|tt| {
        tt.target_type.can_be_same_as(target_type) && tt.trait_type.can_be_same_as(trait_type)
    });
    if let Some(conflict) = conflict {
        if &conflict.target_type == target_type && &conflict.trait_type == trait_type {
            bail!("this trait implementation already exists: {:?}", conflict);
        } else {
            bail!(
                "can't add impl {:?} for {:?} because potentially conflicting trait impl \
                 already exists: {:?}",
                trait_type,
                target_type,
                conflict
            );
        }
    }
    Ok(())
}

struct State<'b, 'a> {
    data: &'b mut ProcessorData<'a>,
    special_module_paths: HashMap<RustSpecialModuleKind, RustPath>,
}

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
                if numeric == &CppBuiltInNumericType::Bool {
                    // TODO: bool may not be safe for FFI
                    RustType::bool()
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
                        CppBuiltInNumericType::WChar => {
                            return Ok(RustType::Common(RustCommonType {
                                path: RustPath::from_good_str("cpp_core::wchar_t"),
                                generic_arguments: None,
                            }));
                        }
                        CppBuiltInNumericType::Char16 => {
                            return Ok(RustType::Common(RustCommonType {
                                path: RustPath::from_good_str("cpp_core::char16_t"),
                                generic_arguments: None,
                            }));
                        }
                        CppBuiltInNumericType::Char32 => {
                            return Ok(RustType::Common(RustCommonType {
                                path: RustPath::from_good_str("cpp_core::char32_t"),
                                generic_arguments: None,
                            }));
                        }
                        _ => bail!("unsupported numeric type: {:?}", numeric),
                    };
                    let path = RustPath::from_good_str("std::os::raw").join(own_name);
                    RustType::Common(RustCommonType {
                        path,
                        generic_arguments: None,
                    })
                }
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
                let name = format!("{}{}", letter, bits);
                RustType::Primitive(name)
            }
            CppType::PointerSizedInteger { is_signed, .. } => {
                let name = if *is_signed { "isize" } else { "usize" };
                RustType::Primitive(name.into())
            }
            CppType::Enum { path } | CppType::Class(path) => {
                let rust_item = self.find_wrapper_type(path)?;
                let path = rust_item
                    .item
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
                let pointer = RustType::FunctionPointer(RustFunctionPointerType {
                    arguments: rust_args,
                    return_type: Box::new(rust_return_type),
                });
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
        qt_core_path(self.data.config.crate_properties().name())
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

    fn is_type_deletable(&self, ffi_type: &CppType, checks: &CppChecks) -> Result<bool> {
        debug!(
            "is_type_deletable(ffi_type={:?}, checks={:?}",
            ffi_type, checks
        );
        let class_type = ffi_type.pointer_like_to_target()?;
        let class_path = if let CppType::Class(path) = class_type {
            path
        } else {
            bail!("not a pointer to class");
        };

        let destructor = if let Some(r) = self
            .data
            .db
            .all_cpp_items()
            .filter_map(|item| item.filter_map(|item| item.as_function_ref()))
            .find(|f| {
                f.item.is_destructor() && f.item.class_path_parts().unwrap() == class_path.items()
            }) {
            r
        } else {
            debug!("    not deletable (destructor not found)");
            return Ok(false);
        };

        let ffi_item = if let Some(r) = self
            .data
            .db
            .all_ffi_items()
            .find(|i| i.source_id.as_ref() == Some(&destructor.id))
        {
            r
        } else {
            debug!("    not deletable (ffi item for destructor not found)");
            return Ok(false);
        };

        let destructor_checks = self.data.db.cpp_checks(&ffi_item.id)?;
        debug!("    destructor checks: {:?}", destructor_checks);

        let is_deletable =
            !destructor_checks.is_empty() && destructor_checks.is_always_success_for(checks);

        debug!("    is_type_deletable = {}", is_deletable);
        Ok(is_deletable)
    }

    /// Generates `CompleteType` from `CppFfiType`, adding
    /// Rust API type, Rust FFI type and conversion between them.
    #[allow(clippy::collapsible_if)]
    fn rust_final_type(
        &self,
        cpp_ffi_type: &CppFfiType,
        argument_meaning: &CppFfiArgumentMeaning,
        allocation_place: ReturnValueAllocationPlace,
        checks: Option<&CppChecks>,
    ) -> Result<RustFinalType> {
        let rust_ffi_type = self.ffi_type_to_rust_ffi_type(cpp_ffi_type.ffi_type())?;
        let mut api_to_ffi_conversion = RustToFfiTypeConversion::None;
        if let RustType::PointerLike { .. } = &rust_ffi_type {
            if let CppToFfiTypeConversion::ValueToPointer { .. } = cpp_ffi_type.conversion() {
                if argument_meaning == &CppFfiArgumentMeaning::ReturnValue {
                    match allocation_place {
                        ReturnValueAllocationPlace::Stack => {
                            api_to_ffi_conversion = RustToFfiTypeConversion::ValueToPtr;
                        }
                        ReturnValueAllocationPlace::Heap => {
                            let is_deletable = if let Some(checks) = checks {
                                self.is_type_deletable(cpp_ffi_type.ffi_type(), checks)?
                            } else {
                                true
                            };

                            if is_deletable {
                                api_to_ffi_conversion = RustToFfiTypeConversion::CppBoxToPtr;
                            } else {
                                api_to_ffi_conversion = RustToFfiTypeConversion::UtilsRefToPtr {
                                    force_api_is_const: None,
                                };
                            }
                        }
                        ReturnValueAllocationPlace::NotApplicable => {
                            bail!("NotApplicable conflicts with ValueToPointer");
                        }
                    }
                } else {
                    // argument passed by value is represented as a reference on Rust side
                    api_to_ffi_conversion = RustToFfiTypeConversion::ImplCastInto(Box::new(
                        RustToFfiTypeConversion::UtilsRefToPtr {
                            force_api_is_const: None,
                        },
                    ));
                }
            } else {
                if argument_meaning == &CppFfiArgumentMeaning::This {
                    api_to_ffi_conversion = RustToFfiTypeConversion::RefToPtr {
                        force_api_is_const: None,
                        lifetime: None,
                    };
                } else if argument_meaning == &CppFfiArgumentMeaning::ReturnValue {
                    api_to_ffi_conversion =
                        if let CppToFfiTypeConversion::ReferenceToPointer { .. } =
                            cpp_ffi_type.conversion()
                        {
                            RustToFfiTypeConversion::UtilsRefToPtr {
                                force_api_is_const: None,
                            }
                        } else {
                            RustToFfiTypeConversion::UtilsPtrToPtr {
                                force_api_is_const: None,
                            }
                        };
                } else {
                    // argument
                    api_to_ffi_conversion =
                        if let CppToFfiTypeConversion::ReferenceToPointer { .. } =
                            cpp_ffi_type.conversion()
                        {
                            RustToFfiTypeConversion::ImplCastInto(Box::new(
                                RustToFfiTypeConversion::UtilsRefToPtr {
                                    force_api_is_const: None,
                                },
                            ))
                        } else {
                            RustToFfiTypeConversion::ImplCastInto(Box::new(
                                RustToFfiTypeConversion::UtilsPtrToPtr {
                                    force_api_is_const: None,
                                },
                            ))
                        };
                }
            }
        }
        if cpp_ffi_type.conversion() == &CppToFfiTypeConversion::QFlagsToInt {
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
            let rust_enum_path = rust_enum_type.item.path().ok_or_else(|| {
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
    fn generate_ffi_function(&self, data: &CppFfiFunction) -> Result<RustFunction> {
        let mut args = Vec::new();
        for (ffi_index, arg) in data.arguments.iter().enumerate() {
            let rust_type = self.ffi_type_to_rust_ffi_type(arg.argument_type.ffi_type())?;
            args.push(RustFunctionArgument {
                name: sanitize_rust_identifier(&arg.name, false),
                argument_type: RustFinalType::new(rust_type, RustToFfiTypeConversion::None)?,
                ffi_index,
            });
        }
        let return_type = self.ffi_type_to_rust_ffi_type(data.return_type.ffi_type())?;
        let function = RustFunction {
            is_public: true,
            return_type: RustFinalType::new(return_type, RustToFfiTypeConversion::None)?,
            path: self.generate_rust_path(&data.path, NameType::FfiFunction)?,
            kind: RustFunctionKind::FfiFunction,
            arguments: args,
            is_unsafe: false,
        };
        Ok(function)
    }

    fn fix_cast_function(
        mut unnamed_function: UnnamedRustFunction,
        _cast: &CppCast,
        is_const: bool,
    ) -> Result<UnnamedRustFunction> {
        let force_const = if is_const { Some(true) } else { None };
        unnamed_function.return_type = RustFinalType::new(
            unnamed_function.return_type.ffi_type().clone(),
            RustToFfiTypeConversion::UtilsPtrToPtr {
                force_api_is_const: force_const,
            },
        )?;

        unnamed_function.arguments[0].argument_type = RustFinalType::new(
            unnamed_function.arguments[0]
                .argument_type
                .ffi_type()
                .clone(),
            RustToFfiTypeConversion::UtilsPtrToPtr {
                force_api_is_const: force_const,
            },
        )?;
        //unnamed_function.arguments[0].name = "self".to_string();
        Ok(unnamed_function)
    }

    fn process_operator_as_trait_impl(
        unnamed_function: UnnamedRustFunction,
        operator_info: TraitImplInfo,
        crate_name: &str,
        trait_types: &[TraitTypes],
    ) -> Result<RustTraitImpl> {
        let trait_path = RustPath::from_good_str(operator_info.trait_path);

        let self_type = unnamed_function
            .arguments
            .get(0)
            .ok_or_else(|| err_msg("no arguments"))?
            .argument_type
            .ffi_type()
            .clone();

        let self_value_type = self_type.pointer_like_to_target()?;

        let is_self_const = match operator_info.self_arg_kind {
            RustFunctionSelfArgKind::ConstRef | RustFunctionSelfArgKind::Value => true,
            RustFunctionSelfArgKind::MutRef => false,
            RustFunctionSelfArgKind::None => unreachable!(),
        };

        let target_type = if operator_info.target_is_reference {
            RustType::new_reference(true, self_value_type.clone())
        } else {
            self_value_type.clone()
        };

        let trait_args;
        let other_type;

        if operator_info.trait_arg_is_second_arg_type {
            let mut other_type1 = unnamed_function
                .arguments
                .get(1)
                .ok_or_else(|| err_msg("not enough arguments"))?
                .argument_type
                .clone();

            if let RustToFfiTypeConversion::ImplCastInto(conversion) = other_type1.conversion() {
                other_type1 =
                    RustFinalType::new(other_type1.ffi_type().clone(), (**conversion).clone())?;
            }
            trait_args = Some(vec![other_type1.api_type().clone()]);
            other_type = Some(other_type1);
        } else {
            other_type = None;
            trait_args = None;
        }

        let trait_type = RustCommonType {
            path: trait_path.clone(),
            generic_arguments: trait_args,
        };

        check_trait_impl_uniqueness(trait_types, &target_type, &trait_type)?;

        let parent_path = if let RustType::Common(RustCommonType { path, .. }) = self_value_type {
            let type_crate_name = path
                .crate_name()
                .ok_or_else(|| err_msg("common type must have crate name"))?;
            if type_crate_name != crate_name {
                bail!("self type is outside current crate");
            }
            path.parent()?
        } else {
            bail!("self type is not Common");
        };

        let associated_types = if operator_info.has_output_associated_type {
            let output = RustTraitAssociatedType {
                name: "Output".into(),
                value: unnamed_function.return_type.api_type().clone(),
            };
            vec![output]
        } else {
            Vec::new()
        };

        let mut function = unnamed_function.with_path(trait_path.join(operator_info.function_name));
        function.is_unsafe = operator_info.is_unsafe;
        function.arguments[0].argument_type = RustFinalType::new(
            function.arguments[0].argument_type.ffi_type().clone(),
            RustToFfiTypeConversion::RefToPtr {
                force_api_is_const: Some(is_self_const),
                lifetime: None,
            },
        )?;
        function.arguments[0].name = "self".to_string();
        if let Some(other_type) = other_type {
            function.arguments[1].argument_type = other_type;
        }

        if operator_info.second_arg_is_reference {
            let other_arg = &mut function.arguments[1].argument_type;
            *other_arg = RustFinalType::new(
                other_arg.ffi_type().clone(),
                RustToFfiTypeConversion::RefTo(Box::new(other_arg.conversion().clone())),
            )?;
        }

        match operator_info.return_type_constraint {
            ReturnTypeConstraint::Any => {}
            ReturnTypeConstraint::Unit => {
                if function.return_type.api_type() != &RustType::unit() {
                    function.return_type = RustFinalType::new(
                        function.return_type.ffi_type().clone(),
                        RustToFfiTypeConversion::UnitToAnything,
                    )?;
                }
            }
            ReturnTypeConstraint::Usize => {
                if function.return_type.api_type() != &RustType::Primitive("usize".into()) {
                    function.return_type = RustFinalType::new(
                        function.return_type.ffi_type().clone(),
                        RustToFfiTypeConversion::AsCast {
                            api_type: RustType::Primitive("usize".into()),
                        },
                    )?;
                }
            }
            ReturnTypeConstraint::Bool => {
                if function.return_type.api_type() != &RustType::bool() {
                    bail!("return type is not bool");
                }
            }
        }

        Ok(RustTraitImpl {
            target_type,
            parent_path,
            trait_type,
            associated_types,
            functions: vec![function],
            extra_kind: RustTraitImplExtraKind::Normal,
        })
    }

    fn process_destructor(
        unnamed_function: UnnamedRustFunction,
        allocation_place: ReturnValueAllocationPlace,
    ) -> Result<RustTraitImpl> {
        if unnamed_function.arguments.len() != 1 {
            bail!("destructor must have one argument");
        }
        let target_type = unnamed_function.arguments[0]
            .argument_type
            .api_type()
            .pointer_like_to_target()?;

        let parent_path = if let RustType::Common(RustCommonType { path, .. }) = &target_type {
            path.parent()
                .expect("destructor argument path must have parent")
        } else {
            bail!("can't get parent for target type: {:?}", target_type);
        };

        let function_name;
        let trait_path;
        let is_unsafe;
        match allocation_place {
            ReturnValueAllocationPlace::Stack => {
                function_name = "drop";
                trait_path = RustPath::from_good_str("std::ops::Drop");
                is_unsafe = false;
            }
            ReturnValueAllocationPlace::Heap => {
                function_name = "delete";
                trait_path = RustPath::from_good_str("cpp_core::CppDeletable");
                is_unsafe = true;
            }
            ReturnValueAllocationPlace::NotApplicable => {
                bail!("invalid allocation_place for destructor");
            }
        }
        let mut function = unnamed_function.with_path(trait_path.join(function_name));
        function.is_unsafe = is_unsafe;

        Ok(RustTraitImpl {
            target_type,
            parent_path,
            trait_type: RustCommonType {
                path: trait_path,
                generic_arguments: None,
            },
            associated_types: Vec::new(),
            functions: vec![function],
            extra_kind: RustTraitImplExtraKind::Normal,
        })
    }

    fn process_cast(
        mut unnamed_function: UnnamedRustFunction,
        cast: &CppCast,
        trait_types: &[TraitTypes],
    ) -> Result<Vec<RustTraitImpl>> {
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
                    trait_path = RustPath::from_good_str("cpp_core::StaticDowncast");
                    derived_type = to_type;
                    cast_function_name = "static_downcast";
                    cast_function_name_mut = "static_downcast_mut";
                } else {
                    trait_path = RustPath::from_good_str("cpp_core::StaticUpcast");
                    derived_type = from_type;
                    cast_function_name = "static_upcast";
                    cast_function_name_mut = "static_upcast_mut";
                }
            }
            CppCast::Dynamic => {
                trait_path = RustPath::from_good_str("cpp_core::DynamicCast");
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
        results.push(RustTraitImpl {
            target_type: target_type.clone(),
            parent_path: parent_path.clone(),
            trait_type: RustCommonType {
                path: trait_path,
                generic_arguments: Some(vec![to_type_value.clone()]),
            },
            associated_types: Vec::new(),
            functions: vec![cast_function, cast_function_mut],
            extra_kind: RustTraitImplExtraKind::Normal,
        });

        if cast.is_first_static_cast() && !cast.is_unsafe_static_cast() {
            let make_type_ref = |type1: &mut RustFinalType, is_const: bool| -> Result<()> {
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
            let deref_trait_type = RustCommonType {
                path: deref_trait_path.clone(),
                generic_arguments: None,
            };
            match check_trait_impl_uniqueness(trait_types, &target_type, &deref_trait_type) {
                Ok(_) => {
                    let mut deref_function =
                        fixed_function.with_path(deref_trait_path.join("deref"));
                    deref_function.is_unsafe = false;
                    make_type_ref(&mut deref_function.return_type, true)?;
                    make_type_ref(&mut deref_function.arguments[0].argument_type, true)?;
                    deref_function.arguments[0].name = "self".into();
                    results.push(RustTraitImpl {
                        target_type: target_type.clone(),
                        parent_path: parent_path.clone(),
                        trait_type: deref_trait_type,
                        associated_types: vec![RustTraitAssociatedType {
                            name: "Target".to_string(),
                            value: to_type_value,
                        }],
                        functions: vec![deref_function],
                        extra_kind: RustTraitImplExtraKind::Deref,
                    });

                    let deref_mut_trait_path = RustPath::from_good_str("std::ops::DerefMut");
                    let mut deref_mut_function =
                        fixed_function_mut.with_path(deref_mut_trait_path.join("deref_mut"));
                    deref_mut_function.is_unsafe = false;
                    make_type_ref(&mut deref_mut_function.return_type, false)?;
                    make_type_ref(&mut deref_mut_function.arguments[0].argument_type, false)?;
                    deref_mut_function.arguments[0].name = "self".into();
                    results.push(RustTraitImpl {
                        target_type,
                        parent_path,
                        trait_type: RustCommonType {
                            path: deref_mut_trait_path,
                            generic_arguments: None,
                        },
                        associated_types: Vec::new(),
                        functions: vec![deref_mut_function],
                        extra_kind: RustTraitImplExtraKind::DerefMut,
                    });
                }
                Err(err) => {
                    debug!("not implementing Deref: {}", err);
                }
            }
        }

        Ok(results)
    }

    fn convert_callbacks_to_closure(
        &self,
        id: &ItemId,
        function: &mut UnnamedRustFunction,
        checks: &CppChecks,
    ) -> Result<()> {
        let callback_type = if let Some(t) = detect_callback_function(function) {
            t.clone()
        } else {
            return Ok(());
        };
        println!("OK!!! {:?}", callback_type);

        let wrapper = self
            .data
            .db
            .source_ffi_item(id)?
            .ok_or_else(|| err_msg("source ffi item not found"))?
            .item
            .as_slot_wrapper_ref()
            .ok_or_else(|| err_msg("invalid source ffi item type"))?;

        println!("OK1 wrapper {:?}", wrapper);
        let closure_arguments = wrapper.arguments.iter().map_if_ok(|arg| {
            self.rust_final_type(
                arg,
                // closure argument should be handled in the same way
                // as return type (value is produced behind FFI)
                &CppFfiArgumentMeaning::ReturnValue,
                ReturnValueAllocationPlace::NotApplicable,
                Some(&checks),
            )
        })?;
        println!("OK2 closure_args {:?}", closure_arguments);
        let closure_return_type = self.rust_final_type(
            &CppFfiType::void(),
            // TODO: not sure about the meaning.
            &CppFfiArgumentMeaning::Argument(0),
            ReturnValueAllocationPlace::NotApplicable,
            Some(&checks),
        )?;

        function.arguments.drain(function.arguments.len() - 2..);
        let arg = function
            .arguments
            .last_mut()
            .expect("function must have enough args");
        arg.argument_type = RustFinalType::new(
            arg.argument_type.ffi_type().clone(),
            RustToFfiTypeConversion::ClosureToCallback(Box::new(RustClosureToCallbackConversion {
                closure_arguments,
                closure_return_type,
            })),
        )?;

        Ok(())
    }

    /// Converts one function to a `RustSingleMethod`.
    fn process_rust_function(
        &self,
        item: DbItem<&CppFfiFunction>,
        checks: &CppChecks,
        trait_types: &[TraitTypes],
    ) -> Result<Vec<ProcessedFfiItem>> {
        let function = item.item;
        let rust_ffi_function = self.generate_ffi_function(&function)?;
        let ffi_function_path = rust_ffi_function.path.clone();
        let mut results = vec![ProcessedFfiItem::Item(RustItem::Function(
            rust_ffi_function,
        ))];

        let mut arguments = Vec::new();
        for (arg_index, arg) in function.arguments.iter().enumerate() {
            if arg.meaning != CppFfiArgumentMeaning::ReturnValue {
                let arg_type = self.rust_final_type(
                    &arg.argument_type,
                    &arg.meaning,
                    function.allocation_place,
                    Some(checks),
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
        let mut return_type = if let Some(arg) = function
            .arguments
            .iter()
            .find(|arg| arg.meaning == CppFfiArgumentMeaning::ReturnValue)
        {
            // an argument has return value meaning, so
            // FFI return type must be void
            assert_eq!(function.return_type, CppFfiType::void());

            self.rust_final_type(
                &arg.argument_type,
                &arg.meaning,
                function.allocation_place,
                Some(checks),
            )?
        } else {
            // none of the arguments has return value meaning,
            // so FFI return value must be used
            self.rust_final_type(
                &function.return_type,
                &CppFfiArgumentMeaning::ReturnValue,
                function.allocation_place,
                Some(checks),
            )?
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
                        function.path.to_cpp_pseudo_code()
                    );
                    "static".to_string()
                } else {
                    "l0".to_string()
                };
                return_type = return_type.with_lifetime(return_lifetime)?;
            }
        }

        let mut unnamed_function = UnnamedRustFunction {
            is_public: true,
            arguments: arguments.clone(),
            return_type,
            kind: RustFunctionKind::FfiWrapper(RustFfiWrapperData { ffi_function_path }),
            is_unsafe: true,
        };
        self.convert_callbacks_to_closure(&item.id, &mut unnamed_function, checks)?;

        let cpp_item = self
            .data
            .db
            .source_cpp_item(&item.id)?
            .ok_or_else(|| err_msg("source cpp item not found"))?
            .item;

        if let CppFfiFunctionKind::Function = &function.kind {
            let cpp_function = cpp_item
                .as_function_ref()
                .ok_or_else(|| err_msg("invalid source cpp item type"))?;

            if cpp_function.is_destructor() {
                let item = State::process_destructor(unnamed_function, function.allocation_place)?;
                results.push(ProcessedFfiItem::Item(RustItem::TraitImpl(item)));
                return Ok(results);
            }
            if let Some(cast) = &cpp_function.cast {
                let impls = State::process_cast(unnamed_function, cast, trait_types)?;
                results.extend(
                    impls
                        .into_iter()
                        .map(|x| ProcessedFfiItem::Item(RustItem::TraitImpl(x))),
                );
                return Ok(results);
            }
            if cpp_function.operator.as_ref() == Some(&CppOperator::NotEqualTo) {
                bail!("NotEqualTo is not needed in public API because PartialEq is used");
            }
            if let Some(operator_info) = TraitImplInfo::new(cpp_function) {
                match State::process_operator_as_trait_impl(
                    unnamed_function.clone(),
                    operator_info,
                    self.data.db.crate_name(),
                    trait_types,
                ) {
                    Ok(item) => {
                        results.push(ProcessedFfiItem::Item(RustItem::TraitImpl(item)));
                        if !operator_info.is_inherent {
                            return Ok(results);
                        }
                    }
                    Err(err) => {
                        debug!("failed to convert operator to trait: {}", err);
                        debug!("function: {:?}", function);
                        debug!("rust function: {:?}", unnamed_function);
                    }
                }
            }
        }

        let cpp_path = cpp_item
            .path()
            .ok_or_else(|| err_msg("cpp item (function or field) expected to have a path"))?;

        if let CppFfiFunctionKind::Function = &function.kind {
            let cpp_function = cpp_item
                .as_function_ref()
                .ok_or_else(|| err_msg("invalid source cpp item type"))?;

            if cpp_function.is_operator() {
                let arg0 = unnamed_function
                    .arguments
                    .get_mut(0)
                    .ok_or_else(|| err_msg("no arguments"))?;

                if arg0.name != "self" {
                    if let Ok(type1) = arg0.argument_type.ffi_type().pointer_like_to_target() {
                        if let RustType::Common(type1) = type1 {
                            if type1.path.crate_name() == Some(self.data.db.crate_name()) {
                                arg0.name = "self".into();
                                arg0.argument_type = RustFinalType::new(
                                    arg0.argument_type.ffi_type().clone(),
                                    RustToFfiTypeConversion::RefToPtr {
                                        force_api_is_const: None,
                                        lifetime: None,
                                    },
                                )?;

                                let name = self
                                    .special_function_rust_name(item.clone(), &type1.path)?
                                    .ok_or_else(|| err_msg("operator must have special name"))?;
                                results.push(ProcessedFfiItem::Function(FunctionWithDesiredPath {
                                    function: unnamed_function,
                                    desired_path: type1.path.join(name),
                                }));
                                return Ok(results);
                            }
                        }
                    }
                }
            }
        }

        let desired_path = self.generate_rust_path(cpp_path, NameType::ApiFunction(item))?;
        results.push(ProcessedFfiItem::Function(FunctionWithDesiredPath {
            function: unnamed_function,
            desired_path,
        }));
        Ok(results)
    }

    fn find_wrapper_type(&self, cpp_path: &CppPath) -> Result<DbItem<&RustItem>> {
        self.data
            .db
            .find_rust_items_for_cpp_path(cpp_path, true)?
            .find(|item| item.item.is_wrapper_type())
            .ok_or_else(|| {
                format_err!("no Rust type wrapper for {}", cpp_path.to_cpp_pseudo_code())
            })
    }

    fn get_path_scope(
        &self,
        parent_path: &CppPath,
        name_type: NameType<'_>,
    ) -> Result<RustPathScope> {
        trace!("get_path_scope({:?}, {:?})", parent_path, name_type);
        if let Some(hook) = self.data.config.rust_path_scope_hook() {
            if let Some(strategy) = hook(parent_path)? {
                return Ok(strategy);
            }
        }

        let allow_module_for_nested;
        let allow_wrapper_type;
        match name_type {
            NameType::Type { .. } | NameType::Module { .. } => {
                allow_module_for_nested = true;
                allow_wrapper_type = false;
            }
            _ => {
                allow_module_for_nested = false;
                allow_wrapper_type = true;
            }
        };

        let mut rust_items = match self
            .data
            .db
            .find_rust_items_for_cpp_path(parent_path, false)
        {
            Ok(r) => r,
            Err(err) => match name_type {
                NameType::Type {
                    is_from_other_crate,
                }
                | NameType::Module {
                    is_from_other_crate,
                } if is_from_other_crate => {
                    return Ok(self.default_path_scope());
                }
                _ => return Err(err),
            },
        };

        let rust_item = rust_items
            .find(|item| {
                (allow_wrapper_type && item.item.is_wrapper_type())
                    || (item.item.is_module() && !item.item.is_module_for_nested())
                    || (allow_module_for_nested && item.item.is_module_for_nested())
            })
            .ok_or_else(|| {
                format_err!(
                    "no Rust type wrapper for {}",
                    parent_path.to_cpp_pseudo_code()
                )
            })?;

        let rust_path = rust_item.item.path().ok_or_else(|| {
            format_err!(
                "rust item doesn't have rust path (parent_path = {:?})",
                parent_path
            )
        })?;

        Ok(RustPathScope {
            path: rust_path.clone(),
            prefix: None,
        })
    }

    fn default_path_scope(&self) -> RustPathScope {
        RustPathScope {
            path: RustPath {
                parts: vec![self.data.config.crate_properties().name().into()],
            },
            prefix: None,
        }
    }

    fn type_list_caption(&self, types: &[CppType], context: &RustPath) -> Result<String> {
        let mut captions = Vec::new();
        for arg in types {
            let rust_type = self.rust_final_type(
                &ffi_type(arg, CppTypeRole::NotReturnType)?,
                &CppFfiArgumentMeaning::Argument(0),
                ReturnValueAllocationPlace::NotApplicable,
                None,
            )?;
            captions.push(
                rust_type
                    .api_type()
                    .caption(context, RustTypeCaptionStrategy::LastName)?,
            );
        }
        Ok(captions.join("_"))
    }

    /// Returns method name. For class member functions, the name doesn't
    /// include class name and scope. For free functions, the name includes
    /// modules.
    fn special_function_rust_name(
        &self,
        item: DbItem<&CppFfiFunction>,
        context: &RustPath,
    ) -> Result<Option<String>> {
        let function = item.item;
        let cpp_item = self
            .data
            .db
            .source_cpp_item(&item.id)?
            .ok_or_else(|| err_msg("source cpp item not found"))?
            .item;

        let r = match &function.kind {
            CppFfiFunctionKind::Function => {
                let cpp_function = cpp_item
                    .as_function_ref()
                    .ok_or_else(|| err_msg("invalid source cpp item type"))?;

                if cpp_function.is_constructor() {
                    if cpp_function.is_copy_constructor() {
                        Some("new_copy".to_string())
                    } else {
                        Some("new".to_string())
                    }
                } else if let Some(operator) = &cpp_function.operator {
                    match operator {
                        CppOperator::Conversion(type1) => {
                            let rust_type = self.rust_final_type(
                                &ffi_type(type1, CppTypeRole::ReturnType)?,
                                &CppFfiArgumentMeaning::ReturnValue,
                                function.allocation_place,
                                None,
                            )?;
                            Some(format!(
                                "to_{}",
                                rust_type
                                    .api_type()
                                    .caption(context, RustTypeCaptionStrategy::LastName)?
                            ))
                        }
                        CppOperator::Assignment => Some("copy_from".to_string()),
                        _ => Some(operator_function_name(operator)?.to_string()),
                    }
                } else {
                    None
                }
            }
            CppFfiFunctionKind::FieldAccessor { accessor_type } => {
                let field = cpp_item
                    .as_field_ref()
                    .ok_or_else(|| err_msg("invalid source cpp item type"))?;

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

    fn cpp_path_item_to_name(
        &self,
        item: &CppPathItem,
        context: &RustPath,
        name_type: &NameType<'_>,
    ) -> Result<String> {
        if let Some(template_arguments) = &item.template_arguments {
            let captions = self.type_list_caption(template_arguments, context)?;
            if name_type.is_api_function() {
                Ok(format!("{}_{}", item.name, captions))
            } else {
                Ok(format!("{}_of_{}", item.name, captions))
            }
        } else {
            Ok(item.name.clone())
        }
    }

    fn generate_rust_path(&self, cpp_path: &CppPath, name_type: NameType<'_>) -> Result<RustPath> {
        if let Some(hook) = self.data.config.rust_path_hook() {
            if let Some(path) = hook(cpp_path, name_type.clone(), &self.data)? {
                return Ok(path);
            }
        }
        let scope = match &name_type {
            NameType::FfiFunction => RustPathScope {
                path: self.special_module_paths[&RustSpecialModuleKind::Ffi].clone(),
                prefix: None,
            },
            NameType::SizedItem => RustPathScope {
                path: self.special_module_paths[&RustSpecialModuleKind::SizedTypes].clone(),
                prefix: None,
            },
            NameType::QtSlotWrapper { .. } => {
                // crate root
                self.default_path_scope()
            }
            NameType::Type { .. }
            | NameType::Module { .. }
            | NameType::EnumValue
            | NameType::ApiFunction { .. }
            | NameType::ReceiverFunction { .. } => {
                if let Ok(parent) = cpp_path.parent() {
                    self.get_path_scope(&parent, name_type.clone())?
                } else if let NameType::ApiFunction(item) = &name_type {
                    let cpp_item = self
                        .data
                        .db
                        .source_cpp_item(&item.id)?
                        .ok_or_else(|| err_msg("source cpp item not found"))?
                        .item;

                    let is_operator = cpp_item
                        .as_function_ref()
                        .map_or(false, |f| f.is_operator());

                    if is_operator {
                        RustPathScope {
                            path: self.special_module_paths[&RustSpecialModuleKind::Ops].clone(),
                            prefix: None,
                        }
                    } else {
                        self.default_path_scope()
                    }
                } else {
                    self.default_path_scope()
                }
            }
        };

        let full_last_name = match &name_type {
            NameType::SizedItem => cpp_path
                .items()
                .iter()
                .map_if_ok(|item| self.cpp_path_item_to_name(item, &scope.path, &name_type))?
                .join("_"),
            NameType::ApiFunction(function) => {
                let s = if is_second_slot_constructor(self.data, function)? {
                    "with".to_string()
                } else if let Some(last_name_override) =
                    self.special_function_rust_name(function.clone(), &scope.path)?
                {
                    last_name_override.clone()
                } else {
                    self.cpp_path_item_to_name(cpp_path.last(), &scope.path, &name_type)?
                };
                s.to_snake_case()
            }
            NameType::ReceiverFunction { receiver_type } => {
                let name = self
                    .cpp_path_item_to_name(cpp_path.last(), &scope.path, &name_type)?
                    .to_snake_case();
                match receiver_type {
                    RustQtReceiverType::Signal => name,
                    RustQtReceiverType::Slot => format!("slot_{}", name),
                }
            }
            NameType::Type { .. } | NameType::EnumValue => {
                if cpp_path.to_templateless_string() == "std::vector" {
                    // remove allocator template argument
                    let mut path_item = cpp_path.last().clone();
                    if let Some(args) = &mut path_item.template_arguments {
                        args.pop();
                    }
                    self.cpp_path_item_to_name(&path_item, &scope.path, &name_type)?
                        .to_class_case()
                } else {
                    self.cpp_path_item_to_name(&cpp_path.last(), &scope.path, &name_type)?
                        .to_class_case()
                }
            }
            NameType::Module { .. } => self
                .cpp_path_item_to_name(&cpp_path.last(), &scope.path, &name_type)?
                .to_snake_case(),
            NameType::FfiFunction => cpp_path.last().name.clone(),
            NameType::QtSlotWrapper { signal_arguments } => {
                if signal_arguments.is_empty() {
                    "Slot".to_string()
                } else {
                    let captions = self.type_list_caption(signal_arguments, &scope.path)?;
                    format!("SlotOf_{}", captions).to_class_case()
                }
            }
        };

        if name_type == NameType::FfiFunction {
            let rust_path = scope.apply(&full_last_name);
            if self.data.db.find_rust_item(&rust_path).is_some() {
                bail!("ffi function path already taken: {:?}", rust_path);
            }
            return Ok(rust_path);
        }

        let sanitized_name = sanitize_rust_identifier(&full_last_name, name_type.is_module());
        let rust_path = scope.apply(&sanitized_name);

        if name_type.is_api_function() {
            Ok(rust_path)
        } else {
            Ok(self.data.db.make_unique_rust_path(&rust_path))
        }
    }

    fn process_ffi_item(
        &self,
        ffi_item: DbItem<&CppFfiItem>,
        checks: &CppChecks,
        trait_types: &[TraitTypes],
    ) -> Result<Vec<ProcessedFfiItem>> {
        match ffi_item.item {
            CppFfiItem::Function(_) => self.process_rust_function(
                ffi_item.map(|i| i.as_function_ref().unwrap()),
                checks,
                trait_types,
            ),
            CppFfiItem::QtSlotWrapper(_) => {
                bail!("slot wrappers do not need to be processed here");
            }
        }
    }

    #[allow(clippy::useless_let_if_seq)]
    fn process_cpp_class(&self, item: DbItem<&CppTypeDeclaration>) -> Result<Vec<RustItem>> {
        trace!("process_cpp_class: {:?}", item);
        let data = item.item;

        // TODO: do something about `QUrlTwoFlags<T1, T2>`
        if is_qflags(&data.path) {
            let argument = &data.path.last().template_arguments.as_ref().unwrap()[0];
            if !argument.is_template_parameter() {
                if let CppType::Enum { path } = &argument {
                    let rust_type = self.find_wrapper_type(path)?;
                    let rust_type_path = rust_type
                        .item
                        .path()
                        .expect("enum rust item must have path");
                    let rust_item = RustItem::ExtraImpl(RustExtraImpl {
                        parent_path: rust_type_path.parent()?,
                        kind: RustExtraImplKind::FlagEnum(RustFlagEnumImpl {
                            enum_path: rust_type_path.clone(),
                        }),
                    });
                    return Ok(vec![rust_item]);
                }
            }
        }

        let mut qt_slot_wrapper = None;
        if let Some(source_ffi_item) = self.data.db.source_ffi_item(&item.id)? {
            if let Some(item) = source_ffi_item.filter_map(|i| i.as_slot_wrapper_ref()) {
                qt_slot_wrapper = Some(item);
            }
        }

        let is_from_other_crate = item
            .source_id
            .map_or(false, |id| id.crate_name() != self.data.db.crate_name());

        let public_name_type = if let Some(wrapper) = &qt_slot_wrapper {
            NameType::QtSlotWrapper {
                signal_arguments: &wrapper.item.signal_arguments,
            }
        } else {
            NameType::Type {
                is_from_other_crate,
            }
        };

        let public_path = self.generate_rust_path(&data.path, public_name_type)?;

        let mut rust_items = Vec::new();

        let is_movable = false;

        let wrapper_kind;
        if is_movable {
            let internal_path = self.generate_rust_path(&data.path, NameType::SizedItem)?;

            if internal_path == public_path {
                bail!(
                    "internal path is the same as public path: {:?}",
                    internal_path
                );
            }

            let internal_rust_item = RustItem::Struct(RustStruct {
                path: internal_path.clone(),
                kind: RustStructKind::SizedType(RustSizedType {
                    cpp_path: data.path.clone(),
                }),
                is_public: true,
                raw_slot_wrapper_data: None,
            });

            rust_items.push(internal_rust_item);

            wrapper_kind = RustWrapperTypeKind::MovableClassWrapper {
                sized_type_path: internal_path,
            };
        } else {
            wrapper_kind = RustWrapperTypeKind::ImmovableClassWrapper;
        }

        let nested_types_path = self.generate_rust_path(
            &data.path,
            NameType::Module {
                is_from_other_crate,
            },
        )?;

        let nested_types_rust_item = RustItem::Module(RustModule {
            is_public: true,
            path: nested_types_path,
            kind: RustModuleKind::CppNestedTypes,
        });
        rust_items.push(nested_types_rust_item);

        let raw_slot_wrapper_data;
        if let Some(wrapper) = qt_slot_wrapper {
            let arg_types = wrapper
                .item
                .arguments
                .iter()
                .map_if_ok(|t| self.ffi_type_to_rust_ffi_type(t.ffi_type()))?;

            let impl_item = RustItem::ExtraImpl(RustExtraImpl {
                parent_path: public_path.parent()?,
                kind: RustExtraImplKind::RawSlotReceiver(RustRawSlotReceiver {
                    target_path: public_path.clone(),
                    arguments: RustType::Tuple(arg_types.clone()),
                }),
            });
            rust_items.push(impl_item);

            raw_slot_wrapper_data = Some(RustRawQtSlotWrapperData {
                arguments: arg_types,
            });
        } else {
            raw_slot_wrapper_data = None;
        }

        let public_rust_item = RustItem::Struct(RustStruct {
            path: public_path,
            kind: RustStructKind::WrapperType(wrapper_kind),
            is_public: true,
            raw_slot_wrapper_data,
        });
        rust_items.push(public_rust_item);
        Ok(rust_items)
    }

    fn process_cpp_item(&self, cpp_item: DbItem<&CppItem>) -> Result<Vec<RustItem>> {
        if let Some(ffi_item) = self.data.db.source_ffi_item(&cpp_item.id)? {
            if !self.data.db.cpp_checks(&ffi_item.id)?.any_success() {
                bail!("cpp checks failed");
            }
        }

        match &cpp_item.item {
            CppItem::Namespace(namespace) => {
                let rust_path = self.generate_rust_path(
                    &namespace.path,
                    NameType::Module {
                        is_from_other_crate: false,
                    },
                )?;
                let rust_item = RustItem::Module(RustModule {
                    is_public: true,
                    path: rust_path,
                    kind: RustModuleKind::CppNamespace,
                });
                Ok(vec![rust_item])
            }
            CppItem::Type(data) => match data.kind {
                CppTypeDeclarationKind::Class { .. } => {
                    self.process_cpp_class(cpp_item.map(|v| v.as_type_ref().unwrap()))
                }
                CppTypeDeclarationKind::Enum => {
                    let rust_path = self.generate_rust_path(
                        &data.path,
                        NameType::Type {
                            is_from_other_crate: false,
                        },
                    )?;
                    let rust_item = RustItem::Struct(RustStruct {
                        path: rust_path,
                        kind: RustStructKind::WrapperType(RustWrapperTypeKind::EnumWrapper),
                        is_public: true,
                        raw_slot_wrapper_data: None,
                    });

                    Ok(vec![rust_item])
                }
            },
            CppItem::EnumValue(value) => {
                let rust_path = self.generate_rust_path(&value.path, NameType::EnumValue)?;

                let rust_item = RustItem::EnumValue(RustEnumValue {
                    path: rust_path,
                    value: value.value,
                });

                Ok(vec![rust_item])
            }
            CppItem::Function(cpp_function) => {
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

                let has_source_slot_wrapper = self
                    .data
                    .db
                    .source_ffi_item(&cpp_item.id)?
                    .map_or(false, |item| item.item.is_slot_wrapper());
                if has_source_slot_wrapper {
                    // no need to add slot accessors because
                    // `AsReceiver` is implemented directly on structs
                    return Ok(Vec::new());
                }

                let original_item = self
                    .data
                    .db
                    .original_cpp_item(&cpp_item.id)?
                    .ok_or_else(|| err_msg("cpp item must have original cpp item"))?;

                if let Some(original_function) = original_item.item.as_function_ref() {
                    if original_function.arguments.len() != cpp_function.arguments.len() {
                        return Ok(Vec::new());
                    }
                }

                let function_kind = RustFunctionKind::SignalOrSlotGetter(RustSignalOrSlotGetter {});

                let path = self.generate_rust_path(
                    &cpp_function.path,
                    NameType::ReceiverFunction { receiver_type },
                )?;

                let class_type = self.find_wrapper_type(&cpp_function.path.parent()?)?;
                let self_type = RustType::PointerLike {
                    kind: RustPointerLikeTypeKind::Reference { lifetime: None },
                    is_const: true,
                    target: Box::new(RustType::Common(RustCommonType {
                        path: class_type.item.path().unwrap().clone(),
                        generic_arguments: None,
                    })),
                };

                let self_type = RustFinalType::new(self_type, RustToFfiTypeConversion::None)?;

                let return_type_path = match receiver_type {
                    RustQtReceiverType::Signal => self.qt_core_path().join("Signal"),
                    RustQtReceiverType::Slot => self.qt_core_path().join("Receiver"),
                };

                let arguments = cpp_function
                    .arguments
                    .iter()
                    .map_if_ok(|arg| -> Result<_> {
                        // TODO: rust generator shouldn't know about cpp ffi types
                        let ffi_type = ffi_type(&arg.argument_type, CppTypeRole::NotReturnType)?;
                        self.ffi_type_to_rust_ffi_type(ffi_type.ffi_type())
                    })?;

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
                };
                Ok(vec![RustItem::Function(rust_function)])
            }
            CppItem::ClassField(_) | CppItem::ClassBase(_) => {
                // only need to process FFI items
                Ok(Vec::new())
            }
        }
    }

    fn generate_crate_reexport(&mut self, crate_name: &str) -> Result<()> {
        let path = RustPath::from_parts(vec![
            self.data.config.crate_properties().name().to_string(),
            crate_name.to_string(),
        ]);

        let source = RustReexportSource::DependencyCrate {
            crate_name: crate_name.into(),
        };

        if self
            .data
            .db
            .rust_items()
            .filter_map(|item| item.item.as_reexport_ref())
            .any(|item| item.source == source)
        {
            // already created
            return Ok(());
        }

        let rust_item = RustItem::Reexport(RustReexport {
            path,
            source,
            target: RustPath::from_parts(vec![crate_name.to_string()]),
        });
        self.add_rust_item(None, rust_item)?;
        Ok(())
    }

    fn add_rust_item(
        &mut self,
        source_id: Option<ItemId>,
        mut item: RustItem,
    ) -> Result<Option<ItemId>> {
        if let Some(hook) = self.data.config.rust_item_hook() {
            hook(&mut item, &self.data)?;
        }
        self.data.db.add_rust_item(source_id, item)
    }

    fn generate_special_module(&mut self, kind: RustSpecialModuleKind) -> Result<()> {
        let crate_name = self.data.config.crate_properties().name().to_string();
        let rust_path_parts = match kind {
            RustSpecialModuleKind::CrateRoot => vec![crate_name],
            RustSpecialModuleKind::Ffi => vec![crate_name, "__ffi".to_string()],
            RustSpecialModuleKind::Ops => vec![crate_name, "ops".to_string()],
            RustSpecialModuleKind::SizedTypes => vec![crate_name, "__sized_types".to_string()],
        };
        let rust_path = RustPath::from_parts(rust_path_parts);

        let rust_item = RustItem::Module(RustModule {
            is_public: match kind {
                RustSpecialModuleKind::CrateRoot | RustSpecialModuleKind::Ops => true,
                RustSpecialModuleKind::Ffi | RustSpecialModuleKind::SizedTypes => false,
            },
            path: rust_path.clone(),
            kind: RustModuleKind::Special(kind),
        });
        let new_id = self.add_rust_item(None, rust_item)?;
        let real_path = if new_id.is_some() {
            rust_path
        } else {
            self.data
                .db
                .rust_items()
                .filter_map(|i| i.item.as_module_ref())
                .find(|module| module.kind == RustModuleKind::Special(kind))
                .ok_or_else(|| err_msg("add failed but module not found"))?
                .path
                .clone()
        };
        self.special_module_paths.insert(kind, real_path);
        Ok(())
    }

    fn process_cpp_items(&mut self) -> Result<()> {
        let mut processed_ids = HashSet::new();
        let all_cpp_item_ids = self.data.db.cpp_item_ids().collect_vec();
        loop {
            let mut any_processed = false;
            for cpp_item_id in all_cpp_item_ids.clone() {
                if processed_ids.contains(&cpp_item_id) {
                    continue;
                }

                let cpp_item = self.data.db.cpp_item(&cpp_item_id)?;
                if let Ok(rust_items) = self.process_cpp_item(cpp_item) {
                    for rust_item in rust_items {
                        self.add_rust_item(Some(cpp_item_id.clone()), rust_item)?;
                    }
                    processed_ids.insert(cpp_item_id);
                    any_processed = true;
                }
            }

            if !any_processed {
                break;
            }
        }

        for cpp_item_id in all_cpp_item_ids {
            let cpp_item = self.data.db.cpp_item(&cpp_item_id)?;
            if let Err(err) = self.process_cpp_item(cpp_item.clone()) {
                debug!("failed to process cpp item: {}: {}", &cpp_item.item, err);
                print_trace(&err, Some(log::Level::Trace));
            }
        }
        Ok(())
    }

    fn process_ffi_items(
        &mut self,
    ) -> Result<BTreeMap<RustPath, Vec<ItemWithSource<FunctionWithDesiredPath>>>> {
        let mut grouped_functions = BTreeMap::<_, Vec<_>>::new();
        let mut trait_types = self
            .data
            .db
            .rust_items()
            .filter_map(|item| item.item.as_trait_impl_ref())
            .map(TraitTypes::from)
            .collect_vec();

        for ffi_item_id in self.data.db.ffi_item_ids().collect_vec() {
            let ffi_item = self.data.db.ffi_item(&ffi_item_id)?;
            let checks = self.data.db.cpp_checks(&ffi_item_id)?;
            if !checks.any_success() {
                debug!(
                    "skipping ffi item with failed checks: {}",
                    ffi_item.item.short_text(),
                );
                continue;
            }
            match self.process_ffi_item(ffi_item.clone(), &checks, &trait_types) {
                Ok(results) => {
                    for item in results {
                        match item {
                            ProcessedFfiItem::Item(rust_item) => {
                                if let RustItem::TraitImpl(trait_impl) = &rust_item {
                                    trait_types.push(trait_impl.into());
                                }

                                self.add_rust_item(Some(ffi_item_id.clone()), rust_item)?;
                            }
                            ProcessedFfiItem::Function(function) => {
                                let entry = grouped_functions
                                    .entry(function.desired_path.clone())
                                    .or_default();
                                entry.push(ItemWithSource::new(&ffi_item_id, function));
                            }
                        }
                    }
                }
                Err(err) => {
                    debug!(
                        "failed to process ffi item: {}: {}",
                        ffi_item.item.short_text(),
                        err
                    );
                    print_trace(&err, Some(log::Level::Trace));
                }
            }
        }
        Ok(grouped_functions)
    }

    fn try_caption_strategy(
        &self,
        functions: &[ItemWithSource<FunctionWithDesiredPath>],
        strategy: &RustFunctionCaptionStrategy,
    ) -> Result<()> {
        let mut paths = BTreeSet::new();
        for function in functions {
            let path = function.item.apply_strategy(strategy)?;
            if paths.contains(&path) {
                bail!("conflicting path: {:?}", path);
            }
            if self.data.db.find_rust_item(&path).is_some() {
                bail!("path already taken by an existing item: {:?}", path);
            }
            paths.insert(path);
        }

        Ok(())
    }

    fn finalize_functions(
        &mut self,
        grouped_functions: BTreeMap<RustPath, Vec<ItemWithSource<FunctionWithDesiredPath>>>,
    ) -> Result<()> {
        let all_strategies = RustFunctionCaptionStrategy::all();

        for (_group_path, functions) in grouped_functions {
            let mut chosen_strategy = None;
            if functions.len() > 1 {
                trace!("choosing caption strategy for:");
                for function in &functions {
                    trace!("* {}", function.item.function.kind.short_text());
                }
                for strategy in &all_strategies {
                    match self.try_caption_strategy(&functions, strategy) {
                        Ok(_) => {
                            trace!("  chosen strategy: {:?}", strategy);
                            chosen_strategy = Some(strategy.clone());
                            break;
                        }
                        Err(err) => {
                            trace!("  strategy failed: {:?}: {}", strategy, err);
                        }
                    }
                }
                if chosen_strategy.is_none() {
                    trace!("  all strategies failed, using default strategy");
                    chosen_strategy = Some(RustFunctionCaptionStrategy {
                        mut_: false,
                        args_count: false,
                        arg_names: false,
                        arg_types: Some(RustTypeCaptionStrategy::LastName),
                        static_: false,
                    });
                }
            }

            for function in functions {
                let path = if let Some(strategy) = &chosen_strategy {
                    function.item.apply_strategy(strategy).unwrap()
                } else {
                    function.item.desired_path
                };
                let final_path = self.data.db.make_unique_rust_path(&path);
                let item = RustItem::Function(function.item.function.with_path(final_path));
                self.add_rust_item(Some(function.source_id), item)?;
            }
        }
        Ok(())
    }
}

pub fn run(data: &mut ProcessorData<'_>) -> Result<()> {
    let mut state = State {
        data,
        special_module_paths: HashMap::new(),
    };
    for &module in &[
        RustSpecialModuleKind::CrateRoot,
        RustSpecialModuleKind::Ffi,
        RustSpecialModuleKind::Ops,
        RustSpecialModuleKind::SizedTypes,
    ] {
        state.generate_special_module(module)?;
    }

    state.generate_crate_reexport("cpp_core")?;
    let dependencies = state
        .data
        .config
        .crate_properties()
        .dependencies()
        .iter()
        .filter(|dep| dep.kind() == CrateDependencyKind::Ritual);
    for dependency in dependencies {
        state.generate_crate_reexport(dependency.name())?;
    }

    state.process_cpp_items()?;
    let grouped_functions = state.process_ffi_items()?;
    state.finalize_functions(grouped_functions)?;

    Ok(())
}

fn detect_callback_function(function: &UnnamedRustFunction) -> Option<&RustFunctionPointerType> {
    if function.arguments.len() < 3 {
        return None;
    }
    let args = &function.arguments[function.arguments.len() - 3..];
    let void_ptr = RustType::new_pointer(
        false,
        RustType::Common(RustCommonType {
            path: RustPath::from_good_str("std::ffi::c_void"),
            generic_arguments: None,
        }),
    );
    if args[0].name != "callback" {
        return None;
    }
    let callback_type;
    if let RustType::Common(common) = &args[0].argument_type.ffi_type() {
        if common.path != "std::option::Option" {
            return None;
        }
        if let Some(args) = &common.generic_arguments {
            if args.len() == 1 {
                if let RustType::FunctionPointer(t) = &args[0] {
                    callback_type = t;
                } else {
                    return None;
                }
            } else {
                return None;
            }
        } else {
            return None;
        }
    } else {
        return None;
    }
    if args[1].name != "deleter" {
        return None;
    }
    let deleter_type = RustType::new_option(RustType::FunctionPointer(RustFunctionPointerType {
        arguments: vec![void_ptr.clone()],
        return_type: Box::new(RustType::unit()),
    }));
    if args[1].argument_type.ffi_type() != &deleter_type {
        return None;
    }
    if args[2].name != "data" {
        return None;
    }
    if args[2].argument_type.ffi_type() != &void_ptr {
        return None;
    }
    Some(callback_type)
}

impl FunctionWithDesiredPath {
    fn apply_strategy(&self, strategy: &RustFunctionCaptionStrategy) -> Result<RustPath> {
        let mut suffix = String::new();
        let normal_args = self
            .function
            .arguments
            .iter()
            .filter(|arg| arg.name != "self")
            .collect_vec();
        if strategy.args_count {
            suffix.push_str(&format!("_{}a", normal_args.len()));
        }
        if strategy.arg_names && !normal_args.is_empty() {
            let names = normal_args.iter().map(|arg| &arg.name).join("_");
            suffix.push_str(&format!("_{}", names));
        }
        if let Some(type_strategy) = strategy.arg_types {
            if !normal_args.is_empty() {
                let context = self.desired_path.parent()?;

                let mut types_with_counts = Vec::<(u32, &RustFinalType)>::new();
                for t in normal_args {
                    if let Some((count, type_)) = types_with_counts.last_mut() {
                        if type_.api_type() == t.argument_type.api_type() {
                            *count += 1;
                            continue;
                        }
                    }
                    types_with_counts.push((1, &t.argument_type));
                }

                let types = types_with_counts
                    .map_if_ok(|(count, arg)| -> Result<String> {
                        let text = arg.api_type().caption(&context, type_strategy)?;
                        Ok(if count == 1 {
                            text
                        } else {
                            format!("{}_{}", count, text)
                        })
                    })?
                    .join("_");
                suffix.push_str(&format!("_{}", types));
            }
        }
        match self.function.self_arg_kind()? {
            RustFunctionSelfArgKind::ConstRef | RustFunctionSelfArgKind::Value => {}
            RustFunctionSelfArgKind::None => {
                if strategy.static_ {
                    suffix.push_str("_static");
                }
            }
            RustFunctionSelfArgKind::MutRef => {
                if strategy.mut_ {
                    suffix.push_str("_mut");
                }
            }
        }

        let suffix = suffix.to_snake_case();
        let name = if suffix.is_empty() {
            self.desired_path.last().to_string()
        } else if strategy.arg_types.is_some() && self.desired_path.last() == "new" {
            format!("from_{}", suffix)
        } else {
            let delimiter = if self.desired_path.last().ends_with('_') {
                ""
            } else {
                "_"
            };
            format!("{}{}{}", self.desired_path.last(), delimiter, suffix)
        };
        let name = sanitize_rust_identifier(&name, false);
        Ok(self.desired_path.parent()?.join(name))
    }
}

/// Returns alphanumeric identifier for this operator
/// used to name wrapper functions.
fn operator_function_name(operator: &CppOperator) -> Result<&'static str> {
    use self::CppOperator::*;
    Ok(match operator {
        Conversion(..) => {
            bail!("operator_function_name: conversion operators are not supported");
        }
        Assignment => "set_from",
        Addition => "add",
        Subtraction => "sub",
        UnaryPlus => "unary_plus",
        UnaryMinus => "neg",
        Multiplication => "mul",
        Division => "div",
        Modulo => "rem",
        PrefixIncrement => "inc",
        PostfixIncrement => "inc_postfix",
        PrefixDecrement => "dec",
        PostfixDecrement => "dec_postfix",
        EqualTo => "eq",
        NotEqualTo => "ne",
        GreaterThan => "gt",
        LessThan => "lt",
        GreaterThanOrEqualTo => "ge",
        LessThanOrEqualTo => "le",
        LogicalNot => "not",
        LogicalAnd => "and",
        LogicalOr => "or",
        BitwiseNot => "bit_not",
        BitwiseAnd => "bit_and",
        BitwiseOr => "bit_or",
        BitwiseXor => "bit_xor",
        BitwiseLeftShift => "shl",
        BitwiseRightShift => "shr",
        AdditionAssignment => "add_assign",
        SubtractionAssignment => "sub_assign",
        MultiplicationAssignment => "mul_assign",
        DivisionAssignment => "div_assign",
        ModuloAssignment => "rem_assign",
        BitwiseAndAssignment => "bit_and_assign",
        BitwiseOrAssignment => "bit_or_assign",
        BitwiseXorAssignment => "bit_xor_assign",
        BitwiseLeftShiftAssignment => "shl_assign",
        BitwiseRightShiftAssignment => "shr_assign",
        Subscript => "index",
        Indirection => "indirection",
        AddressOf => "address_of",
        StructureDereference => "struct_deref",
        PointerToMember => "ptr_to_member",
        FunctionCall => "call",
        Comma => "comma",
        New => "new",
        NewArray => "new_array",
        Delete => "delete",
        DeleteArray => "delete_array",
    })
}

fn is_second_slot_constructor(
    data: &ProcessorData<'_>,
    function: &DbItem<&CppFfiFunction>,
) -> Result<bool> {
    if function.item.arguments.len() != 3 {
        return Ok(false);
    }

    let cpp_function = if let Some(f) = data
        .db
        .source_cpp_item(&function.id)?
        .and_then(|i| i.item.as_function_ref())
    {
        f
    } else {
        return Ok(false);
    };

    if !cpp_function.is_constructor() {
        return Ok(false);
    }

    let from_slot_wrapper = data
        .db
        .source_ffi_item(&function.id)?
        .map_or(false, |x| x.item.is_slot_wrapper());
    Ok(from_slot_wrapper)
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
