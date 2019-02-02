//! Generates Rust public API and FFI functions

use caption_strategy::TypeCaptionStrategy;
use common::errors::{unexpected, Result, ResultExt};
use common::log;
use common::string_utils::{CaseOperations, WordIterator};
use common::utils::{add_to_multihash, MapIfOk};
use cpp_data::{CppDataWithDeps, CppEnumValue, CppTypeAllocationPlace, CppTypeKind};
use cpp_ffi_data::{
    CppAndFfiMethod, CppCast, CppFfiArgumentMeaning, CppFfiHeaderData, CppFfiMethodKind,
    CppFfiType, CppTypeConversionToFfi,
};
use cpp_method::{CppMethod, ReturnValueAllocationPlace};
use cpp_operator::CppOperator;
use cpp_type::{
    CppBuiltInNumericType, CppFunctionPointerType, CppSpecificNumericType,
    CppSpecificNumericTypeKind, CppType, CppTypeClassBase, CppTypeIndirection, CppTypeRole,
};
use doc_formatter;
use itertools::Itertools;
use rust_info::{
    RustEnumValue, RustFFIArgument, RustFFIFunction, RustMethod, RustMethodArgument,
    RustMethodArguments, RustMethodArgumentsVariant, RustMethodCaptionStrategy, RustMethodDocItem,
    RustMethodScope, RustMethodSelfArgKind, RustModule, RustProcessedTypeInfo,
    RustQtReceiverDeclaration, RustQtReceiverType, RustQtSlotWrapper, RustTypeDeclaration,
    RustTypeDeclarationKind, RustTypeWrapperKind, TraitAssociatedType, TraitImpl, TraitImplExtra,
};
use rust_type::{CompleteType, RustName, RustToCTypeConversion, RustType, RustTypeIndirection};
use std::collections::{hash_map, HashMap, HashSet};

/// Generator of the Rust public API of the crate.
pub struct RustGenerator<'a> {
    /// Data collected on previous step of the generator workflow
    input_data: RustGeneratorInputData<'a>,

    top_module_names: HashMap<String, RustName>,
    /// Type wrappers created for this crate
    processed_types: Vec<RustProcessedTypeInfo>,
}

/// Results of adapting API for Rust wrapper.
/// This data is passed to Rust code generator.
pub struct RustGeneratorOutput {
    /// List of Rust modules to be generated.
    pub modules: Vec<RustModule>,
    /// List of FFI function imports to be generated.
    pub ffi_functions: Vec<(String, Vec<RustFFIFunction>)>,
    /// List of processed C++ types and their corresponding Rust names
    pub processed_types: Vec<RustProcessedTypeInfo>,
}

/// Information required by Rust generator
pub struct RustGeneratorInputData<'a> {
    /// Processed C++ data
    pub cpp_data: &'a CppDataWithDeps<'a>,
    /// Generated headers
    pub cpp_ffi_headers: Vec<CppFfiHeaderData>,
    /// Type wrappers found in all dependencies
    pub dependency_types: Vec<&'a [RustProcessedTypeInfo]>,
    /// Name of generated crate
    pub crate_name: String,
    /// Flag instructing to remove leading "Q" and "Qt"
    /// from identifiers.
    pub remove_qt_prefix: bool,
    /// List of namespaces to filter out during code generation
    pub filtered_namespaces: Vec<String>,
}

impl<'a> RustGeneratorInputData<'a> {
    /// Execute processing
    #[allow(clippy::extend_from_slice)]
    #[allow(clippy::block_in_if_condition_stmt)]
    pub fn run(self) -> Result<RustGeneratorOutput> {
        let mut generator = RustGenerator {
            top_module_names: HashMap::new(),
            processed_types: Vec::new(),
            input_data: self,
        };

        generator.processed_types = generator.calc_processed_types()?;
        let mut modules = Vec::new();
        {
            let mut cpp_methods: Vec<&CppAndFfiMethod> = Vec::new();
            for header in &generator.input_data.cpp_ffi_headers {
                cpp_methods.extend(header.methods.iter());
            }
            let mut module_names_set = HashSet::new();
            for item in &generator.processed_types {
                if !module_names_set.contains(&item.rust_name.parts[1]) {
                    module_names_set.insert(item.rust_name.parts[1].clone());
                }
            }
            cpp_methods = cpp_methods
        .into_iter()
        .filter(|method| {
          if let Some(ref info) = method.cpp_method.class_membership {
            if !generator.processed_types.iter().any(|t| {
              t.cpp_name == info.class_type.name
                && t.cpp_template_arguments == info.class_type.template_arguments
            }) {
              log::llog(log::DebugRustSkips, || {
                "Warning: method is skipped because class type is not available in Rust:"
              });
              log::llog(log::DebugRustSkips, || format!("{}\n", method.short_text()));
              return false;
            }
          }
          true
        })
        .collect();
            for method in cpp_methods.clone() {
                if method.cpp_method.class_membership.is_none() {
                    let rust_name = generator.free_function_rust_name(&method.cpp_method)?;
                    if !module_names_set.contains(&rust_name.parts[1]) {
                        module_names_set.insert(rust_name.parts[1].clone());
                    }
                }
            }

            let mut module_names: Vec<_> = module_names_set.into_iter().collect();
            module_names.sort();
            let module_count = module_names.len();
            for (i, module_name) in module_names.into_iter().enumerate() {
                log::status(format!(
                    "({}/{}) Generating module: {}",
                    i + 1,
                    module_count,
                    module_name
                ));
                let full_module_name =
                    RustName::new(vec![generator.input_data.crate_name.clone(), module_name])?;
                let (module, tmp_cpp_methods) =
                    generator.generate_module(cpp_methods, &full_module_name)?;
                cpp_methods = tmp_cpp_methods;
                if let Some(module) = module {
                    modules.push(module);
                }
            }
            if !cpp_methods.is_empty() {
                log::error("unprocessed cpp methods left:");
                for method in cpp_methods {
                    log::error(format!("  {}", method.cpp_method.short_text()));
                    if let Some(ref info) = method.cpp_method.class_membership {
                        let rust_name = generator.calculate_rust_name(
                            &info.class_type.name,
                            &method.cpp_method.include_file,
                            false,
                            None,
                        )?;
                        log::error(format!("  -> {}", rust_name.full_name(None)));
                    } else {
                        let rust_name = generator.free_function_rust_name(&method.cpp_method)?;
                        log::error(format!("  -> {}", rust_name.full_name(None)));
                    }
                }
                return Err(unexpected("unprocessed cpp methods left").into());
            }
        }
        let mut any_not_declared = false;
        for type1 in &generator.processed_types {
            if !type1.is_declared_in(&modules) {
                log::error(format!("type is not processed: {:?}", type1));
                any_not_declared = true;
            }
        }
        if any_not_declared {
            return Err(unexpected("unprocessed cpp types left").into());
        }
        Ok(RustGeneratorOutput {
            ffi_functions: generator.generate_ffi_functions(),
            modules: modules,
            processed_types: generator.processed_types,
        })
    }
}

