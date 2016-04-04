cmake_minimum_required(VERSION 3.0.0 FATAL_ERROR)
project (qtcw)
find_package(Qt5Core REQUIRED)
message("Qt version: ${Qt5Core_VERSION}")
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include ${CMAKE_CURRENT_BINARY_DIR})

file(GLOB sources src/*.cpp)
file(GLOB headers include/*.h)
add_library(qtcw STATIC ${sources})
target_link_libraries(qtcw Qt5::Core icui18n icuuc icudata)

add_definitions(-DQTCW_LIBRARY)

install(TARGETS qtcw ARCHIVE DESTINATION lib)
install(FILES ${headers} DESTINATION include)

if(${CMAKE_COMPILER_IS_GNUCXX})
  set(CMAKE_CXX_FLAGS "-Wall -Wextra -Werror=return-type")
endif()
