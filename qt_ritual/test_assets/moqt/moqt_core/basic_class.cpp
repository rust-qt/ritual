#include "basic_class.h"

BasicClass::BasicClass(int x) : int_field(1), intReference_field(int_field) {
    m_foo = x;
    intPointerField = nullptr;
}

int BasicClass::foo() {
    return m_foo;
}

void BasicClass::setFoo(int foo) {
    m_foo = foo;
}


void BasicClass::updateFoo(UpdateTypes updateTypes) {
    if (updateTypes & Add2) {
        m_foo += 2;
    }
    if (updateTypes & Mul3) {
        m_foo *= 3;
    }
    if (updateTypes & Div5) {
        m_foo /= 5;
    }
}

QVector<int> BasicClass::get_vector_int() const {
    auto r = QVector<int>();
    r.append(1);
    r.append(3);
    r.append(5);
    return r;
}

QVector<BasicClassField> BasicClass::get_vector_class() const {
    auto r = QVector<BasicClassField>();
    BasicClassField value;
    value.set(2);
    r.append(value);
    value.set(4);
    r.append(value);
    value.set(6);
    r.append(value);
    return r;
}

BasicClass::operator int() {
    return 3;
}
BasicClass::operator QVector<int>() {
    auto r = QVector<int>();
    r.append(7);
    return r;
}

QFlags<BasicClass::UpdateType> operator|(BasicClass::UpdateType f1, BasicClass::UpdateType f2) {
    return QFlags<BasicClass::UpdateType>(f1 | f2);
}

void BasicClass::setRef(const int& value) {
}