/// Output data of `RustGenerator::generate_type` function.
struct GenerateTypeResult {
    /// Rust declaration of the type passed to the function.
    main_type: RustTypeDeclaration,
    /// Rust declarations of the types created for overloading emulation.
    overloading_types: Vec<RustTypeDeclaration>,
}

/// Output data of `RustGenerator::process_all_sibling_functions` function.
#[derive(Default)]
struct ProcessFunctionsResult {
    /// Final Rust method wrappers
    methods: Vec<RustMethod>,
    /// Final trait implementations generated from some of the functions
    trait_impls: Vec<TraitImpl>,
    /// Rust declarations of the types created for overloading emulation.
    overloading_types: Vec<RustTypeDeclaration>,
}

impl<'aa> RustGenerator<'aa> {



    /// Generates `Drop` or `CppDeletable` trait implementation
    /// from a C++ destructor.
    fn process_destructor(
        &self,
        method: &CppAndFfiMethod,
        scope: &RustMethodScope,
    ) -> Result<TraitImpl> {
        if let RustMethodScope::Impl { ref target_type } = *scope {
            match method.allocation_place {
                ReturnValueAllocationPlace::Stack => {
                    let mut method = self.generate_rust_single_method(method, scope, true)?;
                    method.name = RustName::new(vec!["drop".to_string()])?;
                    method.scope = RustMethodScope::TraitImpl;
                    Ok(TraitImpl {
                        target_type: target_type.clone(),
                        associated_types: Vec::new(),
                        trait_type: RustType::Common {
                            base: RustName::new(vec!["Drop".to_string()])?,
                            indirection: RustTypeIndirection::None,
                            is_const: false,
                            is_const2: false,
                            generic_arguments: None,
                        },
                        extra: None,
                        methods: vec![method.to_rust_method()],
                    })
                }
                ReturnValueAllocationPlace::Heap => Ok(TraitImpl {
                    target_type: target_type.clone(),
                    associated_types: Vec::new(),
                    trait_type: RustType::Common {
                        base: RustName::new(vec![
                            "cpp_utils".to_string(),
                            "CppDeletable".to_string(),
                        ])?,
                        indirection: RustTypeIndirection::None,
                        is_const: false,
                        is_const2: false,
                        generic_arguments: None,
                    },
                    extra: Some(TraitImplExtra::CppDeletable {
                        deleter_name: method.c_name.clone(),
                    }),
                    methods: Vec::new(),
                }),
                ReturnValueAllocationPlace::NotApplicable => {
                    return Err(unexpected("destructor must have allocation place").into())
                }
            }
        } else {
            return Err(unexpected("destructor must be in class scope").into());
        }
    }

    /// Generates trait implementations from `static_cast`, `dynamic_cast`
    /// or `qobject_cast` (to be implemented) C++ function wrappers.
    fn process_cpp_cast(&self, method: RustSingleMethod) -> Result<Vec<TraitImpl>> {
        let mut results = Vec::new();
        // TODO: qobject_cast
        let mut final_methods = vec![(method.clone(), false), (method.clone(), true)];
        let args = &method.arguments;
        let cpp_cast = if let CppFfiMethodKind::Cast(ref cast) = args.cpp_method.kind {
            cast
        } else {
            return Err("not a cast method".into());
        };
        let trait_name = match *cpp_cast {
            CppCast::Static { ref is_unsafe, .. } => {
                if *is_unsafe {
                    vec!["cpp_utils".to_string(), "UnsafeStaticCast".to_string()]
                } else {
                    vec!["cpp_utils".to_string(), "StaticCast".to_string()]
                }
            }
            CppCast::Dynamic => vec!["cpp_utils".to_string(), "DynamicCast".to_string()],
            CppCast::QObject => vec![
                "qt_core".to_string(),
                "object".to_string(),
                "Cast".to_string(),
            ],
        };
        if args.arguments.len() != 1 {
            return Err(unexpected("1 argument expected").into());
        }
        let from_type = &args.arguments[0].argument_type;
        let to_type = &args.return_type;

        for &mut (ref mut final_method, ref mut final_is_const) in &mut final_methods {
            let method_name = if *final_is_const {
                args.cpp_method.cpp_method.name.clone()
            } else {
                format!("{}_mut", args.cpp_method.cpp_method.name)
            };
            final_method.scope = RustMethodScope::TraitImpl;
            final_method.name = RustName::new(vec![method_name])?;
            final_method.is_unsafe = cpp_cast.is_unsafe_static_cast();
            let return_ref_type = args.return_type.ptr_to_ref(*final_is_const)?;
            if &final_method.arguments.cpp_method.cpp_method.name == "static_cast" {
                final_method.arguments.return_type = return_ref_type;
            } else {
                final_method.arguments.return_type.rust_api_to_c_conversion =
                    RustToCTypeConversion::OptionRefToPtr;
                final_method.arguments.return_type.rust_api_type = RustType::Common {
                    base: RustName::new(vec![
                        "std".to_string(),
                        "option".to_string(),
                        "Option".to_string(),
                    ])?,
                    indirection: RustTypeIndirection::None,
                    is_const: false,
                    is_const2: false,
                    generic_arguments: Some(vec![return_ref_type.rust_api_type]),
                }
            };
            final_method.arguments.arguments[0].argument_type = final_method.arguments.arguments[0]
                .argument_type
                .ptr_to_ref(*final_is_const)?;
            final_method.arguments.arguments[0].name = "self".to_string();

            if !cpp_cast.is_unsafe_static_cast() && cpp_cast.is_direct_static_cast() {
                let mut deref_method = final_method.clone();
                deref_method.name = RustName::new(vec![if *final_is_const {
                    "deref"
                } else {
                    "deref_mut"
                }
                .to_string()])?;
                let deref_trait_name =
                    if *final_is_const { "Deref" } else { "DerefMut" }.to_string();
                let associated_types = if *final_is_const {
                    vec![TraitAssociatedType {
                        name: "Target".to_string(),
                        value: to_type.ptr_to_value()?.rust_api_type,
                    }]
                } else {
                    Vec::new()
                };
                results.push(TraitImpl {
                    target_type: from_type.ptr_to_value()?.rust_api_type,
                    associated_types: associated_types,
                    trait_type: RustType::Common {
                        base: RustName::new(vec![
                            "std".to_string(),
                            "ops".to_string(),
                            deref_trait_name,
                        ])?,
                        indirection: RustTypeIndirection::None,
                        is_const: false,
                        is_const2: false,
                        generic_arguments: None,
                    },
                    extra: None,
                    methods: vec![deref_method.to_rust_method()],
                });
            }
        }
        let trait_type = RustType::Common {
            base: RustName::new(trait_name)?,
            indirection: RustTypeIndirection::None,
            is_const: false,
            is_const2: false,
            generic_arguments: Some(vec![to_type.ptr_to_value()?.rust_api_type]),
        };
        results.push(TraitImpl {
            target_type: from_type.ptr_to_value()?.rust_api_type,
            associated_types: Vec::new(),
            trait_type: trait_type,
            extra: None,
            methods: final_methods
                .into_iter()
                .map(|x| x.0.to_rust_method())
                .collect(),
        });
        Ok(results)
    }

