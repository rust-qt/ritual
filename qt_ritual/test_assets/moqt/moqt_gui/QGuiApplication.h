#ifndef QGUIAPPLICATION_H
#define QGUIAPPLICATION_H

#include "moqt_gui_exports.h"
#include "QObject.h"
#include "QString.h"
#include "QCoreApplication.h"

class MOQT_GUI_EXPORT QGuiApplication : public QCoreApplication {
public:
    QGuiApplication(int &argc, char **argv, int flags = 0);
    static int exec();
};


#endif //QGUIAPPLICATION_H
