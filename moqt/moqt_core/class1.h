#include "moqt_core_exports.h"
#include <cstdint>

class MOQT_CORE_EXPORT Class1_F {
public:
  Class1_F() {}
};

class MOQT_CORE_EXPORT Class1 {
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

  void ov5(int a) {}
  void ov5(uint16_t b) {}


private:
  int m_x;
};

namespace ns1 {
    int x() {}

    namespace ns2 {
        int x() {}
        int y() {}

        enum Enum1 {
            Val1,
            Val2,
            Val3
        };
    };

    namespace ns3 {
        int a() {}
        int b() {}

        enum Enum2 {
            Val11 = 1,
            Val12 = 2,
            Val13 = 3,
        };
    };

}

enum impl {
    trait,
    use,
    crate = trait + 1,
    last = -1,
};


class MOQT_CORE_EXPORT BaseClass1 {
public:
    BaseClass1() {}
    virtual ~BaseClass1() {}
    virtual void x() {}
};

class MOQT_CORE_EXPORT DerivedClass1 : public BaseClass1 {
public:
    DerivedClass1() {}
    void x() {}
};

class MOQT_CORE_EXPORT DerivedClass2 : public BaseClass1 {
public:
    DerivedClass2() {}
    void x() {}
};
