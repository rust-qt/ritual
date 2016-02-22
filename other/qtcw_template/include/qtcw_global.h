#ifndef QTCW_GLOBAL_H
#define QTCW_GLOBAL_H

#ifndef __cplusplus
  #include <stdbool.h>
#endif

#include "exports.h"
#include "sizes.h"


#define QTCW_DECLARE_TYPE(name) \
  struct QTCW_Struct_##name { \
    char _space[QTCW_sizeof_##name]; \
  }; \
  typedef struct QTCW_Struct_##name QTCW_##name


#define QTCW_RIC(T, name) T* c_##name = reinterpret_cast<T*>(name)
#define QTCW_RICS(T) QTCW_RIC(T, self)

#endif // QTCW_GLOBAL_H