    /// Generates a single overloaded method from all specified methods or
    /// accepts a single method without change. Adds self argument caption if needed.
    /// All passed methods must be valid for overloading:
    /// - they must have the same name and be in the same scope;
    /// - they must have the same self argument type;
    /// - they must be all safe or all unsafe;
    /// - they must not have exactly the same argument types on any of target platforms.
    ///
    /// Use `RustGenerator::overload_functions` function to group available functions
    /// based on these conditions.
    fn generate_final_method(
        &self,
        mut filtered_methods: Vec<RustSingleMethod>,
        scope: &RustMethodScope,
        self_arg_kind_caption: Option<String>,
    ) -> Result<(RustMethod, Option<RustTypeDeclaration>)> {
        filtered_methods.sort_by(|a, b| {
            a.arguments
                .cpp_method
                .c_name
                .cmp(&b.arguments.cpp_method.c_name)
        });
        let methods_count = filtered_methods.len();
        let mut type_declaration = None;
        let method = if methods_count > 1 {
            let first_method = filtered_methods[0].clone();
            let self_argument = if !first_method.arguments.arguments.is_empty()
                && first_method.arguments.arguments[0].name == "self"
            {
                Some(first_method.arguments.arguments[0].clone())
            } else {
                None
            };
            let cpp_method_name = first_method.arguments.cpp_method.cpp_method.full_name();
            let mut args_variants = Vec::new();
            let mut method_name = first_method.name.clone();
            let mut method_last_name = method_name
                .parts
                .pop()
                .with_context(|| "name can't be empty")?;
            if let Some(self_arg_kind_caption) = self_arg_kind_caption {
                method_last_name =
                    vec![method_last_name.as_ref(), self_arg_kind_caption.as_ref()].to_snake_case();
            }
            let mut trait_name = method_last_name.to_class_case() + "Args";
            method_last_name = sanitize_rust_identifier(&method_last_name);
            method_name.parts.push(method_last_name);
            if let RustMethodScope::Impl { ref target_type } = *scope {
                let target_type_name = if let RustType::Common { ref base, .. } = *target_type {
                    base.last_name()
                } else {
                    Err("RustType::Common expected".into())
                }?;
                trait_name = format!("{}{}", target_type_name, trait_name);
            }
            let mut grouped_by_cpp_method: HashMap<_, Vec<_>> = HashMap::new();
            for mut method in filtered_methods {
                assert!(method.name == first_method.name);
                assert!(method.scope == first_method.scope);
                if let Some(ref self_argument) = self_argument {
                    assert!(
                        method.arguments.arguments.len() > 0
                            && &method.arguments.arguments[0] == self_argument
                    );
                    method.arguments.arguments.remove(0);
                }

                let cpp_method_key = method.arguments.cpp_method.cpp_method.clone();
                //        if let Some(v) = cpp_method_key.arguments_before_omitting {
                //          cpp_method_key.arguments = v;
                //          cpp_method_key.arguments_before_omitting = None;
                //        }
                add_to_multihash(
                    &mut grouped_by_cpp_method,
                    cpp_method_key,
                    method.arguments.clone(),
                );
                args_variants.push(method.arguments);
            }

            let mut doc_items = Vec::new();
            let mut grouped_by_cpp_method_vec: Vec<_> = grouped_by_cpp_method.into_iter().collect();
            grouped_by_cpp_method_vec
                .sort_by(|&(ref a, _), &(ref b, _)| a.short_text().cmp(&b.short_text()));
            for (cpp_method, variants) in grouped_by_cpp_method_vec {
                doc_items.push(RustMethodDocItem {
                    doc: cpp_method.doc.clone(),
                    cpp_fn: cpp_method.short_text(),
                    rust_fns: variants.iter().map_if_ok(|args| -> Result<_> {
                        Ok(doc_formatter::rust_method_variant(
                            args,
                            method_name.last_name()?,
                            first_method.self_arg_kind()?,
                            &self.input_data.crate_name,
                        ))
                    })?,
                });
            }

            // overloaded methods
            let shared_arguments_for_trait = match self_argument {
                None => Vec::new(),
                Some(ref arg) => {
                    let mut renamed_self = arg.clone();
                    renamed_self.name = "original_self".to_string();
                    vec![renamed_self]
                }
            };
            let mut shared_arguments = match self_argument {
                None => Vec::new(),
                Some(arg) => vec![arg],
            };
            let trait_lifetime_name = "largs";
            let mut has_trait_lifetime = shared_arguments
                .iter()
                .any(|x| x.argument_type.rust_api_type.is_ref());
            let first_return_type = args_variants[0].return_type.rust_api_type.clone();
            let common_return_type = if args_variants
                .iter()
                .all(|x| &x.return_type.rust_api_type == &first_return_type)
            {
                if first_return_type.is_ref() {
                    has_trait_lifetime = true;
                    Some(first_return_type.with_lifetime(trait_lifetime_name.to_string()))
                } else {
                    Some(first_return_type)
                }
            } else {
                None
            };
            if has_trait_lifetime {
                for arg in &mut shared_arguments {
                    if arg.argument_type.rust_api_type.is_ref() {
                        arg.argument_type.rust_api_type = arg
                            .argument_type
                            .rust_api_type
                            .with_lifetime(trait_lifetime_name.to_string());
                    }
                }
            }
            let params_trait_lifetime = if has_trait_lifetime {
                Some(trait_lifetime_name.to_string())
            } else {
                None
            };
            type_declaration = Some(RustTypeDeclaration {
                name: {
                    let mut name = first_method.name.clone();
                    name.parts.pop().unwrap();
                    name.parts.push("overloading".to_string());
                    name.parts.push(trait_name.clone());
                    name
                },
                kind: RustTypeDeclarationKind::MethodParametersTrait {
                    shared_arguments: shared_arguments_for_trait,
                    impls: args_variants,
                    lifetime: params_trait_lifetime.clone(),
                    common_return_type: common_return_type.clone(),
                    method_name: method_name.clone(),
                    method_scope: first_method.scope.clone(),
                    is_unsafe: first_method.is_unsafe,
                },
                is_public: true,
                rust_doc: None,
            });

            RustMethod {
                name: method_name,
                scope: first_method.scope,
                arguments: RustMethodArguments::MultipleVariants {
                    params_trait_name: trait_name.clone(),
                    params_trait_lifetime: params_trait_lifetime,
                    common_return_type: common_return_type,
                    shared_arguments: shared_arguments,
                    variant_argument_name: "args".to_string(),
                    cpp_method_name: cpp_method_name,
                },
                variant_docs: doc_items,
                common_doc: None,
                is_unsafe: first_method.is_unsafe,
            }
        } else {
            let mut method = filtered_methods
                .pop()
                .with_context(|| "filtered_methods can't be empty")?;
            let mut last_name = method
                .name
                .parts
                .pop()
                .with_context(|| "name can't be empty")?;
            if let Some(self_arg_kind_caption) = self_arg_kind_caption {
                last_name =
                    vec![last_name.as_ref(), self_arg_kind_caption.as_ref()].to_snake_case();
            }
            method.name.parts.push(sanitize_rust_identifier(&last_name));

            method.doc = Some(RustMethodDocItem {
                cpp_fn: method.arguments.cpp_method.cpp_method.short_text(),
                rust_fns: Vec::new(),
                doc: method.arguments.cpp_method.cpp_method.doc.clone(),
            });
            method.to_rust_method()
        };
        Ok((method, type_declaration))
    }

