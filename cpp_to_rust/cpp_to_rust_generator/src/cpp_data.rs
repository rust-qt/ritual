//! Types for handling information about C++ library APIs.


use cpp_method::{CppMethod, CppMethodKind, CppMethodClassMembership, CppFunctionArgument,
                 CppFieldAccessorType, FakeCppMethod};
use cpp_operator::CppOperator;
use cpp_type::{CppType, CppTypeBase, CppTypeIndirection, CppTypeClassBase};
use common::errors::{Result, ChainErr, unexpected};
use common::file_utils::open_file;
use common::log;

use std::collections::{HashSet, HashMap};
use std::iter::once;
use std::io::{BufRead, BufReader};
use common::string_utils::JoinWithSeparator;

use regex::Regex;

/// One item of a C++ enum declaration
#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct CppEnumValue {
  /// Identifier
  pub name: String,
  /// Corresponding value
  pub value: i64,
  /// C++ documentation for this item in HTML
  pub doc: Option<String>,
}

/// Member field of a C++ class declaration
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
#[derive(Serialize, Deserialize)]
pub struct CppClassField {
  /// Identifier
  pub name: String,
  /// Field type
  pub field_type: CppType,
  /// Visibility
  pub visibility: CppVisibility,
  /// Size of type in bytes
  pub size: Option<usize>,
}

/// A "using" directive inside a class definition,
/// indicating that the class should inherite a
/// certain method of a base class.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
#[derive(Serialize, Deserialize)]
pub struct CppClassUsingDirective {
  /// Name of the base class
  pub class_name: String,
  /// Name of the method
  pub method_name: String,
}

/// Item of base class list in a class declaration
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
#[derive(Serialize, Deserialize)]
pub struct CppBaseSpecifier {
  /// Base class type (can include template arguments)
  pub base_type: CppType,
  /// True if this base is virtual
  pub is_virtual: bool,
  /// Base visibility (public, protected or private)
  pub visibility: CppVisibility,
}


/// Information about a C++ type declaration
#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub enum CppTypeKind {
  /// Enum declaration
  Enum {
    /// List of items
    values: Vec<CppEnumValue>,
  },
  /// Class declaration
  Class {
    /// List of class types this class is derived from
    bases: Vec<CppBaseSpecifier>,
    /// List of class fields
    fields: Vec<CppClassField>,
    /// Information about template arguments of this type.
    template_arguments: Option<TemplateArgumentsDeclaration>,
    /// List of using directives, like "using BaseClass::method1;"
    using_directives: Vec<CppClassUsingDirective>,
  },
}

/// Location of a C++ type's definition in header files.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
#[derive(Serialize, Deserialize)]
pub struct CppOriginLocation {
  // Full path to the include file
  pub include_file_path: String,
  /// Line of the file
  pub line: u32,
  /// Column of the file
  pub column: u32,
}

/// Visibility of a C++ entity. Defaults to `Public`
/// for entities that can't have visibility (like free functions)
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
#[derive(Serialize, Deserialize)]
pub enum CppVisibility {
  Public,
  Protected,
  Private,
}

/// C++ documentation for a type
#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct CppTypeDoc {
  /// HTML content
  pub html: String,
  /// Absolute URL to online documentation page for this type
  pub url: String,
  /// Absolute documentation URLs encountered in the content
  pub cross_references: Vec<String>,
}

/// Information about a C++ type declaration
#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct CppTypeData {
  /// Identifier, including namespaces and nested classes
  /// (separated with "::", like in C++)
  pub name: String,
  /// File name of the include file (without full path)
  pub include_file: String,
  /// Exact location of the declaration
  pub origin_location: CppOriginLocation,
  /// Type information
  pub kind: CppTypeKind,
  /// C++ documentation data for this type
  pub doc: Option<CppTypeDoc>,
}

/// Information about template arguments of a C++ class type
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
#[derive(Serialize, Deserialize)]
pub struct TemplateArgumentsDeclaration {
  /// Indicates how many template types this type is nested into.
  ///
  /// In the following example class `A`
  /// has level 0, and class `B` has level 1.
  ///
  /// ```C++
  /// template<class T>
  /// class A {
  ///   template<class T2>
  ///   class B {};
  /// };
  /// ```
  pub nested_level: usize,
  /// Names of template arguments. Names themselves are
  /// not particularly important, but their count is.
  pub names: Vec<String>,
}

/// Information about a C++ template class
/// instantiation.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[derive(Serialize, Deserialize)]
pub struct CppTemplateInstantiation {
  /// List of template arguments used in this instantiation
  pub template_arguments: Vec<CppType>,
}

/// List of template instantiations of
/// a template class.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[derive(Serialize, Deserialize)]
pub struct CppTemplateInstantiations {
  /// Template class name
  pub class_name: String,
  /// List of encountered instantiations
  pub instantiations: Vec<CppTemplateInstantiation>,
}

/// Type allocation place of a C++ type.
///
/// The generator chooses type allocation place for each C++ type based on the library's API.
/// This value can be overriden using `Config::set_type_allocation_place`.
///
/// See `cpp_to_rust_generator`'s `README.md` for detailed description.
#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub enum CppTypeAllocationPlace {
  /// Values are stored on C++ heap and used as `CppBox<T>`.
  Heap,
  /// Values are stored on Rust stack and used as `T`.
  Stack,
}

