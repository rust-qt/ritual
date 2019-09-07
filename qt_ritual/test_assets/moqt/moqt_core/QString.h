#ifndef QSTRING_H
#define QSTRING_H

#include "moqt_core_exports.h"

class MOQT_CORE_EXPORT QByteArray {
public:
    QByteArray(int, char) {}
    QByteArray(const char*, int) {}
    QByteArray() {}
    const char *constData() const;
    int size() const;
};

class QString;

class MOQT_CORE_EXPORT QDebug {
public:
    QDebug() {}
    QDebug(QString*) {}
    QDebug(int) {}
};

class MOQT_CORE_EXPORT QString {
public:
    static QString fromUtf8(const char *str, int size = -1);
    static QString fromUtf8(const QByteArray& str);
    QByteArray toUtf8() const;
};


#endif //QSTRING_H