    /// Splits `methods` to groups based on overloading constraints.
    /// See `RustGenerator::generate_final_method` documentation for full list of these constraints.
    /// Each element of the returned vector contains a list of methods that
    /// can be safely overloaded together and a name suffix for these methods.
    fn overload_functions(
        &self,
        methods: Vec<RustSingleMethod>,
    ) -> Result<Vec<(Option<String>, Vec<RustSingleMethod>)>> {
        let mut buckets: Vec<Vec<RustSingleMethod>> = Vec::new();
        for method in methods {
            if let Some(b) = buckets
                .iter_mut()
                .find(|b| b.iter().all(|m| m.can_be_overloaded_with(&method).unwrap()))
            {
                b.push(method);
                continue;
            }
            buckets.push(vec![method]);
        }
        let mut all_self_args: HashSet<_> = HashSet::new();
        for bucket in &buckets {
            all_self_args.insert(bucket[0].self_arg_kind()?.clone());
        }

        let mut final_names = None;
        {
            let try_strategy = |strategy| -> Result<Vec<Option<String>>> {
                let mut result = Vec::new();
                for (bucket_index, bucket) in buckets.iter().enumerate() {
                    let mut bucket_caption: Option<Option<String>> = None;
                    for method in bucket {
                        let caption = method.name_suffix(strategy, &all_self_args, bucket_index)?;
                        if bucket_caption.is_none() {
                            bucket_caption = Some(caption);
                        } else if Some(caption) != bucket_caption {
                            return Err("different captions within a bucket".into());
                        }
                    }
                    let bucket_caption = bucket_caption.expect("can't be None here");
                    if result.iter().any(|c| c == &bucket_caption) {
                        return Err("same captions for two buckets".into());
                    }
                    result.push(bucket_caption);
                }
                Ok(result)
            };

            for strategy in RustMethodCaptionStrategy::all() {
                if let Ok(names) = try_strategy(&strategy) {
                    final_names = Some(names);
                    break;
                }
            }
        }
        if let Some(final_names) = final_names {
            return Ok(final_names.into_iter().zip(buckets.into_iter()).collect());
        } else {
            return Err(unexpected("all Rust caption strategies failed").into());
        }
    }

