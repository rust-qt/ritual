#ifndef QBYTEARRAY_H
#define QBYTEARRAY_H

#include "moqt_core_exports.h"

class MOQT_CORE_EXPORT QByteArray {
public:
    QByteArray(int, char) {}
    QByteArray(const char*, int) {}
    QByteArray() {}
    char* data();
    const char * data() const;
    const char *constData() const;
    int size() const;
    const char* begin() const;
    const char* end() const;
};

#endif //QBYTEARRAY_H
