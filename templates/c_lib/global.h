#ifndef {lib_name_uppercase}_GLOBAL_H
#define {lib_name_uppercase}_GLOBAL_H

#include <stdint.h>

// placement new statements require this
#include <new>

#include "{cpp_lib_include_file}"

#include "{lib_name_lowercase}_exports.h"

#ifdef __cplusplus // if C++
template<typename T>
void {lib_name_lowercase}_call_destructor(T* x) {{
    x->~T();
}}
#endif


#endif // {lib_name_uppercase}_GLOBAL_H
