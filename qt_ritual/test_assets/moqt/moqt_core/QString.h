#ifndef QSTRING_H
#define QSTRING_H

#include "moqt_core_exports.h"

class MOQT_CORE_EXPORT QByteArray {
public:
    const char *constData() const;
    int size() const;
};

class MOQT_CORE_EXPORT QString {
public:
    static QString fromUtf8(const char *str, int size = -1);
    static QString fromUtf8(const QByteArray& str);
    QByteArray toUtf8() const;
};


#endif //QSTRING_H
