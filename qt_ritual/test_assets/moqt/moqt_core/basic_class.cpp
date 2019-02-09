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
