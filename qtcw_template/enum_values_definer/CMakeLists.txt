add_executable(enum_values_definer enum_values_definer.cpp)
find_package(Qt5Core REQUIRED)
target_link_libraries(enum_values_definer Qt5::Core icui18n icuuc icudata)
