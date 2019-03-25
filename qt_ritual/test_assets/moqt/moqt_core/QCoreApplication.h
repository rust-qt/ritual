#ifndef QCOREAPPLICATION_H
#define QCOREAPPLICATION_H

#include "moqt_core_exports.h"
#include "QObject.h"
#include "QString.h"

class MOQT_CORE_EXPORT QCoreApplication : public QObject {
public:
    QCoreApplication(int &argc, char **argv, int flags = 0);
    static int exec();

Q_SIGNALS:
    void appNameChanged(const QString& name);
};


#endif //QCOREAPPLICATION_H
