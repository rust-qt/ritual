cmake_minimum_required(VERSION 3.0.0 FATAL_ERROR)
project (ctrt1)

include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include)

file(GLOB sources src/*.cpp)
file(GLOB headers include/*.h)
add_library(ctrt1 SHARED ${sources})

add_definitions(-DCTRT1_LIBRARY)

install(TARGETS ctrt1
  LIBRARY DESTINATION lib
  ARCHIVE DESTINATION lib
  RUNTIME DESTINATION lib
)
