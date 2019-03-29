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
    r.push(1);
    r.push(3);
    r.push(5);
    return r;
}

QVector<BasicClassField> BasicClass::get_vector_class() const {
    auto r = QVector<BasicClassField>();
    BasicClassField value;
    value.set(2);
    r.push(value);
    value.set(4);
    r.push(value);
    value.set(6);
    r.push(value);
    return r;
}

BasicClass::operator int() {
    return 3;
}
BasicClass::operator QVector<int>() {
    auto r = QVector<int>();
    r.push(7);
    return r;
}

QFlags<BasicClass::UpdateType> operator|(BasicClass::UpdateType f1, BasicClass::UpdateType f2) {
    return QFlags<BasicClass::UpdateType>(f1 | f2);
}
