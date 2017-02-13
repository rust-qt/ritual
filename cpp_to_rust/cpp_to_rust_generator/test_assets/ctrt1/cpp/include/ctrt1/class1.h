#include "ctrt1/exports.h"
#include <cstdint>

class CTRT1_EXPORT Class1_F {
public:
  Class1_F() {}
};

class CTRT1_EXPORT Class1 {
public:
  Class1(int x);
  int x();

  void f1();
  void f1() const;
  static void f1(int);

  void f2();
  void f2() const;

  void f3() const;
  static void f3(int);

  void f4();
  static void f4(int);

  int ov1() { return 1; }
  int ov1(int) { return 0; }

  int ov2() { return 1; }
  float ov2(int) { return 0; }

  int field1;
  int* field2;
  int& field3;
  Class1_F field4;

  Class1& ov3(int) { return *this; }
  Class1& ov3(double) { return *this; }

  void ov4(uint32_t a) {}
  void ov4(uint16_t a) {}


private:
  int m_x;
};
