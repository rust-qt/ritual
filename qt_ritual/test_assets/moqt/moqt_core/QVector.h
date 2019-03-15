#ifndef QVECTOR_H
#define QVECTOR_H

#include "moqt_core_exports.h"
#include <vector>

template<typename T>
class MOQT_CORE_EXPORT SomethingElse {};

template<typename T>
class MOQT_CORE_EXPORT QVector {
public:
    QVector() {}
    void push(T value) {
        m_data[m_size] = value;
        m_size++;
    }
    T& at(int pos) {
        return m_data[pos];
    }
    int count() const {
        return m_size;
    }

    operator SomethingElse<T>() {
        return SomethingElse<T>();
    }

private:
    T m_data[32];
    int m_size = 0;
};


#endif //QVECTOR_H
