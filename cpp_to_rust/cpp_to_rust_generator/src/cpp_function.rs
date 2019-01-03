//! Types for handling information about C++ methods.

use crate::common::errors::{bail, Result};
use crate::common::string_utils::JoinWithSeparator;
use crate::common::utils::MapIfOk;
use crate::cpp_data::CppName;
use crate::cpp_data::CppVisibility;
pub use crate::cpp_operator::{CppOperator, CppOperatorInfo};
use crate::cpp_type::CppPointerLikeTypeKind;
use crate::cpp_type::{CppClassType, CppType};
use serde_derive::{Deserialize, Serialize};

/// Information about an argument of a C++ method
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppFunctionArgument {
    /// Identifier. If the argument doesn't have a name
    /// (which is allowed in C++), this field contains
    /// generated name "argX" (X is position of the argument).
    pub name: String,
    /// Argument type
    pub argument_type: CppType,
    /// Flag indicating that the argument has default value and
    /// therefore can be omitted when calling the method
    pub has_default_value: bool,
}

/// Enumerator indicating special cases of C++ methods.
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub enum CppFunctionKind {
    /// Just a class method
    Regular,
    /// Constructor
    Constructor,
    /// Destructor
    Destructor,
}

/// Information about a C++ class member method
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppFunctionMemberData {
    /// Type of the class where this method belong. This is used to construct
    /// type of "this" pointer and return type of constructors.
    pub class_type: CppClassType,
    /// Whether this method is a constructor, a destructor or an operator
    pub kind: CppFunctionKind,
    /// True if this is a virtual method
    pub is_virtual: bool,
    /// True if this is a pure virtual method (requires is_virtual = true)
    pub is_pure_virtual: bool,
    /// True if this is a const method, i.e. "this" pointer receives by
    /// this method has const type
    pub is_const: bool,
    /// True if this is a static method, i.e. it doesn't receive "this" pointer at all.
    pub is_static: bool,
    /// Method visibility
    pub visibility: CppVisibility,
    /// True if the method is a Qt signal
    pub is_signal: bool,
    /// True if the method is a Qt slot
    pub is_slot: bool,
}

/// C++ documentation for a method
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppFunctionDoc {
    /// HTML anchor of this documentation entry
    /// (used to detect duplicates)
    pub anchor: String,
    /// HTML content
    pub html: String,
    /// If the documentation parser couldn't find documentation for the exact same
    /// method, it can still provide documentation entry for the closest match.
    /// In this case, this field should contain C++ declaration of the found method.
    pub mismatched_declaration: Option<String>,
    /// Absolute URL to online documentation page for this method
    pub url: String,
    /// Absolute documentation URLs encountered in the content
    pub cross_references: Vec<String>,
}

/// Information about a C++ method
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppFunction {
    /// Identifier. For class methods, this field includes
    /// only the method's own name. For free functions,
    /// this field also includes namespaces (if any).
    pub name: CppName,
    /// Additional information about a class member function
    /// or None for free functions
    pub member: Option<CppFunctionMemberData>,
    /// If the method is a C++ operator, indicates its kind
    pub operator: Option<CppOperator>,
    /// Return type of the method.
    /// Return type is reported as void for constructors and destructors.
    pub return_type: CppType,
    /// List of the method's arguments
    pub arguments: Vec<CppFunctionArgument>,
    //  /// If Some, the method is derived from another method by omitting arguments,
    //  /// and this field contains all arguments of the original method.
    //  pub arguments_before_omitting: Option<Vec<CppMethodArgument>>,
    /// Whether the argument list is terminated with "..."
    pub allows_variadic_arguments: bool,
    /// For a template method, this fields contains its template arguments
    /// in the form of `CppTypeBase::TemplateParameter` types.
    /// For an instantiated template method, this field contains the types
    /// used for instantiation.
    ///
    /// For example, `T QObject::findChild<T>()` would have
    /// a `TemplateParameter` type in `template_arguments`
    /// because it's not instantiated, and
    /// `QWidget* QObject::findChild<QWidget*>()` would have `QWidget*` type in
    /// `template_arguments`.
    ///
    /// This field is `None` if this is not a template method.
    /// If the method belongs to a template class,
    /// the class's template arguments are not included here.
    /// Instead, they are available in `member.class_type`.
    pub template_arguments: Option<Vec<CppType>>,
    //pub template_arguments_values: Option<Vec<CppType>>,
    /// C++ code of the method's declaration.
    /// None if the method was not explicitly declared.
    pub declaration_code: Option<String>,
    /// C++ documentation data for this method
    pub doc: Option<CppFunctionDoc>,
}

/// Chosen type allocation place for the method
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub enum ReturnValueAllocationPlace {
    /// The method returns a class object by value (or is a constructor), and
    /// it's translated to "output" FFI argument and placement new
    Stack,
    /// The method returns a class object by value (or is a constructor), and
    /// it's translated to pointer FFI return type and plain new
    Heap,
    /// The method does not return a class object by value, so
    /// the direct equivalent of the value is used in FFI.
    NotApplicable,
}

impl CppFunctionKind {
    /// Returns true if this method is a constructor
    pub fn is_constructor(&self) -> bool {
        match *self {
            CppFunctionKind::Constructor => true,
            _ => false,
        }
    }
    /// Returns true if this method is a destructor
    pub fn is_destructor(&self) -> bool {
        match *self {
            CppFunctionKind::Destructor => true,
            _ => false,
        }
    }
    #[allow(dead_code)]
    /// Returns true if this method is a regular method or a free function
    pub fn is_regular(&self) -> bool {
        match *self {
            CppFunctionKind::Regular => true,
            _ => false,
        }
    }
}

