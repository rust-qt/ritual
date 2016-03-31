add_executable(size_definer size_definer.cpp)
find_package(Qt5Core REQUIRED)
target_link_libraries(size_definer Qt5::Core icui18n icuuc icudata)
