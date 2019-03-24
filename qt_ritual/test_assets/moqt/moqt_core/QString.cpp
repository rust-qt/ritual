#include "QString.h"

const char *QByteArray::constData() const {
    return nullptr;
}

int QByteArray::size() const {
    return 0;
}

QString QString::fromUtf8(const char *str, int size) {
    return QString();
}

QByteArray QString::toUtf8() const {
    return QByteArray();
}