/// C++ parser output
#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[derive(Serialize, Deserialize)]
pub struct ParserCppData {
  /// List of found type declarations
  pub types: Vec<CppTypeData>,
  /// List of found methods
  pub methods: Vec<CppMethod>,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[derive(Serialize, Deserialize)]
pub struct ProcessedCppData {
  /// Automatically generated methods
  pub extra_methods: Vec<CppMethod>,
  /// Methods inherited from base classes (?)
  pub inherited_methods: Vec<CppMethod>,
  /// List of found template instantiations. Key is name of
  /// the template class, value is list of instantiations.
  pub template_instantiations: Vec<CppTemplateInstantiations>,
  /// List of all argument types used by signals,
  /// including variations with omitted arguments,
  /// but excluding argument types from dependencies.
  pub signal_argument_types: Vec<Vec<CppType>>,
  /// List of selected (automatically or in configuration)
  /// type allocation places for all class types.
  pub type_allocation_places: HashMap<String, CppTypeAllocationPlace>,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct CppData {
  pub parser: ParserCppData,
  pub processed: ProcessedCppData,
}

impl CppData {
  pub fn all_methods
    (&self)
     -> ::std::iter::Chain<::std::slice::Iter<CppMethod>, ::std::slice::Iter<CppMethod>> {
    self
      .parser
      .methods
      .iter()
      .chain(self.processed.extra_methods.iter())
  }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct CppDataWithDeps {
  pub current: CppData,
  /// Data of dependencies
  pub dependencies: Vec<CppData>,
}





/// Convenience function to create `CppMethod` object for
/// `static_cast` or `dynamic_cast` from type `from` to type `to`.
/// See `CppMethod`'s documentation for more information
/// about `is_unsafe_static_cast` and `is_direct_static_cast`.
pub fn create_cast_method(name: &str,
                          from: &CppType,
                          to: &CppType,
                          is_unsafe_static_cast: bool,
                          is_direct_static_cast: bool,
                          include_file: &str)
                          -> CppMethod {
  CppMethod {
    name: name.to_string(),
    class_membership: None,
    operator: None,
    return_type: to.clone(),
    arguments: vec![CppFunctionArgument {
                      name: "ptr".to_string(),
                      argument_type: from.clone(),
                      has_default_value: false,
                    }],
    arguments_before_omitting: None,
    allows_variadic_arguments: false,
    include_file: include_file.to_string(),
    origin_location: None,
    template_arguments: None,
    template_arguments_values: Some(vec![to.clone()]),
    declaration_code: None,
    doc: None,
    inheritance_chain: Vec::new(),
    //is_fake_inherited_method: false,
    is_ffi_whitelisted: true,
    is_unsafe_static_cast: is_unsafe_static_cast,
    is_direct_static_cast: is_direct_static_cast,
  }
}

/// Tries to apply each of `template_instantiations` to `method`.
/// Only types at the specified `nested_level` are replaced.
/// Returns `Err` if any of `template_instantiations` is incompatible
/// with the method.
fn apply_instantiations_to_method(method: &CppMethod,
                                  nested_level: usize,
                                  template_instantiations: &[CppTemplateInstantiation])
                                  -> Result<Vec<CppMethod>> {
  let mut new_methods = Vec::new();
  for ins in template_instantiations {
    log::llog(log::DebugTemplateInstantiation,
              || format!("instantiation: {:?}", ins.template_arguments));
    let mut new_method = method.clone();
    if let Some(ref args) = method.template_arguments {
      if args.nested_level == nested_level {
        if args.count() != ins.template_arguments.len() {
          return Err("template arguments count mismatch".into());
        }
        new_method.template_arguments = None;
        new_method.template_arguments_values = Some(ins.template_arguments.clone());
      }
    }
    new_method.arguments.clear();
    for arg in &method.arguments {
      new_method
        .arguments
        .push(CppFunctionArgument {
                name: arg.name.clone(),
                has_default_value: arg.has_default_value,
                argument_type: arg
                  .argument_type
                  .instantiate(nested_level, &ins.template_arguments)?,
              });
    }
    if let Some(ref args) = method.arguments_before_omitting {
      let mut new_args = Vec::new();
      for arg in args {
        new_args.push(CppFunctionArgument {
                        name: arg.name.clone(),
                        has_default_value: arg.has_default_value,
                        argument_type: arg
                          .argument_type
                          .instantiate(nested_level, &ins.template_arguments)?,
                      });
      }
      new_method.arguments_before_omitting = Some(new_args);
    }
    new_method.return_type = method
      .return_type
      .instantiate(nested_level, &ins.template_arguments)?;
    if let Some(ref mut info) = new_method.class_membership {
      info.class_type = info
        .class_type
        .instantiate_class(nested_level, &ins.template_arguments)?;
    }
    let mut conversion_type = None;
    if let Some(ref mut operator) = new_method.operator {
      if let CppOperator::Conversion(ref mut cpp_type) = *operator {
        let r = cpp_type
          .instantiate(nested_level, &ins.template_arguments)?;
        *cpp_type = r.clone();
        conversion_type = Some(r);
      }
    }
    if new_method
         .all_involved_types()
         .iter()
         .any(|t| t.base.is_or_contains_template_parameter()) {
      return Err(format!("extra template parameters left: {}",
                         new_method.short_text())
                     .into());
    } else {
      if let Some(conversion_type) = conversion_type {
        new_method.name = format!("operator {}", conversion_type.to_cpp_code(None)?);
      }
      log::llog(log::DebugTemplateInstantiation,
                || format!("success: {}", new_method.short_text()));
      new_methods.push(new_method);
    }
  }
  Ok(new_methods)
}

impl CppTypeData {
  /// Checks if the type is a class type.
  pub fn is_class(&self) -> bool {
    match self.kind {
      CppTypeKind::Class { .. } => true,
      _ => false,
    }
  }

  /// Creates CppTypeBase object representing type
  /// of an object of this type. See
  /// default_template_arguments() documentation
  /// for details about handling template parameters.
  pub fn default_class_type(&self) -> Result<CppTypeClassBase> {
    if !self.is_class() {
      return Err("not a class".into());
    }
    Ok(CppTypeClassBase {
         name: self.name.clone(),
         template_arguments: self.default_template_arguments(),
       })
  }

  /// Creates template parameters expected for this type.
  /// For example, QHash<QString, int> will have 2 default
  /// template parameters with indexes 0 and 1. This function
  /// is helpful for determining type of "this" pointer.
  /// Result of this function may differ from actual template
  /// parameters, for example:
  /// - if a class is inside another template class,
  /// nested level should be 1 instead of 0;
  /// - if QList<V> type is used inside QHash<K, V> type,
  /// QList's template parameter will have index = 1
  /// instead of 0.
  pub fn default_template_arguments(&self) -> Option<Vec<CppType>> {
    match self.kind {
      CppTypeKind::Class { ref template_arguments, .. } => {
        match *template_arguments {
          None => None,
          Some(ref arguments) => {
            Some(arguments
                   .names
                   .iter()
                   .enumerate()
                   .map(|(num, _)| {
              CppType {
                is_const: false,
                is_const2: false,
                indirection: CppTypeIndirection::None,
                base: CppTypeBase::TemplateParameter {
                  nested_level: arguments.nested_level,
                  index: num,
                },
              }
            })
                   .collect())
          }
        }
      }
      _ => None,
    }
  }

  /// Checks if the type was directly derived from specified type.
  #[allow(dead_code)]
  pub fn inherits_directly(&self, class_name: &str) -> bool {
    if let CppTypeKind::Class { ref bases, .. } = self.kind {
      for base in bases {
        if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = base.base_type.base {
          if name == class_name {
            return true;
          }
        }
      }
    }
    false
  }
}

impl ParserCppData {
  /// Checks if specified class is a template class.
  #[allow(dead_code)]
  pub fn is_template_class(&self, name: &str) -> bool {
    if let Some(type_info) = self.types.iter().find(|t| &t.name == name) {
      if let CppTypeKind::Class {
               ref template_arguments,
               ref bases,
               ..
             } = type_info.kind {
        if template_arguments.is_some() {
          return true;
        }
        for base in bases {
          if let CppTypeBase::Class(CppTypeClassBase {
                                      ref name,
                                      ref template_arguments,
                                    }) = base.base_type.base {
            if template_arguments.is_some() {
              return true;
            }
            if self.is_template_class(name) {
              return true;
            }
          }
        }
      }
    } else {
      log::llog(log::DebugGeneral,
                || format!("Unknown type assumed to be non-template: {}", name));
    }
    false
  }

  /// Checks if `class_name` types inherits `base_name` type directly or indirectly.
  pub fn inherits(&self, class_name: &str, base_name: &str, dependencies: &[CppData]) -> bool {
    for types in once(&self.types).chain(dependencies.iter().map(|c| &c.parser.types)) {
      if let Some(info) = types.iter().find(|x| &x.name == class_name) {
        if let CppTypeKind::Class { ref bases, .. } = info.kind {
          for base1 in bases {
            if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = base1.base_type.base {
              if name == base_name {
                return true;
              }
              if self.inherits(name, base_name, dependencies) {
                return true;
              }
            }
          }
        }
      }
    }
    false
  }



