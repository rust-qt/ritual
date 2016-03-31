#ifndef EXPORTS_H
#define EXPORTS_H

#ifdef _WIN32
    #ifdef QTCW_LIBRARY
        #define QTCW_EXPORT __declspec(dllexport)
    #else
        #define QTCW_EXPORT __declspec(dllimport)
    #endif
#else
    #define QTCW_EXPORT
#endif

#endif // EXPORTS_H

