#ifndef QWINDOW_H
#define QWINDOW_H

#include <basic_class.h>
#include "QPoint.h"

class QWindow {
public:
    QWindow() {}

    BasicClass getBasicClass() { return BasicClass(42); }
    BasicClass* getBasicClassPtr();

    QPoint pos() const;
    void setPos(const QPoint& pos);

private:
    QPoint m_pos;
};
QWindow* get_window();


#endif //QWINDOW_H