  /// Parses include files to detect which methods are signals or slots.
  pub fn detect_signals_and_slots(&mut self, dependencies: &[CppData]) -> Result<()> {
    let mut files = HashSet::new();
    for type1 in &self.types {
      if self.inherits(&type1.name, "QObject", dependencies) &&
         !files.contains(&type1.origin_location.include_file_path) {
        files.insert(type1.origin_location.include_file_path.clone());
      }
    }

    #[derive(Debug, Clone)]
    enum SectionType {
      Signals,
      Slots,
      Other,
    }
    #[derive(Debug)]
    struct Section {
      line: usize,
      section_type: SectionType,
    }

    if files.is_empty() {
      return Ok(());
    }
    log::status("Detecting signals and slots");
    let re_signals = Regex::new(r"(signals|Q_SIGNALS)\s*:")?;
    let re_slots = Regex::new(r"(slots|Q_SLOTS)\s*:")?;
    let re_other = Regex::new(r"(public|protected|private)\s*:")?;
    let mut sections = HashMap::new();

    for file_path in files {
      let mut file_sections = Vec::new();
      let file = open_file(&file_path)?;
      let reader = BufReader::new(file.into_file());
      for (line_num, line) in reader.lines().enumerate() {
        let line = line
          .chain_err(|| format!("failed while reading lines from {}", &file_path))?;
        let section_type = if re_signals.is_match(&line) {
          Some(SectionType::Signals)
        } else if re_slots.is_match(&line) {
          Some(SectionType::Slots)
        } else if re_other.is_match(&line) {
          Some(SectionType::Other)
        } else {
          None
        };
        if let Some(section_type) = section_type {
          file_sections.push(Section {
                               line: line_num,
                               section_type: section_type,
                             });
        }
      }
      // println!("sections: {:?}", file_sections);
      if !file_sections.is_empty() {
        sections.insert(file_path, file_sections);
      }
    }
    for type1 in &self.types {
      if let Some(sections) = sections.get(&type1.origin_location.include_file_path) {
        let sections: Vec<_> = sections
          .iter()
          .filter(|x| x.line + 1 >= type1.origin_location.line as usize)
          .collect();
        for method in &mut self.methods {
          let mut section_type = SectionType::Other;
          if let Some(ref info) = method.class_membership {
            if info.class_type.name == type1.name {
              if let Some(ref location) = method.origin_location {
                let matching_sections: Vec<_> = sections
                  .clone()
                  .into_iter()
                  .filter(|x| x.line + 1 <= location.line as usize)
                  .collect();
                if !matching_sections.is_empty() {
                  let section = matching_sections[matching_sections.len() - 1];
                  section_type = section.section_type.clone();
                  if log::is_on(log::DebugSignals) {
                    match section.section_type {
                      SectionType::Signals => {
                        log::log(log::DebugSignals,
                                 format!("Found signal: {}", method.short_text()));
                      }
                      SectionType::Slots => {
                        log::log(log::DebugSignals,
                                 format!("Found slot: {}", method.short_text()));
                      }
                      SectionType::Other => {}
                    }
                  }
                }
              }
            }
          }
          if let Some(ref mut info) = method.class_membership {
            match section_type {
              SectionType::Signals => {
                info.is_signal = true;
              }
              SectionType::Slots => {
                info.is_slot = true;
              }
              SectionType::Other => {}
            }
          }

        }
      }
    }
    Ok(())
  }

  /// Checks if specified class has explicitly declared protected or private destructor.
  pub fn has_non_public_destructor(&self, class_type: &CppTypeClassBase) -> bool {
    for method in &self.methods {
      if let Some(ref info) = method.class_membership {
        if info.kind == CppMethodKind::Destructor && &info.class_type == class_type {
          return info.visibility != CppVisibility::Public;
        }
      }
    }
    false
  }



  /// Performs data conversion to make it more suitable
  /// for further wrapper generation.
  pub fn post_process(self,
                      dependencies: Vec<CppData>,
                      allocation_place_overrides: &HashMap<String, CppTypeAllocationPlace>)
                      -> Result<CppDataWithDeps> {
    let mut r = CppDataWithDeps {
      current: CppData {
        parser: self,
        processed: ProcessedCppData {
          extra_methods: Vec::new(),
          template_instantiations: Vec::new(),
          inherited_methods: Vec::new(),
          signal_argument_types: Vec::new(),
          type_allocation_places: HashMap::new(),
        },
      },
      dependencies: dependencies,
    };
    r.current.processed.template_instantiations = r.find_template_instantiations();

    r.current.processed.type_allocation_places =
      r.choose_allocation_places(allocation_place_overrides)?;
    r.current.processed.signal_argument_types = r.detect_signal_argument_types()?;
    {
      let mut methods = r.generate_methods_with_omitted_args();
      r.current.processed.extra_methods.append(&mut methods);
    }
    {
      let mut methods = r.instantiate_templates()?;
      r.current.processed.extra_methods.append(&mut methods);
    }
    r.current.processed.inherited_methods = r.detect_inherited_methods2()?;
    {
      let mut methods = r.ensure_explicit_destructors()?;
      r.current.processed.extra_methods.append(&mut methods);
    }
    {
      // TODO: fix doc generator for field accessors
      let mut methods = r.add_field_accessors()?;
      r.current.processed.extra_methods.append(&mut methods);
    }
    {
      // TODO: fix doc generator for field accessors
      let mut methods = r.add_casts()?;
      r.current.processed.extra_methods.append(&mut methods);
    }
    Ok(r)
  }
}

impl TemplateArgumentsDeclaration {
  /// Returns count of the template arguments.
  #[allow(dead_code)]
  pub fn count(&self) -> usize {
    self.names.len()
  }
}

impl CppDataWithDeps {
  /// Returns true if `type1` is a known template instantiation.
  fn check_template_type(&self, type1: &CppType) -> Result<()> {
    if let CppTypeBase::Class(CppTypeClassBase {
                                ref name,
                                ref template_arguments,
                              }) = type1.base {
      if let Some(ref template_arguments) = *template_arguments {
        let is_valid = |cpp_data: &CppData| {
          cpp_data
            .processed
            .template_instantiations
            .iter()
            .any(|inst| {
                   &inst.class_name == name &&
                   inst
                     .instantiations
                     .iter()
                     .any(|x| &x.template_arguments == template_arguments)
                 })
        };
        if !once(&self.current)
              .chain(self.dependencies.iter())
              .any(is_valid) {
          return Err(format!("type not available: {:?}", type1).into());
        }
        for arg in template_arguments {
          self.check_template_type(arg)?;
        }
      }
    }
    Ok(())
  }

