#include "ctrt1/exports.h"

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

private:
  int m_x;
};
