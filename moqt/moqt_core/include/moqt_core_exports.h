#ifndef MOQT_CORE_EXPORTS_H
#define MOQT_CORE_EXPORTS_H

#ifdef _WIN32
    #ifdef MOQT_CORE_LIBRARY
        #define MOQT_CORE_EXPORT __declspec(dllexport)
    #else
        #define MOQT_CORE_EXPORT __declspec(dllimport)
    #endif
#else
    #define MOQT_CORE_EXPORT
#endif

#endif // MOQT_CORE_EXPORTS_H

