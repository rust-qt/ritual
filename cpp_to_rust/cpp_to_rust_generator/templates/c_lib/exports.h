#ifndef {lib_name_uppercase}_EXPORTS_H
#define {lib_name_uppercase}_EXPORTS_H

// This header creates a definition required to export the library's
// symbols properly on all platforms.

#ifdef _WIN32
    #ifdef {lib_name_uppercase}_LIBRARY
        #define {lib_name_uppercase}_EXPORT __declspec(dllexport)
    #else
        #define {lib_name_uppercase}_EXPORT __declspec(dllimport)
    #endif
#else
    #define {lib_name_uppercase}_EXPORT
#endif

#endif // {lib_name_uppercase}_EXPORTS_H

