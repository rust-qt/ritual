//! Types for handling information about C++ library APIs.


use cpp_method::{CppMethod, CppMethodKind};
pub use cpp_operator::CppOperator;
use cpp_type::{CppType, CppTypeBase, CppTypeIndirection, CppTypeClassBase};
use common::errors::{Result, ChainErr};
use common::file_utils::open_file;
use common::log;

use std::collections::{HashSet, HashMap};
use std::iter::once;
use std::io::{BufRead, BufReader};

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
  pub implicit_destructors: Vec<CppMethod>,
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
  /// Returns an iterator over all explicitly declared methods and implicit destructors.
  pub fn methods_and_implicit_destructors(
    &self,
  ) -> ::std::iter::Chain<::std::slice::Iter<CppMethod>, ::std::slice::Iter<CppMethod>> {
    self.parser.methods.iter().chain(
      self
        .processed
        .implicit_destructors
        .iter(),
    )
  }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct CppDataWithDeps<'a> {
  pub current: CppData,
  /// Data of dependencies
  pub dependencies: Vec<&'a CppData>,
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
            Some(
              arguments
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
                .collect(),
            )
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
      } = type_info.kind
      {
        if template_arguments.is_some() {
          return true;
        }
        for base in bases {
          if let CppTypeBase::Class(CppTypeClassBase {
                                      ref name,
                                      ref template_arguments,
                                    }) = base.base_type.base
          {
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
      log::llog(log::DebugGeneral, || {
        format!("Unknown type assumed to be non-template: {}", name)
      });
    }
    false
  }

  /// Checks if `class_name` types inherits `base_name` type directly or indirectly.
  pub fn inherits(&self, class_name: &str, base_name: &str, dependencies: &[&CppData]) -> bool {
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
  pub fn detect_signals_and_slots(&mut self, dependencies: &[&CppData]) -> Result<()> {
    let mut files = HashSet::new();
    for type1 in &self.types {
      if self.inherits(&type1.name, "QObject", dependencies) &&
        !files.contains(&type1.origin_location.include_file_path)
      {
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
        let line = line.chain_err(|| {
          format!("failed while reading lines from {}", &file_path)
        })?;
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
                        log::log(
                          log::DebugSignals,
                          format!("Found signal: {}", method.short_text()),
                        );
                      }
                      SectionType::Slots => {
                        log::log(
                          log::DebugSignals,
                          format!("Found slot: {}", method.short_text()),
                        );
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
}

impl TemplateArgumentsDeclaration {
  /// Returns count of the template arguments.
  #[allow(dead_code)]
  pub fn count(&self) -> usize {
    self.names.len()
  }
}




impl<'a> CppDataWithDeps<'a> {
  /// Returns true if `type1` is a known template instantiation.
  pub fn check_template_type(&self, type1: &CppType) -> Result<()> {
    if let CppTypeBase::Class(CppTypeClassBase {
                                ref name,
                                ref template_arguments,
                              }) = type1.base
    {
      if let Some(ref template_arguments) = *template_arguments {
        let is_valid = |cpp_data: &CppData| {
          cpp_data.processed.template_instantiations.iter().any(
            |inst| {
              &inst.class_name == name &&
                inst.instantiations.iter().any(|x| {
                  &x.template_arguments == template_arguments
                })
            },
          )
        };
        if !once(&self.current)
          .chain(self.dependencies.iter().map(|x| *x))
          .any(is_valid)
        {
          return Err(format!("type not available: {:?}", type1).into());
        }
        for arg in template_arguments {
          self.check_template_type(arg)?;
        }
      }
    }
    Ok(())
  }






  /// Returns selected type allocation place for type `class_name`.
  pub fn type_allocation_place(&self, class_name: &str) -> Result<CppTypeAllocationPlace> {
    if let Some(r) = self.current.processed.type_allocation_places.get(
      class_name,
    )
    {
      return Ok(r.clone());
    }
    for dep in &self.dependencies {
      if let Some(r) = dep.processed.type_allocation_places.get(class_name) {
        return Ok(r.clone());
      }
    }
    Err(
      format!("no type allocation place information for {}", class_name).into(),
    )
  }

  /// Search for a `CppTypeData` object in this `CppData` and all dependencies.
  pub fn find_type_info<F>(&self, f: F) -> Option<&CppTypeData>
  where
    F: Fn(&&CppTypeData) -> bool,
  {
    once(&self.current.parser.types)
      .chain(self.dependencies.iter().map(|d| &d.parser.types))
      .flat_map(|x| x)
      .find(f)
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








  /// Checks if specified class has any virtual methods (own or inherited).
  pub fn has_virtual_methods(&self, class_name: &str) -> bool {
    for method in self.current.parser.methods.iter().chain(
      self
        .current
        .processed
        .inherited_methods
        .iter(),
    )
    {
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
    for method in self.current.parser.methods.iter().chain(
      self
        .current
        .processed
        .inherited_methods
        .iter(),
    )
    {
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
    for method in &self.current.parser.methods {
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
      let type_info = self
        .find_type_info(|x| &x.name == &instantiations.class_name)
        .chain_err(|| {
          format!("type info not found for {}", &instantiations.class_name)
        })?;
      if !result.contains(&type_info.include_file) {
        result.insert(type_info.include_file.clone());
      }
    }
    Ok(result)
  }
}
