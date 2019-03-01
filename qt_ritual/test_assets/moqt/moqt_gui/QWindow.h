#ifndef QWINDOW_H
#define QWINDOW_H

#include "moqt_gui_exports.h"
#include <basic_class.h>
#include "QPoint.h"
#include "QObject.h"

class MOQT_GUI_EXPORT QWindow : public QObject {
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
MOQT_GUI_EXPORT QWindow* get_window();


#endif //QWINDOW_H
