cmake_minimum_required(VERSION 3.0.0 FATAL_ERROR)
project (rust_qt_generator)
find_package(Qt5Core REQUIRED)
message("Qt version: ${Qt5Core_VERSION}")

get_target_property(QT_CORE_LOCATION Qt5::Core LOCATION)
# get_filename_component(CPP_LIB_DIR ${QT_CORE_LOCATION} DIRECTORY)

execute_process(
  COMMAND qmake -query QT_INSTALL_LIBS
  OUTPUT_VARIABLE CPP_LIB_DIR)
string(STRIP "${CPP_LIB_DIR}" CPP_LIB_DIR)
message("Qt library path: ${CPP_LIB_DIR}")

set(QT_DOC_DIR
  CACHE PATH "Path to Qt documentation HTML files")
SET(QTCW_MAKE_OPTIONS
  CACHE STRING "Options for building QTCW (e.g. '-j8'")
SET(QTCW_COMPILE_BLOCKED
  CACHE BOOL "Block recompilation of QTCW library")

#message("QT_DOC_DIR = ${QT_DOC_DIR}")
#if(NOT IS_DIRECTORY ${QT_DOC_DIR})
#  message(FATAL_ERROR "Invalid QT_DOC_DIR")
#endif()
message("QTCW_COMPILE_BLOCKED = ${QTCW_COMPILE_BLOCKED}")

set(DOC_PARSE_RESULT_PATH ${CMAKE_BINARY_DIR}/doc_parse_result.json)
set(CPP_INFO_EXTRACTOR_ACTIONS_PATH ${CMAKE_BINARY_DIR}/cpp_info_extractor_actions.h)
set(RUST_GENERATOR_BUILD_DIR ${CMAKE_BINARY_DIR}/qt_wrapper_generator)
set(QTCW_GENERATED_SOURCE_DIR ${CMAKE_BINARY_DIR}/qtcw)
set(RUST_GENERATOR_EXE ${RUST_GENERATOR_BUILD_DIR}/debug/qt_wrapper_generator)
set(CPP_EXTRACTED_INFO_PATH ${CMAKE_BINARY_DIR}/cpp_extracted_info.json)
set(RUST_QT_GENERATED_SOURCE_DIR ${CMAKE_BINARY_DIR}/rust_qt)
set(QTCW_BUILD_DIR ${CMAKE_BINARY_DIR}/build_qtcw)
set(QTCW_INSTALL_DIR ${CMAKE_BINARY_DIR}/install_qtcw)