impl CppFunction {
    /// Checks if two methods have exactly the same set of input argument types
    pub fn argument_types_equal(&self, other: &CppFunction) -> bool {
        if self.arguments.len() != other.arguments.len() {
            return false;
        }
        if self.allows_variadic_arguments != other.allows_variadic_arguments {
            return false;
        }
        for (i, j) in self.arguments.iter().zip(other.arguments.iter()) {
            if i.argument_type != j.argument_type {
                return false;
            }
        }
        true
    }

    pub fn is_same(&self, other: &CppFunction) -> bool {
        self.name == other.name
            && self.member == other.member
            && self.operator == other.operator
            && self.return_type == other.return_type
            && self.argument_types_equal(other)
            && self.template_arguments == other.template_arguments
    }

    /// Returns fully qualified C++ name of this method,
    /// i.e. including namespaces and class name (if any).
    /// This method is not suitable for code generation.
    pub fn full_name(&self) -> String {
        if let Some(ref info) = self.member {
            format!(
                "{}::{}",
                CppType::Class(info.class_type.clone()).to_cpp_pseudo_code(),
                self.name
            )
        } else {
            self.name.to_string()
        }
    }

    /// Returns the identifier this method would be presented with
    /// in Qt documentation.
    pub fn doc_id(&self) -> String {
        if let Some(ref info) = self.member {
            format!("{}::{}", info.class_type.name, self.name)
        } else {
            self.name.to_string()
        }
    }

    /// Returns short text representing values in this method
    /// (only for debugging output).
    pub fn short_text(&self) -> String {
        let mut s = String::new();
        if let Some(ref info) = self.member {
            if info.is_virtual {
                if info.is_pure_virtual {
                    s = format!("{} pure virtual", s);
                } else {
                    s = format!("{} virtual", s);
                }
            }
            if info.is_static {
                s = format!("{} static", s);
            }
            if info.visibility == CppVisibility::Protected {
                s = format!("{} protected", s);
            }
            if info.visibility == CppVisibility::Private {
                s = format!("{} private", s);
            }
            if info.is_signal {
                s = format!("{} [signal]", s);
            }
            if info.is_slot {
                s = format!("{} [slot]", s);
            }
            match info.kind {
                CppFunctionKind::Constructor => s = format!("{} [constructor]", s),
                CppFunctionKind::Destructor => s = format!("{} [destructor]", s),
                CppFunctionKind::Regular => {}
            }
        }
        if self.allows_variadic_arguments {
            s = format!("{} [var args]", s);
        }
        s = format!("{} {}", s, self.return_type.to_cpp_pseudo_code());
        s = format!("{} {}", s, self.full_name());
        if let Some(ref args) = self.template_arguments {
            s = format!(
                "{}<{}>",
                s,
                args.iter().map(|x| x.to_cpp_pseudo_code()).join(", ")
            );
        }
        s = format!(
            "{}({})",
            s,
            self.arguments
                .iter()
                .map(|arg| format!(
                    "{} {}{}",
                    arg.argument_type.to_cpp_pseudo_code(),
                    arg.name,
                    if arg.has_default_value {
                        " = ?".to_string()
                    } else {
                        String::new()
                    }
                ))
                .join(", ")
        );
        if let Some(ref info) = self.member {
            if info.is_const {
                s = format!("{} const", s);
            }
        }
        s.trim().to_string()
    }

    /// Returns name of the class this method belongs to, if any.
    pub fn class_name(&self) -> Option<&CppName> {
        match self.member {
            Some(ref info) => Some(&info.class_type.name),
            None => None,
        }
    }

    /// Returns true if this method is a constructor.
    pub fn is_constructor(&self) -> bool {
        match self.member {
            Some(ref info) => info.kind.is_constructor(),
            None => false,
        }
    }

    /// Returns true if this method is a destructor.
    pub fn is_destructor(&self) -> bool {
        match self.member {
            Some(ref info) => info.kind.is_destructor(),
            None => false,
        }
    }

    pub fn is_virtual(&self) -> bool {
        match self.member {
            Some(ref info) => info.is_virtual,
            None => false,
        }
    }

    /// Returns the identifier that should be used in `QObject::connect`
    /// to specify this signal or slot.
    pub fn receiver_id(&self) -> Result<String> {
        let type_num = if let Some(ref info) = self.member {
            if info.is_slot {
                "1"
            } else if info.is_signal {
                "2"
            } else {
                bail!("not a signal or slot");
            }
        } else {
            bail!("not a class method");
        };
        Ok(format!(
            "{}{}({})",
            type_num,
            self.name,
            self.arguments
                .iter()
                .map_if_ok(|arg| arg.argument_type.to_cpp_code(None))?
                .join(",")
        ))
    }

    pub fn member(&self) -> Option<&CppFunctionMemberData> {
        self.member.as_ref()
    }

    #[allow(dead_code)]
    /// Returns true if this method is an operator.
    pub fn is_operator(&self) -> bool {
        self.operator.is_some()
    }

    /// Returns collection of all types found in the signature of this method,
    /// including argument types, return type and type of `this` implicit parameter.
    pub fn all_involved_types(&self) -> Vec<CppType> {
        let mut result: Vec<CppType> = Vec::new();
        if let Some(ref class_membership) = self.member {
            result.push(CppType::PointerLike {
                is_const: class_membership.is_const,
                kind: CppPointerLikeTypeKind::Pointer,
                target: Box::new(CppType::Class(class_membership.class_type.clone())),
            });
        }
        for t in self.arguments.iter().map(|x| x.argument_type.clone()) {
            result.push(t);
        }
        result.push(self.return_type.clone());
        if let Some(ref operator) = self.operator {
            if let CppOperator::Conversion(ref cpp_type) = *operator {
                result.push(cpp_type.clone());
            }
        }
        result
    }
}
