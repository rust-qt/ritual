#include "QWindow.h"

QWindow* get_window() {
    return nullptr;
}

BasicClass* QWindow::getBasicClassPtr() {
    auto p = new BasicClass(43);
    p->foo();
    return p;
}
