#ifndef QCOREAPPLICATION_H
#define QCOREAPPLICATION_H

#include "moqt_core_exports.h"

class MOQT_CORE_EXPORT QCoreApplication {
public:
    QCoreApplication(int &argc, char **argv, int flags = 0);
    static int exec();
};


#endif //QCOREAPPLICATION_H
