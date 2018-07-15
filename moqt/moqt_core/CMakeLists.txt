# This is a C++ library used by `full_run` test to perform
# an integration test of `cpp_to_rust`.
# The library is built and passed to `cpp_to_rust` to produce a crate.
# The test itself is located at `cpp_to_rust/cpp_to_rust_generator/src/tests/full_run.rs`.
#
# To enhance the test, add or change types and methods in this library
# and put corresponding tests in `?`
# directory.


cmake_minimum_required(VERSION 3.0.0 FATAL_ERROR)
project (moqtcore)
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include)

file(GLOB sources src/*.cpp)
file(GLOB headers include/*.h)
add_library(moqtcore SHARED ${sources})
set_target_properties(moqtcore PROPERTIES PUBLIC_HEADER "${headers}")
set (CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=c++11")
add_definitions(-DMOQT_CORE_LIBRARY)

install(TARGETS moqtcore
  LIBRARY DESTINATION lib
  ARCHIVE DESTINATION lib
  RUNTIME DESTINATION lib
  PUBLIC_HEADER DESTINATION include
)
