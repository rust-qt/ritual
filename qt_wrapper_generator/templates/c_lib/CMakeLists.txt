cmake_minimum_required(VERSION 3.0.0 FATAL_ERROR)
project ({lib_name_lowercase})

include_directories(${{CMAKE_CURRENT_SOURCE_DIR}}/include ${{CMAKE_CURRENT_BINARY_DIR}})

file(GLOB sources src/*.cpp)
file(GLOB headers include/*.h)
add_library({lib_name_lowercase} STATIC ${{sources}})
target_link_libraries({lib_name_lowercase} {link_libraries})
include_directories({include_directories})

add_definitions(-D{lib_name_uppercase}_LIBRARY)

install(TARGETS {lib_name_lowercase} ARCHIVE DESTINATION lib)
install(FILES ${{headers}} DESTINATION include)

if(${{CMAKE_COMPILER_IS_GNUCXX}})
  set(CMAKE_CXX_FLAGS "${{CMAKE_CXX_FLAGS}} -Wall -Wextra -Werror=return-type")
endif()
set (CMAKE_CXX_FLAGS "${{CMAKE_CXX_FLAGS}} -fPIC")
