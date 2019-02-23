#ifndef QWINDOW_H
#define QWINDOW_H

#include <basic_class.h>
#include "QPoint.h"
#include "QObject.h"

class QWindow : public QObject {
public:
    QWindow() {}

    BasicClass getBasicClass() { return BasicClass(42); }
    BasicClass* getBasicClassPtr();

    QPoint pos() const;
    void setPos(const QPoint& pos);

    int showVectorOfInt(const QVector<int> &vec) { return vec.count(); }
    int showVectorOfWindows(const QVector<QWindow*> &vec) { return vec.count(); }

Q_SIGNALS:
    void posChanged(const QPoint& pos);

private:
    QPoint m_pos;
};
QWindow* get_window();


#endif //QWINDOW_H
