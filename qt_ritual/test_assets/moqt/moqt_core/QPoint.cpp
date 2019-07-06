#include "QPoint.h"

QPoint QPoint::operator+(const QPoint& other) const {
    return QPoint(
        m_x + other.m_x,
        m_y + other.m_y
    );
}

QPoint& QPoint::operator+=(const QPoint& other) {
    m_x += other.m_x;
    m_y += other.m_y;
    return *this;
}


QPoint QPoint::operator*(const QPoint& other) {
    return QPoint(
        m_x * other.m_x,
        m_y * other.m_y
    );
}

QPoint operator-(const QPoint& one, const QPoint& other) {
    return QPoint(
            one.m_x - other.m_x,
            one.m_y - other.m_y
    );
}
