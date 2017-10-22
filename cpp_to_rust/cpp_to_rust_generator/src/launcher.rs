//! Main function of the generator

/*
use config::Config;
use cpp_code_generator::{CppCodeGenerator, generate_cpp_type_size_requester, CppTypeSizeRequest};
use cpp_type::CppTypeClassBase;
use cpp_data::{CppData, CppDataWithDeps, ParserCppData};
use cpp_ffi_generator;
use cpp_parser;
use cpp_post_processor::cpp_post_process;
use common::errors::{Result, ChainErr};
use common::string_utils::CaseOperations;
use common::file_utils::{PathBufWithAdded, move_files, create_dir_all, save_json, load_bincode,
                         save_bincode, canonicalize, remove_dir_all, remove_dir, read_dir,
                         create_file, path_to_str};
use common::BuildScriptData;
use common::log;
use rust_code_generator;
use rust_generator;
use rust_info::{RustTypeWrapperKind, RustExportInfo, DependencyInfo};

use std::path::PathBuf;
use std::collections::HashMap;

/// Loads `RustExportInfo` and `CppData` or a dependency previously
/// processed in the cache directory `path`.
fn load_dependency(path: &PathBuf) -> Result<DependencyInfo> {
  log::status(format!("Loading files from {}", path.display()));
  let parser_cpp_data_path = path.with_added("parser_cpp_data.bin");
  if !parser_cpp_data_path.exists() {
    return Err(
      format!("file not found: {}", parser_cpp_data_path.display()).into(),
    );
  }
  let parser_cpp_data = load_bincode(&parser_cpp_data_path)?;



  let processed_cpp_data_path = path.with_added("processed_cpp_data.bin");
  if !processed_cpp_data_path.exists() {
    return Err(
      format!("file not found: {}", processed_cpp_data_path.display()).into(),
    );
  }
  let processed_cpp_data = load_bincode(&processed_cpp_data_path)?;
  let cpp_data = CppData {
    parser: parser_cpp_data,
    processed: processed_cpp_data,
  };
  let rust_export_info_path = path.with_added("rust_export_info.bin");
  if !rust_export_info_path.exists() {
    return Err(
      format!("file not found: {}", rust_export_info_path.display()).into(),
    );
  }
  let rust_export_info = load_bincode(&rust_export_info_path)?;
  Ok(DependencyInfo {
    rust_export_info: rust_export_info,
    cpp_data: cpp_data,
    cache_path: path.clone(),
  })
}



/// Executes the generator for multiple configs.
pub fn exec<T: Iterator<Item = Config>>(configs: T) -> Result<()> {
  let mut dependency_cache = HashMap::new();
  for config in configs {


    {
      let cpp_data = load_or_create_cpp_data(
        &config,
        dependencies.iter().map(|dep| &dep.cpp_data).collect(),
      )?;
      let output_path_existed = config.output_dir_path().with_added("src").exists();

      let c_lib_path = config.output_dir_path().with_added("c_lib");
      let c_lib_path_existed = c_lib_path.exists();


      let cpp_ffi_lib_name = format!("{}_c", &config.crate_properties().name());
      let c_lib_tmp_path = if c_lib_path_existed {
        let path = config.cache_dir_path().with_added("c_lib.new");
        if path.exists() {
          remove_dir_all(&path)?;
        }
        path
      } else {
        c_lib_path.clone()
      };
      create_dir_all(&c_lib_tmp_path)?;
      log::status(format!(
        "Generating C++ wrapper library ({})",
        cpp_ffi_lib_name
      ));

      let cpp_ffi_headers = cpp_ffi_generator::run(
        &cpp_data,
        cpp_ffi_lib_name.clone(),
        config.cpp_ffi_generator_filters(),
      ).chain_err(|| "FFI generator failed")?;

      log::status(format!("Generating C++ wrapper code"));
      let code_gen = CppCodeGenerator::new(cpp_ffi_lib_name.clone(), c_lib_tmp_path.clone());
      code_gen.generate_template_files(
        config.include_directives(),
      )?;
      code_gen.generate_files(&cpp_ffi_headers)?;

      let crate_new_path = if output_path_existed {
        let path = config.cache_dir_path().with_added(format!(
          "{}.new",
          &config.crate_properties().name()
        ));
        if path.as_path().exists() {
          remove_dir_all(&path)?;
        }
        path
      } else {
        config.output_dir_path().clone()
      };
      create_dir_all(&crate_new_path)?;
      let rust_config = rust_code_generator::RustCodeGeneratorConfig {
        crate_properties: config.crate_properties().clone(),
        output_path: crate_new_path.clone(),
        crate_template_path: config.crate_template_path().cloned(),
        cpp_ffi_lib_name: cpp_ffi_lib_name.clone(),
        generator_dependencies: &dependencies,
        write_dependencies_local_paths: config.write_dependencies_local_paths(),
        cpp_lib_version: config.cpp_lib_version().map(|s| s.into()),
      };
      log::status("Preparing Rust functions");
      let rust_data = rust_generator::RustGeneratorInputData {
        cpp_data: &cpp_data,
        cpp_ffi_headers: cpp_ffi_headers,
        dependency_types: dependencies
          .iter()
          .map(|dep| &dep.rust_export_info.rust_types as &[_])
          .collect(),
        crate_name: config.crate_properties().name().clone(),
        // TODO: more universal prefix removal (#25)
        remove_qt_prefix: remove_qt_prefix,
        filtered_namespaces: config.cpp_filtered_namespaces().clone(),
      }.run()
        .chain_err(|| "Rust data generator failed")?;
      log::status(format!(
        "Generating Rust crate code ({})",
        &config.crate_properties().name()
      ));
      rust_code_generator::run(rust_config, &rust_data)
        .chain_err(|| "Rust code generator failed")?;
      let mut cpp_type_size_requests = Vec::new();
      for type1 in &rust_data.processed_types {
        if let RustTypeWrapperKind::Struct { ref size_const_name, .. } = type1.kind {
          if let Some(ref size_const_name) = *size_const_name {
            cpp_type_size_requests.push(CppTypeSizeRequest {
              cpp_code: CppTypeClassBase {
                name: type1.cpp_name.clone(),
                template_arguments: type1.cpp_template_arguments.clone(),
              }.to_cpp_code()?,
              size_const_name: size_const_name.clone(),
            });
          }
        }
      }
      {
        let mut file = create_file(c_lib_tmp_path.with_added("type_sizes.cpp"))?;
        file.write(generate_cpp_type_size_requester(
          &cpp_type_size_requests,
          config.include_directives(),
        )?)?;
      }
      if c_lib_path_existed {
        move_files(&c_lib_tmp_path, &c_lib_path)?;
      }
      let rust_export_info = RustExportInfo {
        crate_name: config.crate_properties().name().clone(),
        crate_version: config.crate_properties().version().clone(),
        rust_types: rust_data.processed_types,
        output_path: path_to_str(config.output_dir_path())?.to_string(),
      };
      if config.write_cache() {
        let rust_export_path = config.cache_dir_path().with_added("rust_export_info.bin");
        log::status("Saving Rust export info");
        save_bincode(&rust_export_path, &rust_export_info)?;
        log::status(format!(
          "Rust export info is saved to file: {}",
          rust_export_path.display()
        ));
      }

      if output_path_existed {
        // move all generated top level files and folders (and delete corresponding old folders)
        // but keep existing unknown top level files and folders, such as "target" or ".cargo"
        for item in read_dir(&crate_new_path)? {
          let item = item?;
          move_files(
            &crate_new_path.with_added(item.file_name()),
            &config.output_dir_path().with_added(item.file_name()),
          )?;
        }
        remove_dir(&crate_new_path)?;
      }
      save_json(
        config.output_dir_path().with_added(
          "build_script_data.json",
        ),
        &BuildScriptData {
          cpp_build_config: config.cpp_build_config().clone(),
          cpp_wrapper_lib_name: cpp_ffi_lib_name,
          cpp_lib_version: config.cpp_lib_version().map(|s| s.to_string()),
        },
      )?;
      dependency_cache.insert(
        config.cache_dir_path().clone(),
        DependencyInfo {
          cpp_data: cpp_data.current,
          rust_export_info: rust_export_info,
          cache_path: config.cache_dir_path().clone(),
        },
      );
    }
    for dep in dependencies {
      dependency_cache.insert(dep.cache_path.clone(), dep);
    }
  }
  log::status("cpp_to_rust generator finished");
  Ok(())
}
*/
