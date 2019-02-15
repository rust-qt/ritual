#ifndef MOQT_GUI_EXPORTS_H
#define MOQT_GUI_EXPORTS_H

#ifdef _WIN32
    #ifdef MOQT_GUI_LIBRARY
        #define MOQT_GUI_EXPORT __declspec(dllexport)
    #else
        #define MOQT_GUI_EXPORT __declspec(dllimport)
    #endif
#else
    #define MOQT_GUI_EXPORT
#endif

#endif // MOQT_GUI_EXPORTS_H