file(GLOB_RECURSE QT_DOC_FILES ${QT_DOC_DIR}/*.html)
file(GLOB_RECURSE RUST_GENERATOR_FILES ${CMAKE_SOURCE_DIR}/qt_wrapper_generator/*.rs)
file(GLOB_RECURSE QTCW_TEMPLATE_FILES ${CMAKE_SOURCE_DIR}/qtcw_template/*)
file(GLOB_RECURSE RUST_QT_TEMPLATE_FILES ${CMAKE_SOURCE_DIR}/rust_qt_template/*)
file(GLOB_RECURSE QTCW_TEST_FILES ${CMAKE_SOURCE_DIR}/c_test/*)

# phantom target for setting final output files
add_custom_target(${PROJECT_NAME}
  ALL
  DEPENDS build_rust_qt)

# run python parser

#add_custom_command(
#  OUTPUT ${DOC_PARSE_RESULT_PATH}
#  COMMAND ${CMAKE_SOURCE_DIR}/qt_doc_parser.py ${QT_DOC_DIR} ${DOC_PARSE_RESULT_PATH}
#  DEPENDS ${CMAKE_SOURCE_DIR}/qt_doc_parser.py ${QT_DOC_FILES}
#)

# cargo config file for changing build directory
CONFIGURE_FILE(${CMAKE_SOURCE_DIR}/qt_wrapper_generator/cargo_config_template ${CMAKE_BINARY_DIR}/.cargo/config)

# building main generator
add_custom_command(
  OUTPUT ${RUST_GENERATOR_EXE}
  COMMAND cargo build --manifest-path ${CMAKE_SOURCE_DIR}/qt_wrapper_generator/Cargo.toml
  DEPENDS ${CMAKE_BINARY_DIR}/.cargo/config ${RUST_GENERATOR_FILES}
)

# running main generator for generating cpp extractor actions

#add_custom_command(
#  OUTPUT ${CPP_INFO_EXTRACTOR_ACTIONS_PATH}
#  COMMAND ${RUST_GENERATOR_EXE} stage1 ${DOC_PARSE_RESULT_PATH} ${CPP_INFO_EXTRACTOR_ACTIONS_PATH}
#  DEPENDS ${RUST_GENERATOR_EXE} ${DOC_PARSE_RESULT_PATH}
#)

# building cpp extractor

#add_executable(cpp_info_extractor cpp_info_extractor.cpp ${CPP_INFO_EXTRACTOR_ACTIONS_PATH})
#include_directories(${CMAKE_BINARY_DIR})
#target_link_libraries(cpp_info_extractor ${Qt5Core_LIBRARIES})

# running cpp extractor

#add_custom_command(
#  OUTPUT ${CPP_EXTRACTED_INFO_PATH}
#  COMMAND cpp_info_extractor ${CPP_EXTRACTED_INFO_PATH}
#)

# running main generator for generating C and Rust wrappers
add_custom_command(
  OUTPUT ${QTCW_GENERATED_SOURCE_DIR} ${RUST_QT_GENERATED_SOURCE_DIR}
  COMMAND
    ${CMAKE_COMMAND} -E copy_directory ${CMAKE_SOURCE_DIR}/qtcw_template ${QTCW_GENERATED_SOURCE_DIR} &&
    ${CMAKE_COMMAND} -E copy_directory ${CMAKE_SOURCE_DIR}/rust_qt_template ${RUST_QT_GENERATED_SOURCE_DIR} &&
    ${RUST_GENERATOR_EXE} stage0 ${QTCW_GENERATED_SOURCE_DIR} ${RUST_QT_GENERATED_SOURCE_DIR}
    #${RUST_GENERATOR_EXE} stage2 ${DOC_PARSE_RESULT_PATH} ${CPP_EXTRACTED_INFO_PATH} ${QTCW_GENERATED_SOURCE_DIR} ${RUST_QT_GENERATED_SOURCE_DIR}
  DEPENDS ${RUST_GENERATOR_EXE} ${QTCW_TEMPLATE_FILES} ${RUST_QT_TEMPLATE_FILES}
  #${CPP_EXTRACTED_INFO_PATH}
)

if(${QTCW_COMPILE_BLOCKED})
  message("qtcw compilation is blocked.")
else()
  # building generated C wrapper
  file(MAKE_DIRECTORY ${QTCW_BUILD_DIR})
  add_custom_command(
    COMMAND
      ${CMAKE_COMMAND} -E remove_directory ${QTCW_INSTALL_DIR} &&
      ${CMAKE_COMMAND} ${QTCW_GENERATED_SOURCE_DIR}
        -DCMAKE_PREFIX_PATH=${CMAKE_PREFIX_PATH}
        -DCMAKE_INSTALL_PREFIX=${QTCW_INSTALL_DIR} &&
      make ${QTCW_MAKE_OPTIONS} install
    OUTPUT ${QTCW_INSTALL_DIR}
    DEPENDS ${QTCW_GENERATED_SOURCE_DIR}
    WORKING_DIRECTORY ${QTCW_BUILD_DIR}
  )
  add_custom_target(qtcw_installed DEPENDS ${QTCW_INSTALL_DIR})

  # building and running C tests

  #include_directories(${QTCW_INSTALL_DIR}/include)
  #foreach(QTCW_TEST_FILE ${QTCW_TEST_FILES})
  #  get_filename_component(TEST_NAME ${QTCW_TEST_FILE} NAME_WE)
  #  set(TEST_TARGET "c_test_${TEST_NAME}")
  #  add_executable(${TEST_TARGET} ${QTCW_TEST_FILE})
  #  add_dependencies(${TEST_TARGET} qtcw_installed)
  #  target_link_libraries(${TEST_TARGET} ${QTCW_INSTALL_DIR}/lib/libqtcw.a ${Qt5Core_LIBRARIES} -lstdc++)
  #  add_custom_command(
  #    TARGET ${TEST_TARGET}
  #    POST_BUILD
  #    COMMAND ${TEST_TARGET}
  #  )
  #  set(TEST_TARGETS ${TEST_TARGETS} ${TEST_TARGET})
  #endforeach()

endif()

# building and testing rust_qt crates
add_custom_target(build_rust_qt
  COMMAND
    QTCW_LIB_DIR=${QTCW_INSTALL_DIR}/lib
    LIBRARY_PATH=${CPP_LIB_DIR}
    LD_LIBRARY_PATH=${CPP_LIB_DIR}
    cargo test
  WORKING_DIRECTORY ${RUST_QT_GENERATED_SOURCE_DIR}/qt_core
  DEPENDS ${RUST_QT_GENERATED_SOURCE_DIR} ${QTCW_INSTALL_DIR}
  #${TEST_TARGETS}
)