    /// Generates methods, trait implementations and overloading types
    /// for all specified methods. All methods must either be in the same
    /// `RustMethodScope::Impl` scope or be free functions in the same module.
    #[allow(clippy::for_kv_map)]
    fn process_all_sibling_functions<'b, I>(
        &self,
        methods: I,
        scope: &RustMethodScope,
    ) -> Result<ProcessFunctionsResult>
    where
        I: Iterator<Item = &'b CppAndFfiMethod>,
    {
        // Step 1: convert all methods to SingleVariant Rust methods and
        // split them by last name.
        let mut single_rust_methods: HashMap<String, Vec<RustSingleMethod>> = HashMap::new();
        let mut result = ProcessFunctionsResult::default();
        for method in methods {
            if method.cpp_method.is_destructor() {
                match self.process_destructor(method, scope) {
                    Ok(r) => result.trait_impls.push(r),
                    Err(msg) => log::llog(log::DebugRustSkips, || {
                        format!("Failed to generate destructor: {}\n{:?}\n", msg, method)
                    }),
                }
                continue;
            }
            match self.generate_rust_single_method(method, scope, false) {
                Ok(rust_method) => {
                    if (&method.cpp_method.name == "static_cast"
                        || &method.cpp_method.name == "dynamic_cast"
                        || &method.cpp_method.name == "qobject_cast")
                        && method.cpp_method.class_membership.is_none()
                    {
                        match self.process_cpp_cast(rust_method) {
                            Ok(mut r) => result.trait_impls.append(&mut r),
                            Err(msg) => log::llog(log::DebugRustSkips, || {
                                format!("Failed to generate cast wrapper: {}\n{:?}\n", msg, method)
                            }),
                        }
                    } else {
                        let name = rust_method.name.last_name()?.clone();
                        add_to_multihash(&mut single_rust_methods, name, rust_method);
                    }
                }
                Err(err) => log::llog(log::DebugRustSkips, || {
                    format!("failed to generate Rust function: {}", err)
                }),
            }
        }
        for (_, current_methods) in single_rust_methods {
            assert!(!current_methods.is_empty());

            for (name_suffix, overloaded_methods) in self.overload_functions(current_methods)? {
                let (method, type_declaration) =
                    self.generate_final_method(overloaded_methods, scope, name_suffix)?;
                if method.variant_docs.is_empty() {
                    return Err(unexpected(format!("docs are empty! {:?}", method)).into());
                }
                result.methods.push(method);
                if let Some(r) = type_declaration {
                    result.overloading_types.push(r);
                }
            }
        }
        result.methods.sort_by(|a, b| {
            a.name
                .last_name()
                .unwrap_or(&String::new())
                .cmp(b.name.last_name().unwrap_or(&String::new()))
        });
        result
            .trait_impls
            .sort_by(|a, b| a.trait_type.cmp(&b.trait_type));
        Ok(result)
    }

    /// Generates a Rust module with specified name from specified
    /// C++ header. If the module should have nested modules,
    /// this function calls itself recursively with nested module name
    /// but the same header data.
    pub fn generate_module<'a, 'b>(
        &'a self,
        mut cpp_methods: Vec<&'a CppAndFfiMethod>,
        module_name: &'b RustName,
    ) -> Result<(Option<RustModule>, Vec<&'a CppAndFfiMethod>)> {
        let mut direct_submodules = HashSet::new();
        let cpp_header = if module_name.parts.len() == 2 {
            self.top_module_names
                .iter()
                .find(|&(_k, v)| v == module_name)
                .and_then(|(k, _v)| Some(k.clone()))
        } else {
            None
        };
        let mut module = RustModule {
            name: module_name.last_name()?.clone(),
            types: Vec::new(),
            functions: Vec::new(),
            submodules: Vec::new(),
            trait_impls: Vec::new(),
            doc: if module_name.parts.len() >= 2 && module_name.parts[1] == "slots" {
                if module_name.parts.len() == 3 && module_name.parts[2] == "raw" {
                    Some(doc_formatter::slots_raw_module_doc())
                } else if module_name.parts.len() == 2 {
                    Some(doc_formatter::slots_module_doc())
                } else {
                    return Err(unexpected("unknown slots submodule").into());
                }
            } else {
                cpp_header
                    .as_ref()
                    .map(|h| format!("Entities from `{}` C++ header", h))
            },
        };
        let mut rust_overloading_types = Vec::new();
        let mut good_methods = Vec::new();
        {
            // Checks if the name should be processed.
            // Returns true if the name is directly in this module.
            // If the name is in this module's submodule, adds
            // name of the direct submodule to direct_submodules list.
            let mut check_name = |rust_name: &RustName| {
                if module_name.includes(rust_name) {
                    if module_name.includes_directly(rust_name) {
                        return true;
                    } else {
                        let direct_submodule = &rust_name.parts[module_name.parts.len()];
                        if !direct_submodules.contains(direct_submodule) {
                            direct_submodules.insert(direct_submodule.clone());
                        }
                    }
                }
                false
            };

            for type_data in &self.processed_types {
                if check_name(&type_data.rust_name) {
                    let (mut result, tmp_cpp_methods) =
                        self.generate_type(type_data, cpp_methods)?;
                    cpp_methods = tmp_cpp_methods;
                    if let Some(ref cpp_header) = cpp_header {
                        if &type_data.cpp_name == cpp_header {
                            if let RustTypeDeclarationKind::CppTypeWrapper { ref cpp_doc, .. } =
                                result.main_type.kind
                            {
                                if let Some(ref cpp_doc) = *cpp_doc {
                                    let mut doc = cpp_doc.html.as_str();
                                    if let Some(index) = doc.find("\n") {
                                        doc = &doc[0..index];
                                    }
                                    module.doc = Some(doc.to_string());
                                }
                            }
                        }
                    }
                    doc_formatter::add_special_type_docs(&mut result.main_type)?;
                    module.types.push(result.main_type);
                    rust_overloading_types.append(&mut result.overloading_types);
                }
            }

            let mut tmp_cpp_methods = Vec::new();
            for method in cpp_methods {
                if method.cpp_method.class_membership.is_none() {
                    let rust_name = self.free_function_rust_name(&method.cpp_method)?;

                    if check_name(&rust_name) {
                        good_methods.push(method);
                        continue;
                    }
                }
                tmp_cpp_methods.push(method);
            }
            cpp_methods = tmp_cpp_methods;
        }
        for name in direct_submodules {
            let mut new_name = module_name.clone();
            new_name.parts.push(name);
            let (submodule, tmp_cpp_methods) = self.generate_module(cpp_methods, &new_name)?;
            cpp_methods = tmp_cpp_methods;
            if let Some(submodule) = submodule {
                module.submodules.push(submodule);
            }
        }
        let mut free_functions_result =
            self.process_all_sibling_functions(good_methods.into_iter(), &RustMethodScope::Free)?;
        module.trait_impls = free_functions_result.trait_impls;
        module.functions = free_functions_result.methods;
        rust_overloading_types.append(&mut free_functions_result.overloading_types);
        if !rust_overloading_types.is_empty() {
            rust_overloading_types.sort_by(|a, b| a.name.cmp(&b.name));
            module.submodules.push(RustModule {
                name: "overloading".to_string(),
                types: rust_overloading_types,
                functions: Vec::new(),
                submodules: Vec::new(),
                trait_impls: Vec::new(),
                doc: Some(doc_formatter::overloading_module_doc()),
            });
        }
        module.types.sort_by(|a, b| a.name.cmp(&b.name));
        module.submodules.sort_by(|a, b| a.name.cmp(&b.name));
        if module.types.is_empty() && module.functions.is_empty() && module.submodules.is_empty() {
            log::llog(log::DebugRustSkips, || {
                format!("Skipping empty module: {}", module.name)
            });
            return Ok((None, cpp_methods));
        }
        Ok((Some(module), cpp_methods))
    }

    /// Generates Rust representations of all FFI functions
    pub fn generate_ffi_functions(&self) -> Vec<(String, Vec<RustFFIFunction>)> {
        log::status("Generating Rust FFI functions");
        let mut ffi_functions = Vec::new();

        for header in &self.input_data.cpp_ffi_headers {
            let mut functions = Vec::new();
            for method in &header.methods {
                match self.generate_ffi_function(method) {
                    Ok(function) => {
                        functions.push(function);
                    }
                    Err(msg) => {
                        log::llog(log::DebugRustSkips, || {
                            format!(
                                "Can't generate Rust FFI function for method:\n{}\n{}\n",
                                method.short_text(),
                                msg
                            )
                        });
                    }
                }
            }
            ffi_functions.push((header.include_file_base_name.clone(), functions));
        }
        ffi_functions
    }

    /// Generates Rust names and type information for all available C++ types.
    fn calc_processed_types(&self) -> Result<Vec<RustProcessedTypeInfo>> {
        let mut result = Vec::new();
        for type_info in &self.input_data.cpp_data.current.parser.types {
            if let CppTypeKind::Class {
                ref template_arguments,
                ..
            } = type_info.kind
            {
                if template_arguments.is_some() {
                    continue;
                }
            }
            let rust_name =
                self.calculate_rust_name(&type_info.name, &type_info.include_file, false, None)?;
            let rust_type_info = RustProcessedTypeInfo {
                cpp_name: type_info.name.clone(),
                cpp_doc: type_info.doc.clone(),
                cpp_template_arguments: None,
                kind: match type_info.kind {
                    CppTypeKind::Class { .. } => match self
                        .input_data
                        .cpp_data
                        .type_allocation_place(&type_info.name)
                    {
                        Err(err) => {
                            log::llog(log::DebugRustSkips, || {
                                format!("Can't process type: {}: {}", type_info.name, err)
                            });
                            continue;
                        }
                        Ok(place) => RustTypeWrapperKind::Struct {
                            size_const_name: match place {
                                CppTypeAllocationPlace::Stack => Some(size_const_name(&rust_name)),
                                CppTypeAllocationPlace::Heap => None,
                            },
                            slot_wrapper: None,
                        },
                    },
                    CppTypeKind::Enum { ref values } => {
                        let mut is_flaggable = false;
                        let template_arg_sample = CppType {
                            is_const: false,
                            is_const2: false,
                            indirection: CppTypeIndirection::None,
                            base: CppTypeBase::Enum {
                                name: type_info.name.clone(),
                            },
                        };

                        for flag_owner_name in &["QFlags", "QUrlTwoFlags"] {
                            if let Some(instantiations) = self
                                .input_data
                                .cpp_data
                                .current
                                .processed
                                .template_instantiations
                                .iter()
                                .find(|x| &x.class_name == &flag_owner_name.to_string())
                            {
                                if instantiations.instantiations.iter().any(|ins| {
                                    ins.template_arguments
                                        .iter()
                                        .any(|arg| arg == &template_arg_sample)
                                }) {
                                    is_flaggable = true;
                                    break;
                                }
                            }
                        }
                        RustTypeWrapperKind::Enum {
                            values: prepare_enum_values(values),
                            is_flaggable: is_flaggable,
                        }
                    }
                },
                rust_name: rust_name,
                is_public: true,
            };
            result.push(rust_type_info);
        }
        let template_final_name = |result: &Vec<RustProcessedTypeInfo>,
                                   item: &RustProcessedTypeInfo|
         -> Result<RustName> {
            let mut name = item.rust_name.clone();
            let last_name = name
                .parts
                .pop()
                .with_context(|| "name.parts can't be empty")?;
            let mut arg_captions = Vec::new();
            if let Some(ref args) = item.cpp_template_arguments {
                for x in args {
                    let rust_type = complete_type(
                        result,
                        &self.input_data.dependency_types,
                        &x.to_cpp_ffi_type(CppTypeRole::NotReturnType)?,
                        &CppFfiArgumentMeaning::Argument(0),
                        true,
                        &ReturnValueAllocationPlace::NotApplicable,
                    )?;
                    arg_captions.push(rust_type.rust_api_type.caption(&name)?.to_class_case());
                }
            } else {
                return Err("template arguments expected".into());
            }
            name.parts.push(last_name + &arg_captions.join(""));
            Ok(name)
        };
        let mut unnamed_items = Vec::new();
        for template_instantiations in &self
            .input_data
            .cpp_data
            .current
            .processed
            .template_instantiations
        {
            let type_info = self
                .input_data
                .cpp_data
                .find_type_info(|x| &x.name == &template_instantiations.class_name)
                .with_context(|| {
                    format!(
                        "type info not found for {}",
                        &template_instantiations.class_name
                    )
                })?;
            if template_instantiations.class_name == "QFlags" {
                // special processing is implemented for QFlags
                continue;
            }
            for ins in &template_instantiations.instantiations {
                let rust_name = self.calculate_rust_name(
                    &template_instantiations.class_name,
                    &type_info.include_file,
                    false,
                    None,
                )?;
                unnamed_items.push(RustProcessedTypeInfo {
                    cpp_name: template_instantiations.class_name.clone(),
                    cpp_doc: type_info.doc.clone(),
                    cpp_template_arguments: Some(ins.template_arguments.clone()),
                    kind: RustTypeWrapperKind::Struct {
                        size_const_name: None,
                        slot_wrapper: None,
                    },
                    rust_name: rust_name,
                    is_public: true,
                });
            }
        }
        let mut any_success = true;
        while !unnamed_items.is_empty() {
            if !any_success {
                log::error("Failed to generate Rust names for template types:");
                for r in unnamed_items {
                    log::error(format!(
                        "  {:?}\n  {}\n\n",
                        r,
                        if let Err(err) = template_final_name(&result, &r) {
                            err
                        } else {
                            return Err("template_final_name must return Err at this stage".into());
                        }
                    ));
                }
                break;
            }
            any_success = false;
            let mut unnamed_items_new = Vec::new();
            for mut r in unnamed_items {
                match template_final_name(&result, &r) {
                    Ok(name) => {
                        r.rust_name = name.clone();
                        if let RustTypeWrapperKind::Struct {
                            ref mut size_const_name,
                            ..
                        } = r.kind
                        {
                            match self.input_data.cpp_data.type_allocation_place(&r.cpp_name) {
                                Err(err) => {
                                    log::log(
                                        log::DebugRustSkips,
                                        format!("Can't process type: {}: {}", r.cpp_name, err),
                                    );
                                    continue;
                                }
                                Ok(place) => {
                                    *size_const_name = match place {
                                        CppTypeAllocationPlace::Stack => {
                                            Some(self::size_const_name(&name))
                                        }
                                        CppTypeAllocationPlace::Heap => None,
                                    };
                                }
                            }
                        } else {
                            unreachable!();
                        }
                        result.push(r);
                        any_success = true;
                    }
                    Err(_) => unnamed_items_new.push(r),
                }
            }
            unnamed_items = unnamed_items_new;
        }
        for header in &self.input_data.cpp_ffi_headers {
            for qt_slot_wrapper in &header.qt_slot_wrappers {
                let incomplete_rust_name = self.calculate_rust_name(
                    &format!("raw_slot"),
                    &header.include_file_base_name,
                    false,
                    None,
                )?;
                let arg_names = qt_slot_wrapper
                    .arguments
                    .iter()
                    .map_if_ok(|x| -> Result<_> {
                        let rust_type = complete_type(
                            &result,
                            &self.input_data.dependency_types,
                            x,
                            &CppFfiArgumentMeaning::Argument(0),
                            false,
                            &ReturnValueAllocationPlace::NotApplicable,
                        )?;
                        rust_type.rust_api_type.caption(&incomplete_rust_name)
                    })?;
                let args_text = if arg_names.is_empty() {
                    "no_args".to_string()
                } else {
                    arg_names.join("_")
                };
                let rust_type_info = RustProcessedTypeInfo {
                    cpp_name: qt_slot_wrapper.class_name.clone(),
                    cpp_template_arguments: None,
                    cpp_doc: None, // TODO: do we need doc for this?
                    rust_name: self.calculate_rust_name(
                        &format!("raw_slot_{}", args_text),
                        &header.include_file_base_name,
                        false,
                        None,
                    )?,
                    is_public: true,
                    kind: RustTypeWrapperKind::Struct {
                        size_const_name: None,
                        is_deletable: true,
                        slot_wrapper: Some(RustQtSlotWrapper {
                            arguments: qt_slot_wrapper.arguments.iter().map_if_ok(
                                |t| -> Result<_> {
                                    let mut t = complete_type(
                                        &result,
                                        &self.input_data.dependency_types,
                                        t,
                                        &CppFfiArgumentMeaning::Argument(0),
                                        false,
                                        &ReturnValueAllocationPlace::NotApplicable,
                                    )?;
                                    t.rust_api_type =
                                        t.rust_api_type.with_lifetime("static".to_string());
                                    Ok(t)
                                },
                            )?,
                            receiver_id: qt_slot_wrapper.receiver_id.clone(),
                            public_type_name: format!("slot_{}", args_text).to_class_case(),
                            callback_name: format!("slot_{}_callback", args_text).to_snake_case(),
                        }),
                    },
                };
                result.push(rust_type_info);
            }
        }
        Ok(result)
    }


}

