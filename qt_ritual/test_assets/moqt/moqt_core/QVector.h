#ifndef QVECTOR_H
#define QVECTOR_H

#include "moqt_core_exports.h"
#include <vector>
#include "QString.h"

template<typename T>
class MOQT_CORE_EXPORT SomethingElse {};

template<typename T>
class QVector {
public:
    QVector() {}
    QVector(int size) {
        for(int i = 0; i < size; ++i) {
            append(T());
        }
    }
    void append(const T& value) {
        m_data[m_size] = value;
        m_size++;
    }
    void append(const QVector<T> &l) {}
    T& at(int pos) {
        return m_data[pos];
    }
    int count() const {
        return m_size;
    }

    operator SomethingElse<T>() {
        return SomethingElse<T>();
    }

    class MOQT_CORE_EXPORT Iterator {
    public:
        bool operator==(const Iterator& other) {
            return m_pos == other.m_pos;
        }
        bool operator!=(const Iterator& other) {
            return m_pos != other.m_pos;
        }
        T& operator*() {
            return *m_pos;
        }
        void operator++() {
            m_pos++;
        }
        void operator--() {
            m_pos--;
        }

    private:
        Iterator(T* pos) : m_pos(pos) {}
        T* m_pos;

        friend class QVector<T>;
    };

    Iterator begin() {
        return Iterator(&m_data[0]);
    }
    Iterator end() {
        return Iterator(&m_data[m_size]);
    }

private:
    T m_data[32];
    int m_size = 0;
};

template <typename T>
inline MOQT_CORE_EXPORT QDebug operator<<(QDebug debug, const QVector<T> &vec) {
    return debug;
}


#endif //QVECTOR_H
