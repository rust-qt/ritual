#ifndef QWINDOW_H
#define QWINDOW_H

#include "moqt_gui_exports.h"
#include "basic_class.h"
#include "QPoint.h"
#include "QObject.h"
#include "namespaces.h"

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

MOQT_GUI_EXPORT ns1::Templated1<int> get_same_template1();
MOQT_GUI_EXPORT ns1::ClassNs::Templated2<bool> get_same_template2();
MOQT_GUI_EXPORT ignored_ns::Templated3<int> get_same_template3();

MOQT_GUI_EXPORT ns1::Templated1<float> get_new_template1();
MOQT_GUI_EXPORT ns1::ClassNs::Templated2<float> get_new_template2();
MOQT_GUI_EXPORT ignored_ns::Templated3<float> get_new_template3();

#endif //QWINDOW_H
