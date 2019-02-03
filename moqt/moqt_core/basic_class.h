#include "moqt_core_exports.h"
#include <cstdint>

class MOQT_CORE_EXPORT BasicClassField {
public:
    BasicClassField() {}
};

class MOQT_CORE_EXPORT BasicClass {
public:
    BasicClass(int x);

    void setFoo(int foo);
    int foo();

    void overloaded_normal_const_and_static();
    void overloaded_normal_const_and_static() const;
    static void overloaded_normal_const_and_static(int);

    void overloaded_normal_and_const();
    void overloaded_normal_and_const() const;

    void overloaded_const_and_static() const;
    static void overloaded_const_and_static(int);

    int overloaded_0_and_1_arg() { return 1; }

    int overloaded_0_and_1_arg(int) { return 0; }

    int overloaded_args_and_return_type() { return 1; }

    float overloaded_args_and_return_type(int) { return 0; }

    int public_int_field;
    int *public_int_pointer_field;
    int &public_int_reference_field;
    BasicClassField public_class_field;

    BasicClass &overloaded_args_returns_ref(int) { return *this; }

    BasicClass &overloaded_args_returns_ref(double) { return *this; }

    void overloaded_exact_sized_args(uint32_t a) {}
    void overloaded_exact_sized_args(uint16_t a) {}

    void overloaded_platform_dependent(int a) {}
    void overloaded_platform_dependent(uint16_t b) {}

private:
    int m_foo;
};
