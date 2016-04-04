#ifndef QTCW_GLOBAL_H
#define QTCW_GLOBAL_H

#ifndef __cplusplus // if C
  #include <stdbool.h>
#endif
#include <stdint.h>

#include "qtcw_exports.h"

#ifdef __cplusplus // if C++
  #define QTCW_EXTERN_C_BEGIN extern "C" {
  #define QTCW_EXTERN_C_END }
#else // if C
  #define QTCW_EXTERN_C_BEGIN
  #define QTCW_EXTERN_C_END
#endif

#ifdef __cplusplus // if C++
template<typename T>
void qtcw_call_destructor(T* x) {
    x->~T();
}
#endif


#endif // QTCW_GLOBAL_H