// ---------------------------------
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

#[cfg(test)]
fn calculate_rust_name_test_part(
    name: &'static str,
    include_file: &'static str,
    is_function: bool,
    expected: &[&'static str],
) {
    let header = ::cpp_ffi_data::CppFfiHeaderData {
        include_file_base_name: include_file.to_string(),
        methods: Vec::new(),
        qt_slot_wrappers: Vec::new(),
    };
    let mut generator = RustGenerator {
        top_module_names: HashMap::new(),
        processed_types: Vec::new(),
        input_data: RustGeneratorInputData {
            cpp_ffi_headers: vec![header],
            cpp_data: &Default::default(),
            dependency_types: Vec::new(),
            crate_name: "qt_core".to_string(),
            remove_qt_prefix: true,
            filtered_namespaces: Vec::new(),
        },
    };
    generator.top_module_names = generator.calc_top_module_names().unwrap();

    assert_eq!(
        generator
            .calculate_rust_name(
                &name.to_string(),
                &include_file.to_string(),
                is_function,
                None
            )
            .unwrap(),
        RustName::new(expected.into_iter().map(|x| x.to_string()).collect()).unwrap()
    );
}

#[test]
fn calculate_rust_name_test() {
    calculate_rust_name_test_part(
        "myFunc1",
        "QtGlobal",
        true,
        &["qt_core", "global", "my_func1"],
    );
    calculate_rust_name_test_part(
        "QPointF",
        "QPointF",
        false,
        &["qt_core", "point_f", "PointF"],
    );
    calculate_rust_name_test_part(
        "QStringList::Iterator",
        "QStringList",
        false,
        &["qt_core", "string_list", "Iterator"],
    );
    calculate_rust_name_test_part(
        "QStringList::Iterator",
        "QString",
        false,
        &["qt_core", "string", "string_list", "Iterator"],
    );
    calculate_rust_name_test_part(
        "ns::func1",
        "QRect",
        true,
        &["qt_core", "rect", "ns", "func1"],
    );
}

#[test]
fn prepare_enum_values_test_simple() {
    let r = prepare_enum_values(&[
        CppEnumValue {
            name: "var1".to_string(),
            value: 1,
            doc: None,
        },
        CppEnumValue {
            name: "other_var2".to_string(),
            value: 2,
            doc: None,
        },
    ]);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].name, "Var1");
    assert_eq!(r[0].value, 1);
    assert_eq!(r[1].name, "OtherVar2");
    assert_eq!(r[1].value, 2);
}

