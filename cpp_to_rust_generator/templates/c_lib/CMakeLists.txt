cmake_minimum_required(VERSION 3.0.0 FATAL_ERROR)
project ({lib_name_lowercase})

include_directories(${{CMAKE_CURRENT_SOURCE_DIR}}/include ${{CMAKE_CURRENT_BINARY_DIR}})

file(GLOB sources src/*.cpp)
file(GLOB headers include/*.h)
if(NOT("${{C2R_LIBRARY_TYPE}}" EQUAL "SHARED" OR "${{C2R_LIBRARY_TYPE}}" EQUAL "STATIC"))
  message(FATAL_ERROR "C2R_LIBRARY_TYPE must be either SHARED or STATIC." )
endif()
add_library({lib_name_lowercase} ${{C2R_LIBRARY_TYPE}} ${{sources}})
if(C2R_LINK_LIBRARIES)
  target_link_libraries({lib_name_lowercase} ${{C2R_LINK_LIBRARIES}})
endif()
if(C2R_INCLUDE_DIRECTORIES)
  include_directories(${{C2R_INCLUDE_DIRECTORIES}})
endif()
add_definitions(-D{lib_name_uppercase}_LIBRARY)

install(TARGETS {lib_name_lowercase}
  LIBRARY DESTINATION lib
  ARCHIVE DESTINATION lib
  RUNTIME DESTINATION lib
)

install(FILES ${{headers}} DESTINATION include)

if(${{CMAKE_COMPILER_IS_GNUCXX}})
  set(CMAKE_CXX_FLAGS "${{CMAKE_CXX_FLAGS}} -Wall -Wextra -Werror=return-type")
endif()

set (CMAKE_CXX_FLAGS "${{CMAKE_CXX_FLAGS}} ${{C2R_CPP_FLAGS}}")
