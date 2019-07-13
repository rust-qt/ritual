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

QPoint QPoint::operator-() const {
    return QPoint(-m_x, -m_y);
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

bool operator==(const QPoint& one, const QPoint& other) {
    return one.x() == other.x() && one.y() == other.y();
}

bool operator==(const char* one, const QPoint& other) {
    return one != 0 && other.x() == other.y();
}

bool operator==(const QPoint& one, int other) {
    return one.x() == other;
}
bool operator!=(const QPoint& one, int other) {
    return one.x() != other;
}
bool operator<(const QPoint& one, int other) {
    return one.x() < other;
}
bool operator<=(const QPoint& one, int other) {
    return one.x() <= other;
}
bool operator>(const QPoint& one, int other) {
    return one.x() > other;
}
bool operator>=(const QPoint& one, int other) {
    return one.x() >= other;
}

bool operator==(const QPoint& one, int64_t other) {
    return false;
}

int operator==(const QPoint& one, float other) {
    return 2;
}
