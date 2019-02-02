use crate::config::Config;
use crate::cpp_data::CppPath;
use crate::cpp_data::CppPathItem;
use crate::cpp_data::CppTypeDataKind;
use crate::cpp_ffi_data::CppFfiFunction;
use crate::cpp_ffi_data::CppFfiFunctionKind;
use crate::cpp_ffi_data::CppFieldAccessorType;
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
use crate::rust_info::RustItemKind;
use crate::rust_info::RustModule;
use crate::rust_info::RustModuleDoc;
use crate::rust_info::RustModuleKind;
use crate::rust_info::RustPathScope;
use crate::rust_info::RustStruct;
use crate::rust_info::RustStructKind;
use crate::rust_info::RustWrapperType;
use crate::rust_info::RustWrapperTypeDocData;
use crate::rust_info::RustWrapperTypeKind;
use crate::rust_type::RustPath;
use crate::rust_type::RustPointerLikeTypeKind;
use crate::rust_type::RustType;
use log::trace;
use ritual_common::errors::*;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NameType {
    General,
    Module,
    FfiStruct,
    FfiFunction,
    SizedItem,
    ClassPtr,
    FieldAccessor(CppFieldAccessorType),
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
        // TODO: use dep_databases!
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
            CppType::Void => RustType::EmptyTuple,

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
                let rust_item = self.find_rust_item(path)?;
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
            path: self.generate_rust_path(&data.path, NameType::FfiFunction)?,
            arguments: args,
        })
    }

    fn find_rust_item(&self, cpp_path: &CppPath) -> Result<&RustDatabaseItem> {
        // TODO: search in deps!
        let index = match self.cpp_path_to_index.get(cpp_path) {
            Some(index) => index,
            None => bail!("unknown cpp path: {}", cpp_path),
        };

        self.rust_database
            .items
            .iter()
            .find(|item| item.cpp_item_index == Some(*index))
            .ok_or_else(|| err_msg(format!("rust item not found for path: {:?}", cpp_path)))
    }

    fn get_strategy(&self, parent_path: &CppPath) -> Result<RustPathScope> {
        let rust_item = self.find_rust_item(parent_path)?;

        let rust_path = rust_item.path().ok_or_else(|| {
            err_msg(format!(
                "rust item doesn't have rust path (cpp_path = {:?})",
                parent_path
            ))
        })?;

        Ok(RustPathScope {
            path: rust_path.clone(),
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

    fn generate_rust_path(&self, cpp_path: &CppPath, name_type: NameType) -> Result<RustPath> {
        let strategy = match name_type {
            NameType::FfiStruct | NameType::FfiFunction => {
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
            | NameType::Module
            | NameType::ClassPtr
            | NameType::FieldAccessor(_) => {
                if let Some(parent) = cpp_path.parent() {
                    self.get_strategy(&parent)?
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
            NameType::FfiStruct | NameType::SizedItem => cpp_path
                .items
                .iter()
                .map_if_ok(|item| cpp_path_item_to_name(item))?
                .join("_"),
            NameType::FfiFunction => cpp_path.last().to_string(),
            NameType::ClassPtr => format!("{}Ptr", cpp_path_item_to_name(&cpp_path.last())?),
            NameType::General | NameType::Module => cpp_path_item_to_name(&cpp_path.last())?,
            NameType::FieldAccessor(accessor_type) => {
                let name = &cpp_path.last().name;
                match accessor_type {
                    CppFieldAccessorType::CopyGetter => name.to_string(),
                    CppFieldAccessorType::ConstRefGetter => name.to_string(),
                    CppFieldAccessorType::MutRefGetter => format!("{}_mut", name),
                    CppFieldAccessorType::Setter => format!("set_{}", name),
                }
            }
        };

        let mut number = None;
        if name_type == NameType::FfiFunction {
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
            let sanitized_name = sanitize_rust_identifier(&name_try, name_type == NameType::Module);
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
                    let rust_path = self.generate_rust_path(path, NameType::General)?;
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
                                self.generate_rust_path(&data.path, internal_name_type)?;
                            let public_path =
                                self.generate_rust_path(&data.path, public_name_type)?;
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
                                self.generate_rust_path(&data.path, NameType::General)?;
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
                    let rust_path = self.generate_rust_path(&value.path, NameType::General)?;

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
                CppFfiItemKind::Function(function) => {
                    let ffi_function = self.generate_ffi_function(&function)?;
                    let rust_item = RustDatabaseItem {
                        kind: RustItemKind::FfiFunction(ffi_function),
                        cpp_item_index: Some(cpp_item_index),
                    };

                    let _rust_path = match &function.kind {
                        CppFfiFunctionKind::Function { cpp_function, .. } => {
                            self.generate_rust_path(&cpp_function.path, NameType::General)?
                        }
                        CppFfiFunctionKind::FieldAccessor {
                            field,
                            accessor_type,
                        } => {
                            let name_type = NameType::FieldAccessor(*accessor_type);
                            self.generate_rust_path(&field.path, name_type)?
                        }
                    };
                    // TODO: generate final Rust function

                    self.rust_database.items.push(rust_item);
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
            .filter_map(|(index, item)| match item.cpp_data {
                CppItemData::Namespace(ref path) => Some((path.clone(), index)),
                CppItemData::Type(ref data) => Some((data.path.clone(), index)),
                _ => None,
            })
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

pub fn rust_name_resolver_step() -> ProcessingStep {
    ProcessingStep::new("rust_name_resolver", run)
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
