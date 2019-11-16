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

ns1::Templated1<int> get_same_template1() {
    return ns1::Templated1<int>();
}
ns1::ClassNs::Templated2<bool> get_same_template2() {
    return ns1::ClassNs::Templated2<bool>();
}
ignored_ns::Templated3<int> get_same_template3() {
    return ignored_ns::Templated3<int>();
}
ns1::Templated1<float> get_new_template1() {
    return ns1::Templated1<float>();
}
ns1::ClassNs::Templated2<float> get_new_template2() {
    return ns1::ClassNs::Templated2<float>();
}
ignored_ns::Templated3<float> get_new_template3() {
    return ignored_ns::Templated3<float>();
}
