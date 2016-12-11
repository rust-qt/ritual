#ifndef CTRT1_EXPORTS_H
#define CTRT1_EXPORTS_H

#ifdef _WIN32
    #ifdef CTRT1_LIBRARY
        #define CTRT1_EXPORT __declspec(dllexport)
    #else
        #define CTRT1_EXPORT __declspec(dllimport)
    #endif
#else
    #define CTRT1_EXPORT
#endif

#endif // CTRT1_EXPORTS_H

