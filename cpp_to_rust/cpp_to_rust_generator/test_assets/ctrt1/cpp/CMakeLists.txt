# This is a C++ library used by `full_run` test to perform
# an integration test of `cpp_to_rust`.
# The library is built and passed to `cpp_to_rust` to produce a crate.
# The test itself is located at `cpp_to_rust/cpp_to_rust_generator/src/tests/full_run.rs`.
#
# To enhance the test, add or change types and methods in this library
# and put corresponding tests in `cpp_to_rust/cpp_to_rust_generator/test_assets/ctrt1/crate/tests`
# directory.


cmake_minimum_required(VERSION 3.0.0 FATAL_ERROR)
project (ctrt1)
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include)

file(GLOB sources src/*.cpp)
file(GLOB headers include/*.h)
add_library(ctrt1 SHARED ${sources})
set (CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=c++11")
add_definitions(-DCTRT1_LIBRARY)

install(TARGETS ctrt1
  LIBRARY DESTINATION lib
  ARCHIVE DESTINATION lib
  RUNTIME DESTINATION lib
)
