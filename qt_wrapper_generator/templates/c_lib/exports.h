#ifndef {lib_name_uppercase}_EXPORTS_H
#define {lib_name_uppercase}_EXPORTS_H

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

