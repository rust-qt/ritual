#include "basic_class.h"

BasicClass::BasicClass(int x) : public_int_field(1), public_int_reference_field(public_int_field) {
    m_foo = x;
    public_int_pointer_field = 0;
}

int BasicClass::foo() {
    return m_foo;
}

void BasicClass::setFoo(int foo) {
    m_foo = foo;
}


void BasicClass::overloaded_normal_const_and_static() {}

void BasicClass::overloaded_normal_const_and_static() const {}

void BasicClass::overloaded_normal_const_and_static(int) {}

void BasicClass::overloaded_normal_and_const() {}

void BasicClass::overloaded_normal_and_const() const {}

void BasicClass::overloaded_const_and_static() const {}

void BasicClass::overloaded_const_and_static(int) {}
