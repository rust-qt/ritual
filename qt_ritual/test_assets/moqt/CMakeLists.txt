# This is a C++ library used by `full_run` test to perform
# an integration test of `ritual`.
# The library is built and passed to `ritual` to produce a crate.
# The test itself is located at `qt_ritual/src/test_moqt.rs`.
#
# To enhance the test, add or change types and methods in this library
# and put corresponding tests in `?`
# directory.


cmake_minimum_required(VERSION 3.0.0 FATAL_ERROR)
project (moqt)

foreach(lib_name moqt_core moqt_gui)
    string(TOUPPER ${lib_name} lib_name_uppercase)
    include_directories(${CMAKE_CURRENT_SOURCE_DIR}/${lib_name})

    file(GLOB sources ${lib_name}/*.cpp)
    file(GLOB headers ${lib_name}/*.h)
    add_library(${lib_name} SHARED ${sources})
    if(lib_name STREQUAL moqt_gui)
        target_link_libraries(moqt_gui moqt_core)
    endif()

    set_target_properties(${lib_name} PROPERTIES PUBLIC_HEADER "${headers}")
    set (CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=c++11")
    add_definitions(-D${lib_name_uppercase}_LIBRARY)

    install(TARGETS ${lib_name}
        LIBRARY DESTINATION lib
        ARCHIVE DESTINATION lib
        RUNTIME DESTINATION lib
        PUBLIC_HEADER DESTINATION include/${lib_name}
    )
endforeach()
