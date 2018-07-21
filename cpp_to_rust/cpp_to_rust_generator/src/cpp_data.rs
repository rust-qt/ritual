//! Types for handling information about C++ library APIs.

pub use cpp_operator::CppOperator;
use cpp_type::{CppType, CppTypeBase, CppTypeClassBase};

use cpp_method::CppMethodKind;
use std::collections::HashMap;
use std::iter::once;

/// One item of a C++ enum declaration
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CppEnumValue {
  /// Identifier
  pub name: String,
  /// Corresponding value
  pub value: u64,
  /// C++ documentation for this item in HTML
  pub doc: Option<String>,
  /// Full type name of the enum this item belongs to
  pub enum_name: String,
}

impl CppEnumValue {
  pub fn is_same(&self, other: &CppEnumValue) -> bool {
    self.name == other.name && self.enum_name == other.enum_name && self.value == other.value
  }
}

/// Member field of a C++ class declaration
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppClassField {
  /// Identifier
  pub name: String,
  /// Field type
  pub field_type: CppType,
  /// Visibility
  pub visibility: CppVisibility,
  //  /// Size of type in bytes
  //  pub size: Option<usize>,
  /// Name and template arguments of the class type that owns this field
  pub class_type: CppTypeClassBase,

  pub is_const: bool,
  pub is_static: bool,
}

impl CppClassField {
  pub fn is_same(&self, other: &CppClassField) -> bool {
    // TODO: when doc is added to CppClassField, ignore it here
    self == other
  }

  pub fn short_text(&self) -> String {
    let visibility_text = match self.visibility {
      CppVisibility::Public => "",
      CppVisibility::Protected => "protected ",
      CppVisibility::Private => "private ",
    };
    format!(
      "class {} {{ {}{} {}; }}",
      self.class_type.to_cpp_pseudo_code(),
      visibility_text,
      self.field_type.to_cpp_pseudo_code(),
      self.name
    )
  }
}

/// Item of base class list in a class declaration
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppBaseSpecifier {
  /// Base class type (can include template arguments)
  pub base_class_type: CppTypeClassBase,
  /// Index of this base (for classes that have multiple base classes)
  pub base_index: usize,
  /// True if this base is virtual
  pub is_virtual: bool,
  /// Base visibility (public, protected or private)
  pub visibility: CppVisibility,

  /// Name and template arguments of the class type that
  /// inherits this base class
  pub derived_class_type: CppTypeClassBase,
}

/// Location of a C++ type's definition in header files.
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
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
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub enum CppVisibility {
  Public,
  Protected,
  Private,
}

/// C++ documentation for a type
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CppTypeDoc {
  /// HTML content
  pub html: String,
  /// Absolute URL to online documentation page for this type
  pub url: String,
  /// Absolute documentation URLs encountered in the content
  pub cross_references: Vec<String>,
}

/// Information about a C++ type declaration
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum CppTypeDataKind {
  Enum,
  Class {
    /// Information about name and template arguments of this type.
    type_base: CppTypeClassBase,
  },
}

/// Information about a C++ type declaration
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CppTypeData {
  /// Identifier, including namespaces and nested classes
  /// (separated with "::", like in C++)
  pub name: String,
  pub kind: CppTypeDataKind,
  /// C++ documentation for the type
  pub doc: Option<CppTypeDoc>,
  pub is_stack_allocated_type: bool,
}

impl CppTypeData {
  pub fn is_same(&self, other: &CppTypeData) -> bool {
    self.name == other.name && self.kind == other.kind
  }
}

/// Information about a C++ template class
/// instantiation.
#[derive(Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
pub struct CppTemplateInstantiation {
  /// Template class name
  pub class_name: String,
  /// List of template arguments used in this instantiation
  pub template_arguments: Vec<CppType>,
}

impl CppTypeDataKind {
  /// Checks if the type is a class type.
  pub fn is_class(&self) -> bool {
    match self {
      &CppTypeDataKind::Class { .. } => true,
      _ => false,
    }
  }
}
/*


impl<'a> CppDataWithDeps<'a> {


  /// Returns selected type allocation place for type `class_name`.
  pub fn type_allocation_place(&self, class_name: &str) -> Result<CppTypeAllocationPlace> {
    if let Some(r) = self
      .current
      .processed
      .type_allocation_places
      .get(class_name)
    {
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
  where
    F: Fn(&&CppTypeData) -> bool,
  {
    once(&self.current.parser.types)
      .chain(self.dependencies.iter().map(|d| &d.parser.types))
      .flat_map(|x| x)
      .find(f)
  }


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

  /// Checks if specified class has any virtual methods (own or inherited).
  pub fn has_virtual_methods(&self, class_name: &str) -> bool {
    for method in self
      .current
      .parser
      .methods
      .iter()
      .chain(self.current.processed.inherited_methods.iter())
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
    for method in self
      .current
      .parser
      .methods
      .iter()
      .chain(self.current.processed.inherited_methods.iter())
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
        .chain_err(|| format!("type info not found for {}", &instantiations.class_name))?;
      if !result.contains(&type_info.include_file) {
        result.insert(type_info.include_file.clone());
      }
    }
    Ok(result)
  }
}
*/
