cmake_minimum_required(VERSION 3.0.0 FATAL_ERROR)
project (rust_qt_generator)
find_package(Qt5Core REQUIRED)
message("Qt version: ${Qt5Core_VERSION}")

set(QT_LINK_LIBS Qt5::Core icui18n icuuc icudata)

set(QT_DOC_DIR
  CACHE PATH "Path to Qt documentation HTML files")
SET(QTCW_MAKE_OPTIONS
  CACHE STRING "Options for building QTCW (e.g. '-j8'")
message("QT_DOC_DIR = ${QT_DOC_DIR}")
if(NOT IS_DIRECTORY ${QT_DOC_DIR})
  message(FATAL_ERROR "Invalid QT_DOC_DIR")
endif()

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

add_custom_target(${PROJECT_NAME}
  ALL
  DEPENDS ${QTCW_INSTALL_DIR})

add_custom_command(
  OUTPUT ${DOC_PARSE_RESULT_PATH}
  COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/qt_doc_parser.py ${QT_DOC_DIR} ${DOC_PARSE_RESULT_PATH}
  DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/qt_doc_parser.py ${QT_DOC_FILES}
)

CONFIGURE_FILE(${CMAKE_SOURCE_DIR}/qt_wrapper_generator/cargo_config_template ${CMAKE_BINARY_DIR}/.cargo/config)

add_custom_command(
  OUTPUT ${RUST_GENERATOR_EXE}
  COMMAND cargo build --manifest-path ${CMAKE_SOURCE_DIR}/qt_wrapper_generator/Cargo.toml
  DEPENDS ${CMAKE_BINARY_DIR}/.cargo/config ${RUST_GENERATOR_FILES}
)

add_custom_command(
  OUTPUT ${CPP_INFO_EXTRACTOR_ACTIONS_PATH}
  COMMAND ${RUST_GENERATOR_EXE} stage1 ${DOC_PARSE_RESULT_PATH} ${CPP_INFO_EXTRACTOR_ACTIONS_PATH}
  DEPENDS ${RUST_GENERATOR_EXE} ${DOC_PARSE_RESULT_PATH}
)

add_executable(cpp_info_extractor cpp_info_extractor.cpp ${CPP_INFO_EXTRACTOR_ACTIONS_PATH})
include_directories(${CMAKE_BINARY_DIR})
target_link_libraries(cpp_info_extractor ${QT_LINK_LIBS})

add_custom_command(
  OUTPUT ${CPP_EXTRACTED_INFO_PATH}
  COMMAND cpp_info_extractor ${CPP_EXTRACTED_INFO_PATH}
)

add_custom_command(
  OUTPUT ${QTCW_GENERATED_SOURCE_DIR} ${RUST_QT_GENERATED_SOURCE_DIR}
  COMMAND
    ${CMAKE_COMMAND} -E copy_directory ${CMAKE_SOURCE_DIR}/qtcw_template ${QTCW_GENERATED_SOURCE_DIR} &&
    ${CMAKE_COMMAND} -E copy_directory ${CMAKE_SOURCE_DIR}/rust_qt_template ${RUST_QT_GENERATED_SOURCE_DIR} &&
    ${RUST_GENERATOR_EXE} stage2 ${DOC_PARSE_RESULT_PATH} ${CPP_EXTRACTED_INFO_PATH} ${QTCW_GENERATED_SOURCE_DIR} ${RUST_QT_GENERATED_SOURCE_DIR}
  DEPENDS ${RUST_GENERATOR_EXE} ${QTCW_TEMPLATE_FILES} ${CPP_EXTRACTED_INFO_PATH}
)

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