  /// Searches for template instantiations in this library's API,
  /// excluding results that were already processed in dependencies.
  #[cfg_attr(feature="clippy", allow(block_in_if_condition_stmt))]
  fn find_template_instantiations(&self) -> Vec<CppTemplateInstantiations> {

    fn check_type(type1: &CppType, deps: &[CppData], result: &mut Vec<CppTemplateInstantiations>) {
      if let CppTypeBase::Class(CppTypeClassBase {
                                  ref name,
                                  ref template_arguments,
                                }) = type1.base {
        if let Some(ref template_arguments) = *template_arguments {
          if !template_arguments
                .iter()
                .any(|x| x.base.is_or_contains_template_parameter()) {
            if !deps
                  .iter()
                  .any(|data| {
              data
                .processed
                .template_instantiations
                .iter()
                .any(|i| {
                       &i.class_name == name &&
                       i.instantiations
                         .iter()
                         .any(|x| &x.template_arguments == template_arguments)
                     })
            }) {
              if !result.iter().any(|x| &x.class_name == name) {
                log::llog(log::DebugParser, || {
                  format!("Found template instantiation: {}<{:?}>",
                          name,
                          template_arguments)
                });
                result.push(CppTemplateInstantiations {
                              class_name: name.clone(),
                              instantiations: vec![CppTemplateInstantiation {
                                                     template_arguments: template_arguments.clone(),
                                                   }],
                            });
              } else {
                let item = result
                  .iter_mut()
                  .find(|x| &x.class_name == name)
                  .expect("previously found");
                if !item
                      .instantiations
                      .iter()
                      .any(|x| &x.template_arguments == template_arguments) {
                  log::llog(log::DebugParser, || {
                    format!("Found template instantiation: {}<{:?}>",
                            name,
                            template_arguments)
                  });
                  item
                    .instantiations
                    .push(CppTemplateInstantiation {
                            template_arguments: template_arguments.clone(),
                          });
                }
              }
            }
          }
          for arg in template_arguments {
            check_type(arg, deps, result);
          }
        }
      }
    }
    let mut result = Vec::new();
    for m in &self.current.parser.methods {
      check_type(&m.return_type, &self.dependencies, &mut result);
      for arg in &m.arguments {
        check_type(&arg.argument_type, &self.dependencies, &mut result);
      }
    }
    for t in &self.current.parser.types {
      if let CppTypeKind::Class {
               ref bases,
               ref fields,
               ..
             } = t.kind {
        for base in bases {
          check_type(&base.base_type, &self.dependencies, &mut result);
        }
        for field in fields {
          check_type(&field.field_type, &self.dependencies, &mut result);
        }
      }
    }
    result
  }


  /// Adds methods produced as template instantiations of
  /// methods of existing template classes and existing template methods.
  fn instantiate_templates(&self) -> Result<Vec<CppMethod>> {
    log::status("Instantiating templates");
    let mut new_methods = Vec::new();

    for cpp_data in self.dependencies.iter().chain(once(&self.current)) {
      for method in cpp_data.all_methods() {
        for type1 in method.all_involved_types() {
          if let CppTypeBase::Class(CppTypeClassBase {
                                      ref name,
                                      ref template_arguments,
                                    }) = type1.base {
            if let Some(ref template_arguments) = *template_arguments {
              assert!(!template_arguments.is_empty());
              if template_arguments
                   .iter()
                   .all(|x| x.base.is_template_parameter()) {
                if let Some(template_instantiations) =
                  self
                    .current
                    .processed
                    .template_instantiations
                    .iter()
                    .find(|x| &x.class_name == name) {
                  let nested_level = if let CppTypeBase::TemplateParameter {
                           nested_level, ..
                         } = template_arguments[0].base {
                    nested_level
                  } else {
                    return Err("only template parameters can be here".into());
                  };
                  log::llog(log::DebugTemplateInstantiation, || "");
                  log::llog(log::DebugTemplateInstantiation,
                            || format!("method: {}", method.short_text()));
                  log::llog(log::DebugTemplateInstantiation, || {
                    format!("found template instantiations: {:?}",
                            template_instantiations)
                  });
                  match apply_instantiations_to_method(method,
                                                       nested_level,
                                                       &template_instantiations.instantiations) {
                    Ok(methods) => {
                      for method in methods {
                        let mut ok = true;
                        for type1 in method.all_involved_types() {
                          match self.check_template_type(&type1) {
                            Ok(_) => {}
                            Err(msg) => {
                              ok = false;
                              log::llog(log::DebugTemplateInstantiation, || {
                                format!("method is not accepted: {}", method.short_text())
                              });
                              log::llog(log::DebugTemplateInstantiation, || format!("  {}", msg));
                            }
                          }
                        }
                        if ok {
                          new_methods.push(method);
                        }
                      }
                      break;
                    }
                    Err(msg) => {
                      log::llog(log::DebugTemplateInstantiation,
                                || format!("failed: {}", msg))
                    }
                  }
                  break;
                }
              }
            }
          }
        }
      }
    }
    Ok(new_methods)
  }





  /// Returns selected type allocation place for type `class_name`.
  pub fn type_allocation_place(&self, class_name: &str) -> Result<CppTypeAllocationPlace> {
    if let Some(r) = self
         .current
         .processed
         .type_allocation_places
         .get(class_name) {
      return Ok(r.clone());
    }
    for dep in &self.dependencies {
      if let Some(r) = dep.processed.type_allocation_places.get(class_name) {
        return Ok(r.clone());
      }
    }
    Err(format!("no type allocation place information for {}", class_name).into())
  }

  /// Search for a `CppTypeData` object in this `CppData` and all dependencies.
  pub fn find_type_info<F>(&self, f: F) -> Option<&CppTypeData>
    where F: Fn(&&CppTypeData) -> bool
  {
    once(&self.current.parser.types)
      .chain(self.dependencies.iter().map(|d| &d.parser.types))
      .flat_map(|x| x)
      .find(f)
  }






  /// Adds destructors for every class that does not have explicitly
  /// defined destructor, allowing to create wrappings for
  /// destructors implicitly available in C++.
  fn ensure_explicit_destructors(&self) -> Result<Vec<CppMethod>> {
    let mut methods = Vec::new();
    for type1 in &self.current.parser.types {
      if let CppTypeKind::Class { .. } = type1.kind {
        let class_name = &type1.name;
        let found_destructor = self
          .current
          .parser
          .methods
          .iter()
          .any(|m| m.is_destructor() && m.class_name() == Some(class_name));
        if !found_destructor {
          let is_virtual = self.has_virtual_destructor(class_name);
          methods.push(CppMethod {
                         name: format!("~{}", class_name),
                         class_membership: Some(CppMethodClassMembership {
                                                  class_type: type1.default_class_type()?,
                                                  is_virtual: is_virtual,
                                                  is_pure_virtual: false,
                                                  is_const: false,
                                                  is_static: false,
                                                  visibility: CppVisibility::Public,
                                                  is_signal: false,
                                                  is_slot: false,
                                                  kind: CppMethodKind::Destructor,
                                                  fake: None,
                                                }),
                         operator: None,
                         return_type: CppType::void(),
                         arguments: vec![],
                         arguments_before_omitting: None,
                         allows_variadic_arguments: false,
                         include_file: type1.include_file.clone(),
                         origin_location: None,
                         template_arguments: None,
                         template_arguments_values: None,
                         declaration_code: None,
                         doc: None,
                         inheritance_chain: Vec::new(),
                         //is_fake_inherited_method: false,
                         is_ffi_whitelisted: false,
                         is_unsafe_static_cast: false,
                         is_direct_static_cast: false,
                       });
        }
      }
    }
    Ok(methods)
  }

