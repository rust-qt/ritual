#ifndef QWINDOW_H
#define QWINDOW_H

#include <basic_class.h>

class QWindow {
public:
    QWindow() {}

    BasicClass getBasicClass() { return BasicClass(42); }
    BasicClass* getBasicClassPtr();
};
QWindow* get_window();


#endif //QWINDOW_H
