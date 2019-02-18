#ifndef QVECTOR_H
#define QVECTOR_H

#include "moqt_core_exports.h"
#include <vector>

template<typename T>
class MOQT_CORE_EXPORT QVector {
public:
    QVector() {}
    void push(T value) {
        m_data.push_back(value);
    }
    T& at(int pos) {
        return m_data.at(pos);
    }
    int count() const {
        return m_data.size();
    }

private:
    std::vector<T> m_data;
};


#endif //QVECTOR_H
