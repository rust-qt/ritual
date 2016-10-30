#include "ctrt1/class1.h"

Class1::Class1(int x) {
  m_x = x;
}

int Class1::x() {
  return m_x;
}

void Class1::f1() {}
void Class1::f1() const {}
void Class1::f1(int) {}

void Class1::f2() {}
void Class1::f2() const {}

void Class1::f3() const {}
void Class1::f3(int) {}

void Class1::f4() {}
void Class1::f4(int) {}