#[test]
fn prepare_enum_values_test_duplicates() {
    let r = prepare_enum_values(&[
        CppEnumValue {
            name: "var1".to_string(),
            value: 1,
            doc: None,
        },
        CppEnumValue {
            name: "other_var2".to_string(),
            value: 2,
            doc: None,
        },
        CppEnumValue {
            name: "other_var_dup".to_string(),
            value: 2,
            doc: None,
        },
    ]);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].name, "Var1");
    assert_eq!(r[0].value, 1);
    assert_eq!(r[1].name, "OtherVar2");
    assert_eq!(r[1].value, 2);
}

#[test]
fn prepare_enum_values_test_prefix() {
    let r = prepare_enum_values(&[
        CppEnumValue {
            name: "OptionGood".to_string(),
            value: 1,
            doc: None,
        },
        CppEnumValue {
            name: "OptionBad".to_string(),
            value: 2,
            doc: None,
        },
        CppEnumValue {
            name: "OptionNecessaryEvil".to_string(),
            value: 3,
            doc: None,
        },
    ]);
    assert_eq!(r.len(), 3);
    assert_eq!(r[0].name, "Good");
    assert_eq!(r[1].name, "Bad");
    assert_eq!(r[2].name, "NecessaryEvil");
}

#[test]
fn prepare_enum_values_test_suffix() {
    let r = prepare_enum_values(&[
        CppEnumValue {
            name: "BestFriend".to_string(),
            value: 1,
            doc: None,
        },
        CppEnumValue {
            name: "GoodFriend".to_string(),
            value: 2,
            doc: None,
        },
        CppEnumValue {
            name: "NoFriend".to_string(),
            value: 3,
            doc: None,
        },
    ]);
    assert_eq!(r.len(), 3);
    assert_eq!(r[0].name, "Best");
    assert_eq!(r[1].name, "Good");
    assert_eq!(r[2].name, "No");
}

