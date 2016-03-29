#ifndef QTCW_GLOBAL_H
#define QTCW_GLOBAL_H

#ifndef __cplusplus // if C
  #include <stdbool.h>
  #include <wchar.h>
#endif
#include <stdint.h>

#include "qtcw_exports.h"
#include "qtcw_sizes.h"

#ifdef __cplusplus // if C++
  #define QTCW_EXTERN_C_BEGIN extern "C" {
  #define QTCW_EXTERN_C_END }
#else
  #define QTCW_EXTERN_C_BEGIN
  #define QTCW_EXTERN_C_END
#endif

#endif // QTCW_GLOBAL_H