  /*
  /// Helper function that performs a portion of add_inherited_methods implementation.
  fn inherited_methods_from(&self,
                            base_name: &str,
                            all_base_methods: &[&CppMethod])
                            -> Result<Vec<CppMethod>> {
    // TODO: speed up this method (#12)
    let mut new_methods = Vec::new();
    {
      for type1 in &self.current.parser.types {
        if let CppTypeKind::Class {
                 ref bases,
                 ref using_directives,
                 ..
               } = type1.kind {
          for base in bases {
            if let CppTypeBase::Class(CppTypeClassBase {
                                        ref name,
                                        ref template_arguments,
                                      }) = base.base_type.base {
              if name == base_name {
                log::llog(log::DebugInheritance, || {
                  format!("Adding inherited methods from {} to {}",
                          base_name,
                          type1.name)
                });
                let derived_name = &type1.name;
                let base_template_arguments = template_arguments;
                let base_methods = all_base_methods
                  .into_iter()
                  .filter(|method| if let Some(ref info) = method.class_membership {
                            &info.class_type.template_arguments == base_template_arguments
                          } else {
                            false
                          });
                let mut current_new_methods = Vec::new();
                for base_class_method in base_methods {
                  let mut using_directive_enables = false;
                  let mut using_directive_disables = false;
                  for dir in using_directives {
                    if &dir.method_name == &base_class_method.name {
                      if &dir.class_name == base_name {
                        log::llog(log::DebugInheritance, || {
                          format!("UsingDirective enables inheritance of {}",
                                  base_class_method.short_text())
                        });
                        using_directive_enables = true;
                      } else {
                        log::llog(log::DebugInheritance, || {
                          format!("UsingDirective disables inheritance of {}",
                                  base_class_method.short_text())
                        });
                        using_directive_disables = true;
                      }
                    }
                  }
                  if using_directive_disables {
                    continue;
                  }

                  let mut ok = true;
                  for method in self.current.all_methods() {
                    if method.class_name() == Some(derived_name) &&
                       method.name == base_class_method.name {
                      // without using directive, any method with the same name
                      // disables inheritance of base class method;
                      // with using directive, only method with the same arguments
                      // disables inheritance of base class method.
                      if !using_directive_enables ||
                         method.argument_types_equal(base_class_method) {
                        log::llog(log::DebugInheritance,
                                  || "Method is not added because it's overriden in derived class");
                        log::llog(log::DebugInheritance,
                                  || format!("Base method: {}", base_class_method.short_text()));
                        log::llog(log::DebugInheritance,
                                  || format!("Derived method: {}\n", method.short_text()));
                        ok = false;
                      }
                      break;
                    }
                  }
                  if ok {
                    let mut new_method: CppMethod = (*base_class_method).clone();
                    if let Some(ref mut info) = new_method.class_membership {
                      info.class_type = type1.default_class_type()?;
                    } else {
                      return Err(unexpected("no class membership").into());
                    }
                    new_method.include_file = type1.include_file.clone();
                    new_method.origin_location = None;
                    new_method.declaration_code = None;
                    new_method.inheritance_chain.push(base.clone());
                    new_method.is_fake_inherited_method = true;
                    log::llog(log::DebugInheritance,
                              || format!("Method added: {}", new_method.short_text()));
                    log::llog(log::DebugInheritance, || {
                      format!("Base method: {} ({:?})\n",
                              base_class_method.short_text(),
                              base_class_method.origin_location)
                    });
                    current_new_methods.push(new_method.clone());
                  }
                }
                new_methods.append(&mut self
                                          .inherited_methods_from(derived_name,
                                                                  &current_new_methods
                                                                     .iter()
                                                                     .collect::<Vec<_>>())?);
                new_methods.append(&mut current_new_methods);
              }
            }
          }
        }
      }
    }
    Ok(new_methods)
  }

  /// Adds methods of derived classes inherited from base classes.
  /// A method will not be added if there is a method with the same
  /// name in the derived class. Constructors, destructors and assignment
  /// operators are also not added. This reflects C++'s method inheritance rules.
  #[cfg_attr(feature="clippy", allow(block_in_if_condition_stmt))]
  pub fn add_inherited_methods(&mut self) -> Result<()> {
    log::status("Adding inherited methods");
    let mut all_new_methods = Vec::new();
    for (is_self, cpp_data) in
      self
        .dependencies
        .iter()
        .map(|x| (false, x))
        .chain(once((true, self as &_))) {
      for type1 in &cpp_data.types {
        if type1.is_class() {
          let mut interesting_cpp_datas: Vec<&CppData> = vec![cpp_data];
          if !is_self {
            interesting_cpp_datas.push(self);
          }
          for cpp_data2 in interesting_cpp_datas {
            let base_methods = cpp_data2
              .methods
              .iter()
              .filter(|method| if let Some(ref info) = method.class_membership {
                        &info.class_type.name == &type1.name && !info.kind.is_constructor() &&
                        !info.kind.is_destructor() &&
                        method.operator != Some(CppOperator::Assignment)
                      } else {
                        false
                      });
            all_new_methods.append(&mut self
                                          .inherited_methods_from(&type1.name,
                                                                  &base_methods
                                                                     .collect::<Vec<_>>())?);
          }
        }
      }
    }
    while let Some(method) = all_new_methods.pop() {
      let mut duplicates = Vec::new();
      while let Some(index) =
        all_new_methods
          .iter()
          .position(|m| m.class_name() == method.class_name() && m.name == method.name) {
        duplicates.push(all_new_methods.remove(index));
      }
      if duplicates.is_empty() {
        self.methods.push(method);
      } else {
        duplicates.push(method);

        let mut allow_method = false;

        let mut lowest_visibility = CppVisibility::Public;
        for duplicate in &duplicates {
          if let Some(ref info) = duplicate.class_membership {
            if info.visibility == CppVisibility::Private {
              lowest_visibility = CppVisibility::Private;
            } else if info.visibility == CppVisibility::Protected &&
                      lowest_visibility != CppVisibility::Private {
              lowest_visibility = CppVisibility::Protected;
            }
          } else {
            return Err("only class methods can appear here".into());
          }
        }
        if duplicates
             .iter()
             .find(|m| m.inheritance_chain.last() != duplicates[0].inheritance_chain.last())
             .is_none() {
          // all methods are inherited from one base class
          self.methods.append(&mut duplicates);
        } else {
          let signature_mismatch = duplicates
            .iter()
            .any(|m| {
              let f = &duplicates[0];
              let info_mismatch = if let Some(ref m_info) = m.class_membership {
                if let Some(ref f_info) = f.class_membership {
                  m_info.is_const != f_info.is_const || m_info.is_static != f_info.is_static
                } else {
                  true
                }
              } else {
                true
              };
              info_mismatch || &m.return_type != &f.return_type || !m.argument_types_equal(f) ||
              m.allows_variadic_arguments == f.allows_variadic_arguments
            });
          if !signature_mismatch && !duplicates.iter().any(|x| x.inheritance_chain.is_empty()) {
            // TODO: support more complicated cases (#23)
            let first_base = &duplicates[0].inheritance_chain[0].base_type;
            if duplicates
                 .iter()
                 .all(|x| {
                        x.inheritance_chain[0].is_virtual &&
                        &x.inheritance_chain[0].base_type == first_base
                      }) {
              allow_method = true;
            }
          }
          if allow_method {
            log::llog(log::DebugInheritance,
                      || "Allowing duplicated inherited method (virtual diamond inheritance)");
            log::llog(log::DebugInheritance, || duplicates[0].short_text());
            for duplicate in &duplicates {
              log::llog(log::DebugInheritance,
                        || format!("  {}", duplicate.inheritance_chain_text()));
            }
            if let Some(mut final_method) = duplicates.pop() {
              if let Some(ref mut info) = final_method.class_membership {
                info.visibility = lowest_visibility;
              } else {
                return Err("only class methods can appear here".into());
              }
              self.methods.push(final_method);
            } else {
              return Err(unexpected("duplicates can't be empty").into());
            }
          } else {
            log::llog(log::DebugInheritance,
                      || "Removed ambiguous inherited methods (presumed inaccessible):");
            if signature_mismatch {
              for duplicate in &duplicates {
                log::llog(log::DebugInheritance,
                          || format!("  {}", duplicate.short_text()));
                log::llog(log::DebugInheritance,
                          || format!("  {}", duplicate.inheritance_chain_text()));
              }
            } else {
              log::llog(log::DebugInheritance, || duplicates[0].short_text());
              for duplicate in &duplicates {
                log::llog(log::DebugInheritance,
                          || format!("  {}", duplicate.inheritance_chain_text()));
              }
            }
          }
        }
      }
    }
    Ok(())
  } */