#[test]
fn prepare_enum_values_test_prefix_digits() {
    let r = prepare_enum_values(&[
        CppEnumValue {
            name: "Base32".to_string(),
            value: 1,
            doc: None,
        },
        CppEnumValue {
            name: "Base64".to_string(),
            value: 2,
            doc: None,
        },
    ]);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].name, "Base32");
    assert_eq!(r[1].name, "Base64");
}

#[test]
fn prepare_enum_values_test_suffix_empty() {
    let r = prepare_enum_values(&[
        CppEnumValue {
            name: "NonRecursive".to_string(),
            value: 1,
            doc: None,
        },
        CppEnumValue {
            name: "Recursive".to_string(),
            value: 2,
            doc: None,
        },
    ]);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].name, "NonRecursive");
    assert_eq!(r[1].name, "Recursive");
}

#[test]
fn prepare_enum_values_test_suffix_partial() {
    let r = prepare_enum_values(&[
        CppEnumValue {
            name: "PreciseTimer".to_string(),
            value: 1,
            doc: None,
        },
        CppEnumValue {
            name: "CoarseTimer".to_string(),
            value: 2,
            doc: None,
        },
    ]);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].name, "Precise");
    assert_eq!(r[1].name, "Coarse");
}

impl RustSingleMethod {
    /// Converts this method to a final Rust method
    /// without overloading.
    fn to_rust_method(&self) -> RustMethod {
        RustMethod {
            name: self.name.clone(),
            arguments: RustMethodArguments::SingleVariant(self.arguments.clone()),
            variant_docs: if let Some(ref doc) = self.doc {
                vec![doc.clone()]
            } else {
                Vec::new()
            },
            common_doc: None,
            is_unsafe: self.is_unsafe,
            scope: self.scope.clone(),
        }
    }

    /// Returns information about `self` argument of this method.
    fn self_arg_kind(&self) -> Result<RustMethodSelfArgKind> {
        Ok(if let Some(arg) = self.arguments.arguments.get(0) {
            if arg.name == "self" {
                if let RustType::Common {
                    ref indirection,
                    ref is_const,
                    ..
                } = arg.argument_type.rust_api_type
                {
                    match *indirection {
                        RustTypeIndirection::Ref { .. } => {
                            if *is_const {
                                RustMethodSelfArgKind::ConstRef
                            } else {
                                RustMethodSelfArgKind::MutRef
                            }
                        }
                        RustTypeIndirection::None => RustMethodSelfArgKind::Value,
                        _ => return Err(unexpected("invalid self argument type").into()),
                    }
                } else {
                    return Err(unexpected("invalid self argument type").into());
                }
            } else {
                RustMethodSelfArgKind::None
            }
        } else {
            RustMethodSelfArgKind::None
        })
    }

    /// Generates name suffix for this method using `caption_strategy`.
    /// `all_self_args` should contain all kinds of arguments found in
    /// the methods that have to be disambiguated using the name suffix.
    /// `index` is number of the method used in `RustMethodCaptionStrategy::Index`.
    fn name_suffix(
        &self,
        caption_strategy: &RustMethodCaptionStrategy,
        all_self_args: &HashSet<RustMethodSelfArgKind>,
        index: usize,
    ) -> Result<Option<String>> {
        if caption_strategy == &RustMethodCaptionStrategy::UnsafeOnly {
            return Ok(if self.is_unsafe {
                Some("unsafe".to_string())
            } else {
                None
            });
        }
        let result = {
            let self_arg_kind = self.self_arg_kind()?;
            let self_arg_kind_caption =
                if all_self_args.len() == 1 || self_arg_kind == RustMethodSelfArgKind::ConstRef {
                    None
                } else if self_arg_kind == RustMethodSelfArgKind::None {
                    Some("static")
                } else if self_arg_kind == RustMethodSelfArgKind::MutRef {
                    if all_self_args.contains(&RustMethodSelfArgKind::ConstRef) {
                        Some("mut")
                    } else {
                        None
                    }
                } else {
                    return Err("unsupported self arg kinds combination".into());
                };
            let other_caption = match *caption_strategy {
                RustMethodCaptionStrategy::SelfOnly => None,
                RustMethodCaptionStrategy::UnsafeOnly => unreachable!(),
                RustMethodCaptionStrategy::SelfAndIndex => Some(index.to_string()),
                RustMethodCaptionStrategy::SelfAndArgNames => {
                    if self.arguments.arguments.is_empty() {
                        Some("no_args".to_string())
                    } else {
                        Some(self.arguments.arguments.iter().map(|a| &a.name).join("_"))
                    }
                }
                RustMethodCaptionStrategy::SelfAndArgTypes => {
                    let context = match self.scope {
                        RustMethodScope::Free => &self.name,
                        RustMethodScope::Impl { ref target_type } => {
                            if let RustType::Common { ref base, .. } = *target_type {
                                base
                            } else {
                                return Err("unexpected uncommon Rust type".into());
                            }
                        }
                        RustMethodScope::TraitImpl => {
                            return Err(
                                "can't generate Rust method caption for a trait impl method".into()
                            )
                        }
                    };

                    if self.arguments.arguments.is_empty() {
                        Some("no_args".to_string())
                    } else {
                        Some(
                            self.arguments
                                .arguments
                                .iter()
                                .filter(|t| &t.name != "self")
                                .map_if_ok(|t| t.argument_type.rust_api_type.caption(context))?
                                .join("_"),
                        )
                    }
                }
            };
            let mut key_caption_items = Vec::new();
            if let Some(c) = self_arg_kind_caption {
                key_caption_items.push(c.to_string());
            }
            if let Some(c) = other_caption {
                key_caption_items.push(c);
            }
            if key_caption_items.is_empty() {
                None
            } else {
                Some(key_caption_items.join("_"))
            }
        };
        Ok(result)
    }
}
