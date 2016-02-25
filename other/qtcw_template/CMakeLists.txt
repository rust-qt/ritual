cmake_minimum_required(VERSION 3.0.0 FATAL_ERROR)
project (qtcw)
find_package(Qt5Core REQUIRED)
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include ${CMAKE_CURRENT_BINARY_DIR})

file(GLOB sources src/*.cpp)
add_library(qtcw SHARED ${sources})
target_link_libraries(qtcw Qt5::Core)

add_subdirectory(size_definer)

add_custom_target(
    run_size_definer
    COMMAND size_definer ${CMAKE_CURRENT_BINARY_DIR}/qtcw_sizes.h)
add_dependencies(qtcw run_size_definer)
add_definitions(-DQTCW_LIBRARY)