  /// Generates duplicate methods with fewer arguments for
  /// C++ methods with default argument values.
  fn generate_methods_with_omitted_args(&self) -> Vec<CppMethod> {
    let mut new_methods = Vec::new();
    for method in self.current.all_methods() {
      if let Some(last_arg) = method.arguments.last() {
        if last_arg.has_default_value {
          let mut method_copy = method.clone();
          method_copy.arguments_before_omitting = Some(method.arguments.clone());
          while let Some(arg) = method_copy.arguments.pop() {
            if !arg.has_default_value {
              break;
            }
            new_methods.push(method_copy.clone());
          }
        }
      }
    }
    new_methods
  }

  /// Detects the preferred type allocation place for each type based on
  /// API of all known methods. Keys of `overrides` are C++ type names.
  /// If `overrides` contains type allocation place for a type, it's used instead of
  /// the place that would be automatically selected.
  pub fn choose_allocation_places(&self,
                                  overrides: &HashMap<String, CppTypeAllocationPlace>)
                                  -> Result<HashMap<String, CppTypeAllocationPlace>> {
    log::status("Detecting type allocation places");

    #[derive(Default)]
    struct TypeStats {
      // has_derived_classes: bool,
      has_virtual_methods: bool,
      pointers_count: usize,
      not_pointers_count: usize,
    };
    fn check_type(cpp_type: &CppType, data: &mut HashMap<String, TypeStats>) {
      if let CppTypeBase::Class(CppTypeClassBase {
                                  ref name,
                                  ref template_arguments,
                                }) = cpp_type.base {
        if !data.contains_key(name) {
          data.insert(name.clone(), TypeStats::default());
        }
        match cpp_type.indirection {
          CppTypeIndirection::None | CppTypeIndirection::Ref => {
            data.get_mut(name).unwrap().not_pointers_count += 1
          }
          CppTypeIndirection::Ptr => data.get_mut(name).unwrap().pointers_count += 1,
          _ => {}
        }
        if let Some(ref args) = *template_arguments {
          for arg in args {
            check_type(arg, data);
          }
        }
      }
    }

    let mut data = HashMap::new();
    for type1 in &self.current.parser.types {
      if self.has_virtual_methods(&type1.name) {
        if !data.contains_key(&type1.name) {
          data.insert(type1.name.clone(), TypeStats::default());
        }
        data.get_mut(&type1.name).unwrap().has_virtual_methods = true;
      }
    }
    for method in self.current.all_methods() {
      check_type(&method.return_type, &mut data);
      for arg in &method.arguments {
        check_type(&arg.argument_type, &mut data);
      }
    }
    let mut results = HashMap::new();
    {
      let mut logger = log::default_logger();
      if logger.is_on(log::DebugAllocationPlace) {
        for (name, stats) in &data {
          logger.log(log::DebugAllocationPlace,
                     format!("{}\t{}\t{}\t{}",
                             name,
                             stats.has_virtual_methods,
                             stats.pointers_count,
                             stats.not_pointers_count));
        }
      }
    }

    for type1 in &self.current.parser.types {
      if !type1.is_class() {
        continue;
      }
      let name = &type1.name;
      let result = if overrides.contains_key(name) {
        overrides[name].clone()
      } else if let Some(ref stats) = data.get(name) {
        if stats.has_virtual_methods {
          CppTypeAllocationPlace::Heap
        } else if stats.pointers_count == 0 {
          CppTypeAllocationPlace::Stack
        } else {
          let min_safe_data_count = 5;
          let min_not_pointers_percent = 0.3;
          if stats.pointers_count + stats.not_pointers_count < min_safe_data_count {
            log::llog(log::DebugAllocationPlace,
                      || format!("Can't determine type allocation place for '{}':", name));
            log::llog(log::DebugAllocationPlace, || {
              format!("  Not enough data (pointers={}, not pointers={})",
                      stats.pointers_count,
                      stats.not_pointers_count)
            });
          } else if stats.not_pointers_count as f32 /
                    (stats.pointers_count + stats.not_pointers_count) as f32 >
                    min_not_pointers_percent {
            log::llog(log::DebugAllocationPlace,
                      || format!("Can't determine type allocation place for '{}':", name));
            log::llog(log::DebugAllocationPlace, || {
              format!("  Many not pointers (pointers={}, not pointers={})",
                      stats.pointers_count,
                      stats.not_pointers_count)
            });
          }
          CppTypeAllocationPlace::Heap
        }
      } else {
        log::llog(log::DebugAllocationPlace, || {
          format!("Can't determine type allocation place for '{}' (no data)",
                  name)
        });
        CppTypeAllocationPlace::Heap
      };
      results.insert(name.clone(), result);
    }
    log::llog(log::DebugAllocationPlace, || {
      format!("Allocation place is heap for: {}",
              results
                .iter()
                .filter(|&(_, v)| v == &CppTypeAllocationPlace::Heap)
                .map(|(k, _)| k)
                .join(", "))
    });
    log::llog(log::DebugAllocationPlace, || {
      format!("Allocation place is stack for: {}",
              results
                .iter()
                .filter(|&(_, v)| v == &CppTypeAllocationPlace::Stack)
                .map(|(k, _)| k)
                .join(", "))
    });

    Ok(results)
  }

