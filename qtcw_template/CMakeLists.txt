cmake_minimum_required(VERSION 3.0.0 FATAL_ERROR)
project (qtcw)
find_package(Qt5Core REQUIRED)
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include ${CMAKE_CURRENT_BINARY_DIR})

file(GLOB sources src/*.cpp)
file(GLOB headers include/*.h)
add_library(qtcw SHARED ${sources})
target_link_libraries(qtcw Qt5::Core)

add_subdirectory(size_definer)
add_custom_target(
    run_size_definer
    COMMAND size_definer ${CMAKE_CURRENT_BINARY_DIR}/qtcw_sizes.h)
add_dependencies(qtcw run_size_definer)

add_subdirectory(enum_values_definer)
add_custom_target(
    run_enum_values_definer
    COMMAND enum_values_definer ${CMAKE_CURRENT_BINARY_DIR}/qtcw_enum_values.h)
add_dependencies(qtcw run_enum_values_definer)


add_definitions(-DQTCW_LIBRARY)

install(TARGETS qtcw LIBRARY DESTINATION lib)

INSTALL(FILES ${headers} DESTINATION "include")
INSTALL(FILES ${CMAKE_CURRENT_BINARY_DIR}/qtcw_sizes.h ${CMAKE_CURRENT_BINARY_DIR}/qtcw_enum_values.h DESTINATION "include")

if(${CMAKE_COMPILER_IS_GNUCXX})
  set(CMAKE_CXX_FLAGS "-Wall -Wextra -Werror=return-type")
endif()
