#ifndef QFLAGS_H
#define QFLAGS_H

#include "moqt_core_exports.h"

template<typename T>
class MOQT_CORE_EXPORT QFlags {
public:
    typedef unsigned int uint;
    QFlags(uint value) : m_value(value) {}
    operator uint const() { return m_value; }
    QFlags operator|(T other) const { return QFlags(m_value | uint(other)); }

private:
    int m_value;
};

#endif //QFLAGS_H