  /// Adds fictional getter and setter methods for each known public field of each class.
  fn add_field_accessors(&self) -> Result<Vec<CppMethod>> {
    log::status("Adding field accessors");
    let mut new_methods = Vec::new();
    for type_info in &self.current.parser.types {
      if let CppTypeKind::Class { ref fields, .. } = type_info.kind {
        for field in fields {
          let create_method = |name, accessor_type, return_type, arguments| -> Result<CppMethod> {
            Ok(CppMethod {
                 name: name,
                 class_membership: Some(CppMethodClassMembership {
                                          class_type: type_info.default_class_type()?,
                                          kind: CppMethodKind::Regular,
                                          is_virtual: false,
                                          is_pure_virtual: false,
                                          is_const: match accessor_type {
                                            CppFieldAccessorType::CopyGetter |
                                            CppFieldAccessorType::ConstRefGetter => true,
                                            CppFieldAccessorType::MutRefGetter |
                                            CppFieldAccessorType::Setter => false,
                                          },
                                          is_static: false,
                                          visibility: CppVisibility::Public,
                                          is_signal: false,
                                          is_slot: false,
                                          fake: Some(FakeCppMethod::FieldAccessor {
                                                       accessor_type: accessor_type,
                                                       field_name: field.name.clone(),
                                                     }),
                                        }),
                 operator: None,
                 return_type: return_type,
                 arguments: arguments,
                 arguments_before_omitting: None,
                 allows_variadic_arguments: false,
                 include_file: type_info.include_file.clone(),
                 origin_location: None,
                 template_arguments: None,
                 template_arguments_values: None,
                 declaration_code: None,
                 doc: None,
                 inheritance_chain: Vec::new(),
                 //is_fake_inherited_method: false,
                 is_ffi_whitelisted: false,
                 is_unsafe_static_cast: false,
                 is_direct_static_cast: false,
               })
          };
          if field.visibility == CppVisibility::Public {
            if field.field_type.indirection == CppTypeIndirection::None &&
               field.field_type.base.is_class() {

              let mut type2_const = field.field_type.clone();
              type2_const.is_const = true;
              type2_const.indirection = CppTypeIndirection::Ref;
              let mut type2_mut = field.field_type.clone();
              type2_mut.is_const = false;
              type2_mut.indirection = CppTypeIndirection::Ref;
              new_methods.push(create_method(field.name.clone(),
                                             CppFieldAccessorType::ConstRefGetter,
                                             type2_const,
                                             Vec::new())?);
              new_methods.push(create_method(format!("{}_mut", field.name),
                                             CppFieldAccessorType::MutRefGetter,
                                             type2_mut,
                                             Vec::new())?);
            } else {
              new_methods.push(create_method(field.name.clone(),
                                             CppFieldAccessorType::CopyGetter,
                                             field.field_type.clone(),
                                             Vec::new())?);
            }
            let arg = CppFunctionArgument {
              argument_type: field.field_type.clone(),
              name: "value".to_string(),
              has_default_value: false,
            };
            new_methods.push(create_method(format!("set_{}", field.name),
                                           CppFieldAccessorType::Setter,
                                           CppType::void(),
                                           vec![arg])?);
          }
        }
      }
    }
    Ok(new_methods)
  }

  /// Performs a portion of `add_casts` operation.
  /// Adds casts between `target_type` and `base_type` and calls
  /// `add_casts_one` recursively to add casts between `target_type`
  /// and base types of `base_type`.
  fn add_casts_one(&self,
                   target_type: &CppTypeClassBase,
                   base_type: &CppType,
                   is_direct: bool)
                   -> Result<Vec<CppMethod>> {
    let type_info = self
      .find_type_info(|x| x.name == target_type.name)
      .chain_err(|| "type info not found")?;
    let target_ptr_type = CppType {
      base: CppTypeBase::Class(target_type.clone()),
      indirection: CppTypeIndirection::Ptr,
      is_const: false,
      is_const2: false,
    };
    let base_ptr_type = CppType {
      base: base_type.base.clone(),
      indirection: CppTypeIndirection::Ptr,
      is_const: false,
      is_const2: false,
    };
    let mut new_methods = Vec::new();
    new_methods.push(create_cast_method("static_cast",
                                        &base_ptr_type,
                                        &target_ptr_type,
                                        true,
                                        is_direct,
                                        &type_info.include_file));
    new_methods.push(create_cast_method("static_cast",
                                        &target_ptr_type,
                                        &base_ptr_type,
                                        false,
                                        is_direct,
                                        &type_info.include_file));
    if let CppTypeBase::Class(ref base) = base_type.base {
      if self.has_virtual_methods(&base.name) {
        new_methods.push(create_cast_method("dynamic_cast",
                                            &base_ptr_type,
                                            &target_ptr_type,
                                            false,
                                            false,
                                            &type_info.include_file));
      }
    }

    if let CppTypeBase::Class(ref base) = base_type.base {
      if let Some(type_info) = self.find_type_info(|x| x.name == base.name) {
        if let CppTypeKind::Class { ref bases, .. } = type_info.kind {
          for base in bases {
            new_methods.append(&mut self.add_casts_one(target_type, &base.base_type, false)?);
          }
        }
      }
    }
    Ok(new_methods)
  }

  /// Adds `static_cast` and `dynamic_cast` functions for all appropriate pairs of types
  /// in this `CppData`.
  fn add_casts(&self) -> Result<Vec<CppMethod>> {
    log::status("Adding cast functions");
    let mut new_methods = Vec::new();
    for type_info in &self.current.parser.types {
      if let CppTypeKind::Class { ref bases, .. } = type_info.kind {
        let t = type_info.default_class_type()?;
        let single_base = bases.len() == 1;
        for base in bases {
          new_methods.append(&mut self.add_casts_one(&t, &base.base_type, single_base)?);
        }
      }
    }
    Ok(new_methods)
  }

  /// Checks if `class_name` types inherits `base_name` type directly or indirectly.
  pub fn inherits(&self, class_name: &str, base_name: &str) -> bool {
    for types in self.all_types() {
      if let Some(info) = types.iter().find(|x| &x.name == class_name) {
        if let CppTypeKind::Class { ref bases, .. } = info.kind {
          for base1 in bases {
            if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = base1.base_type.base {
              if name == base_name {
                return true;
              }
              if self.inherits(name, base_name) {
                return true;
              }
            }
          }
        }
      }
    }
    false
  }

  fn detect_signal_argument_types(&self) -> Result<Vec<Vec<CppType>>> {
    let mut all_types = HashSet::new();
    for method in &self.current.parser.methods {
      if let Some(ref method_info) = method.class_membership {
        if method_info.is_signal {
          let types: Vec<_> = method
            .arguments
            .iter()
            .map(|x| x.argument_type.clone())
            .collect();
          if !all_types.contains(&types) &&
             !self
                .dependencies
                .iter()
                .any(|d| {
                       d.processed
                         .signal_argument_types
                         .iter()
                         .any(|t| t == &types)
                     }) {
            all_types.insert(types);
          }
        }
      }
    }

    let mut types_with_omitted_args = HashSet::new();
    for t in &all_types {
      let mut types = t.clone();
      while let Some(_) = types.pop() {
        if !types_with_omitted_args.contains(&types) && !all_types.contains(&types) &&
           !self
              .dependencies
              .iter()
              .any(|d| {
                     d.processed
                       .signal_argument_types
                       .iter()
                       .any(|t| t == &types)
                   }) {
          types_with_omitted_args.insert(types.clone());
        }
      }
    }
    all_types.extend(types_with_omitted_args.into_iter());

    log::llog(log::DebugSignals, || "Signal argument types:");
    for t in &all_types {
      log::llog(log::DebugSignals, || {
        format!("  ({})",
                t.iter().map(|x| x.to_cpp_pseudo_code()).join(", "))
      });
    }
    Ok(all_types.into_iter().collect())
  }





