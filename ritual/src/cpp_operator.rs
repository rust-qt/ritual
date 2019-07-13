//! Types for describing C++ operators

use crate::cpp_type::CppType;
use serde_derive::{Deserialize, Serialize};

/// Available types of C++ operators
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub enum CppOperator {
    /// (type) a
    Conversion(CppType),
    /// a = b
    Assignment,
    /// a + b
    Addition,
    /// a - b
    Subtraction,
    /// +a
    UnaryPlus,
    /// -a
    UnaryMinus,
    /// a * b
    Multiplication,
    /// a / b
    Division,
    /// a % b
    Modulo,
    /// ++a
    PrefixIncrement,
    /// a++
    PostfixIncrement,
    /// --a
    PrefixDecrement,
    /// a--
    PostfixDecrement,
    /// a == b
    EqualTo,
    /// a != b
    NotEqualTo,
    /// a > b
    GreaterThan,
    /// a < b
    LessThan,
    /// a >= b
    GreaterThanOrEqualTo,
    /// a <= b
    LessThanOrEqualTo,
    /// !a
    LogicalNot,
    /// a && b
    LogicalAnd,
    /// a || b
    LogicalOr,
    /// ~a
    BitwiseNot,
    /// a & b
    BitwiseAnd,
    /// a | b
    BitwiseOr,
    /// a ^ b
    BitwiseXor,
    /// a << b
    BitwiseLeftShift,
    /// a >> b
    BitwiseRightShift,

    /// a += b
    AdditionAssignment,
    /// a -= b
    SubtractionAssignment,
    /// a *= b
    MultiplicationAssignment,
    /// a /= b
    DivisionAssignment,
    /// a %= b
    ModuloAssignment,
    /// a &= b
    BitwiseAndAssignment,
    /// a |= b
    BitwiseOrAssignment,
    /// a ^= b
    BitwiseXorAssignment,
    /// a <<= b
    BitwiseLeftShiftAssignment,
    /// a >>= b
    BitwiseRightShiftAssignment,
    /// a[b]
    Subscript,
    /// *a
    Indirection,
    /// &a
    AddressOf,
    /// a->b
    StructureDereference,
    /// a->*b
    PointerToMember,
    /// a(a1, a2)
    FunctionCall,
    /// a, b
    Comma,
    /// new type
    New,
    /// new type[n]
    NewArray,
    /// delete a
    Delete,
    /// delete[] a
    DeleteArray,
}

/// Constraints applied to a C++ operator method
/// of a certain kind
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppOperatorInfo {
    /// String that must appear after `"operator"` in the method name,
    /// e.g. `">"` for `"operator>"`. `"operator"` prefix must
    /// be present for any operator. This field is `None` for
    /// conversion operator, as its name includes
    /// corresponding C++ type instead of a fixed string.
    pub function_name_suffix: Option<&'static str>,
    /// Total number of arguments, including implicit "this" argument.
    /// Most operators can be class members or free functions,
    /// but total number of arguments is the same in both cases.
    pub arguments_count: usize,
    /// True if this kind of operator can have variadic arguments.
    /// Only the function call operator has this property.
    pub allows_variadic_arguments: bool,
}

impl CppOperator {
    /// Reports information about this operator
    pub fn info(&self) -> CppOperatorInfo {
        use self::CppOperator::*;

        fn oi(suffix: &'static str, count: usize) -> CppOperatorInfo {
            CppOperatorInfo {
                function_name_suffix: Some(suffix),
                arguments_count: count,
                allows_variadic_arguments: false,
            }
        }

        match *self {
            Conversion(..) => CppOperatorInfo {
                function_name_suffix: None,
                arguments_count: 1,
                allows_variadic_arguments: false,
            },
            Assignment => oi("=", 2),
            Addition => oi("+", 2),
            Subtraction => oi("-", 2),
            UnaryPlus => oi("+", 1),
            UnaryMinus => oi("-", 1),
            Multiplication => oi("*", 2),
            Division => oi("/", 2),
            Modulo => oi("%", 2),
            PrefixIncrement => oi("++", 1),
            PostfixIncrement => oi("++", 2),
            PrefixDecrement => oi("--", 1),
            PostfixDecrement => oi("--", 2),
            EqualTo => oi("==", 2),
            NotEqualTo => oi("!=", 2),
            GreaterThan => oi(">", 2),
            LessThan => oi("<", 2),
            GreaterThanOrEqualTo => oi(">=", 2),
            LessThanOrEqualTo => oi("<=", 2),
            LogicalNot => oi("!", 1),
            LogicalAnd => oi("&&", 2),
            LogicalOr => oi("||", 2),
            BitwiseNot => oi("~", 1),
            BitwiseAnd => oi("&", 2),
            BitwiseOr => oi("|", 2),
            BitwiseXor => oi("^", 2),
            BitwiseLeftShift => oi("<<", 2),
            BitwiseRightShift => oi(">>", 2),
            AdditionAssignment => oi("+=", 2),
            SubtractionAssignment => oi("-=", 2),
            MultiplicationAssignment => oi("*=", 2),
            DivisionAssignment => oi("/=", 2),
            ModuloAssignment => oi("%=", 2),
            BitwiseAndAssignment => oi("&=", 2),
            BitwiseOrAssignment => oi("|=", 2),
            BitwiseXorAssignment => oi("^=", 2),
            BitwiseLeftShiftAssignment => oi("<<=", 2),
            BitwiseRightShiftAssignment => oi(">>=", 2),
            Subscript => oi("[]", 2),
            Indirection => oi("*", 1),
            AddressOf => oi("&", 1),
            StructureDereference => oi("->", 1),
            PointerToMember => oi("->*", 2),
            FunctionCall => CppOperatorInfo {
                function_name_suffix: Some("()"),
                arguments_count: 0,
                allows_variadic_arguments: true,
            },
            Comma => oi(",", 2),
            New => oi("new", 2),
            NewArray => oi("new[]", 2),
            Delete => oi("delete", 2),
            DeleteArray => oi("delete[]", 2),
        }
    }

    /// Returns all existing operator kinds except for
    /// conversion operator which includes an arbitrary C++ type.
    pub fn all() -> Vec<CppOperator> {
        use self::CppOperator::*;
        vec![
            Assignment,
            Addition,
            Subtraction,
            UnaryPlus,
            UnaryMinus,
            Multiplication,
            Division,
            Modulo,
            PrefixIncrement,
            PostfixIncrement,
            PrefixDecrement,
            PostfixDecrement,
            EqualTo,
            NotEqualTo,
            GreaterThan,
            LessThan,
            GreaterThanOrEqualTo,
            LessThanOrEqualTo,
            LogicalNot,
            LogicalAnd,
            LogicalOr,
            BitwiseNot,
            BitwiseAnd,
            BitwiseOr,
            BitwiseXor,
            BitwiseLeftShift,
            BitwiseRightShift,
            AdditionAssignment,
            SubtractionAssignment,
            MultiplicationAssignment,
            DivisionAssignment,
            ModuloAssignment,
            BitwiseAndAssignment,
            BitwiseOrAssignment,
            BitwiseXorAssignment,
            BitwiseLeftShiftAssignment,
            BitwiseRightShiftAssignment,
            Subscript,
            Indirection,
            AddressOf,
            StructureDereference,
            PointerToMember,
            FunctionCall,
            Comma,
            New,
            NewArray,
            Delete,
            DeleteArray,
        ]
    }
}
