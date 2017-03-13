cmake_minimum_required(VERSION 3.0.0 FATAL_ERROR)
project ({lib_name_lowercase})

include_directories(${{CMAKE_CURRENT_SOURCE_DIR}}/include ${{CMAKE_CURRENT_BINARY_DIR}})
if(C2R_INCLUDE_PATHS)
  include_directories(${{C2R_INCLUDE_PATHS}})
endif()
if(C2R_LIB_PATHS)
  link_directories(${{C2R_LIB_PATHS}})
endif()
# TODO: use C2R_FRAMEWORK_PATHS

file(GLOB sources src/*.cpp)
file(GLOB headers include/*.h)
if(NOT(("${{C2R_LIBRARY_TYPE}}" STREQUAL "SHARED") OR ("${{C2R_LIBRARY_TYPE}}" STREQUAL "STATIC")))
  message(FATAL_ERROR "C2R_LIBRARY_TYPE must be either SHARED or STATIC." )
endif()
add_library({lib_name_lowercase} ${{C2R_LIBRARY_TYPE}} ${{sources}})
if(C2R_LINKED_LIBS)
  target_link_libraries({lib_name_lowercase} ${{C2R_LINKED_LIBS}})
endif()

macro(add_osx_framework fwname target)
  find_library(FRAMEWORK_${{fwname}}
    NAMES ${{fwname}}
    PATHS ${{C2R_FRAMEWORK_PATHS}}
    NO_DEFAULT_PATH)
  if( ${{FRAMEWORK_${{fwname}}}} STREQUAL FRAMEWORK_${{fwname}}-NOTFOUND)
    message(FATAL_ERROR "Framework not found: ${{fwname}} (paths: ${{C2R_FRAMEWORK_PATHS}})")
  else()
    target_link_libraries(${{target}} "${{FRAMEWORK_${{fwname}}}}/${{fwname}}")
    message(STATUS "Framework ${{fwname}} found at ${{FRAMEWORK_${{fwname}}}}")
  endif()
endmacro(ADD_OSX_FRAMEWORK)

if(C2R_LINKED_FRAMEWORKS)
  foreach(framework ${{C2R_LINKED_FRAMEWORKS}})
    ADD_OSX_FRAMEWORK(${{framework}} {lib_name_lowercase})
  endforeach()
endif()


add_definitions(-D{lib_name_uppercase}_LIBRARY)

if(${{CMAKE_COMPILER_IS_GNUCXX}})
  set(CMAKE_CXX_FLAGS "${{CMAKE_CXX_FLAGS}} -Wall -Wextra -Werror=return-type")
endif()

if(C2R_COMPILER_FLAGS)
  set (CMAKE_CXX_FLAGS "${{CMAKE_CXX_FLAGS}} ${{C2R_COMPILER_FLAGS}}")
endif()

add_executable(type_sizes type_sizes.cpp)

if(C2R_LINKED_FRAMEWORKS)
  foreach(framework ${{C2R_LINKED_FRAMEWORKS}})
    ADD_OSX_FRAMEWORK(${{framework}} type_sizes)
  endforeach()
endif()


# disable qt verision tagging to allow compiling type_sizes without
# linking to the libraries
target_compile_definitions(type_sizes PRIVATE QT_NO_VERSION_TAGGING)

install(TARGETS {lib_name_lowercase} type_sizes
  LIBRARY DESTINATION lib
  ARCHIVE DESTINATION lib
  RUNTIME DESTINATION lib
)

install(FILES ${{headers}} DESTINATION include)