  /// Checks if specified class has virtual destructor (own or inherited).
  pub fn has_virtual_destructor(&self, class_name: &str) -> bool {
    for method in self
          .current
          .parser
          .methods
          .iter()
          .chain(self.current.processed.inherited_methods.iter()) {
      if let Some(ref info) = method.class_membership {
        if info.kind == CppMethodKind::Destructor && &info.class_type.name == class_name {
          return info.is_virtual;
        }
      }
    }
    false
  }

  /// Checks if specified class has any virtual methods (own or inherited).
  pub fn has_virtual_methods(&self, class_name: &str) -> bool {
    for method in self
          .current
          .parser
          .methods
          .iter()
          .chain(self.current.processed.inherited_methods.iter()) {
      if let Some(ref info) = method.class_membership {
        if &info.class_type.name == class_name && info.is_virtual {
          return true;
        }
      }
    }
    false
  }

  /// Checks if specified class has any virtual methods (own or inherited).
  pub fn has_pure_virtual_methods(&self, class_name: &str) -> bool {
    for method in self
          .current
          .parser
          .methods
          .iter()
          .chain(self.current.processed.inherited_methods.iter()) {
      if let Some(ref info) = method.class_membership {
        if &info.class_type.name == class_name && info.is_pure_virtual {
          return true;
        }
      }
    }
    false
  }


  //
  //  /// Returns true if C++ type `name` is polymorphic, i.e. has
  ///// at least one virtual function.
  //  pub fn is_polymorphic_type(&self, name: &str) -> bool {
  //    self
  //        .all_methods()
  //        .any(|m| if let Some(ref info) = m.class_membership {
  //          info.is_virtual && &info.class_type.name == name
  //        } else {
  //          false
  //        })
  //  }


  pub fn all_types(&self) -> Vec<&Vec<CppTypeData>> {
    once(&self.current.parser.types)
      .chain(self.dependencies.iter().map(|x| &x.parser.types))
      .collect()
  }

  /// Returns all include files found within this `CppData`
  /// (excluding dependencies).
  pub fn all_include_files(&self) -> Result<HashSet<String>> {
    let mut result = HashSet::new();
    for method in self.current.all_methods() {
      if !result.contains(&method.include_file) {
        result.insert(method.include_file.clone());
      }
    }
    for tp in &self.current.parser.types {
      if !result.contains(&tp.include_file) {
        result.insert(tp.include_file.clone());
      }
    }
    for instantiations in &self.current.processed.template_instantiations {
      let type_info =
        self
          .find_type_info(|x| &x.name == &instantiations.class_name)
          .chain_err(|| format!("type info not found for {}", &instantiations.class_name))?;
      if !result.contains(&type_info.include_file) {
        result.insert(type_info.include_file.clone());
      }
    }
    Ok(result)
  }

  fn detect_inherited_methods2(&self) -> Result<Vec<CppMethod>> {
    let mut remaining_classes: Vec<&CppTypeData> = self
      .current
      .parser
      .types
      .iter()
      .filter(|t| if let CppTypeKind::Class { ref bases, .. } = t.kind {
                !bases.is_empty()
              } else {
                false
              })
      .collect();
    let mut ordered_classes = Vec::new();
    while !remaining_classes.is_empty() {
      let mut any_added = false;
      let mut remaining_classes2 = Vec::new();
      for class in &remaining_classes {
        if let CppTypeKind::Class { ref bases, .. } = class.kind {
          if bases
               .iter()
               .any(|base| if base.visibility != CppVisibility::Private &&
                              base.base_type.indirection == CppTypeIndirection::None {
                      if let CppTypeBase::Class(ref base_info) = base.base_type.base {
                        remaining_classes
                          .iter()
                          .any(|c| c.name == base_info.name)
                      } else {
                        false
                      }
                    } else {
                      false
                    }) {
            remaining_classes2.push(*class);
          } else {
            ordered_classes.push(*class);
            any_added = true;
          }
        } else {
          unreachable!()
        }
      }
      remaining_classes = remaining_classes2;
      if !any_added {
        return Err("Cyclic dependency detected while detecting inherited methods".into());
      }
    }

    let mut result = Vec::new();
    for class in ordered_classes {
      log::llog(log::DebugInheritance,
                || format!("Detecting inherited methods for {}\n", class.name));
      let own_methods: Vec<&CppMethod> = self
        .current
        .parser
        .methods
        .iter()
        .filter(|m| m.class_name() == Some(&class.name))
        .collect();
      let bases = if let CppTypeKind::Class { ref bases, .. } = class.kind {
        bases
      } else {
        unreachable!()
      };
      let bases_with_methods: Vec<(&CppBaseSpecifier, Vec<&CppMethod>)> = bases
        .iter()
        .filter(|base| {
                  base.visibility != CppVisibility::Private &&
                  base.base_type.indirection == CppTypeIndirection::None
                })
        .map(|base| {
          let methods = if let CppTypeBase::Class(ref base_class_base) = base.base_type.base {

            once(&self.current.parser)
              .chain(self.dependencies.iter().map(|d| &d.parser))
              .map(|p| &p.methods)
              .flat_map(|m| m)
              .filter(|m| if let Some(ref info) = m.class_membership {
                        &info.class_type == base_class_base
                      } else {
                        false
                      })
              .collect()
          } else {
            Vec::new()
          };
          (base, methods)
        })
        .filter(|x| !x.1.is_empty())
        .collect();

      for &(ref base, ref methods) in &bases_with_methods {
        if let CppTypeBase::Class(ref base_class_base) = base.base_type.base {
          for method in methods {
            if let CppTypeKind::Class { ref using_directives, .. } = class.kind {
              let use_method = if using_directives
                   .iter()
                   .any(|dir| {
                          dir.class_name == base_class_base.name && dir.method_name == method.name
                        }) {
                true // excplicitly inherited with a using directive
              } else if own_methods.iter().any(|m| m.name == method.name) {
                // not inherited because method with the same name exists in the derived class
                false
              } else if bases_with_methods
                          .iter()
                          .any(|&(ref base2, ref methods2)| {
                                 base != base2 && methods2.iter().any(|m| m.name == method.name)
                               }) {
                // not inherited because method with the same name exists in one of
                // the other bases
                false
              } else {
                // no aliased method found and no using directives
                true
              };
              // TODO: detect diamond inheritance
              if use_method {
                let mut new_method = (*method).clone();
                if let Some(ref mut info) = new_method.class_membership {
                  info.class_type = class.default_class_type()?;
                } else {
                  return Err(unexpected("no class membership").into());
                }
                new_method.include_file = class.include_file.clone();
                new_method.origin_location = None;
                new_method.declaration_code = None;
                new_method.inheritance_chain.push((*base).clone());
                //new_method.is_fake_inherited_method = true;
                log::llog(log::DebugInheritance,
                          || format!("Method added: {}", new_method.short_text()));
                log::llog(log::DebugInheritance, || {
                  format!("Base method: {} ({:?})\n",
                          method.short_text(),
                          method.origin_location)
                });
                result.push(new_method);
              }

            } else {
              unreachable!()
            }

          }
        } else {
          unreachable!()
        }

      }

    }
    Ok(result)
  }
}
