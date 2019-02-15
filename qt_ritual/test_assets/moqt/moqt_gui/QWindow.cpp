#include "QWindow.h"

QWindow* get_window() {
    return nullptr;
}

BasicClass* QWindow::getBasicClassPtr() {
    auto p = new BasicClass(43);
    p->foo();
    return p;
}

QPoint QWindow::pos() const {
    return m_pos;
}
void QWindow::setPos(const QPoint& pos) {
    m_pos = pos;
    m_pos.setX(55);
}
