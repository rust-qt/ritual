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
