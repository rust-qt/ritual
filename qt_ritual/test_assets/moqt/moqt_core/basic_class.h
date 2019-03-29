#include "moqt_core_exports.h"
#include <cstdint>
#include "QFlags.h"
#include <QVector.h>

class MOQT_CORE_EXPORT BasicClassField {
public:
    BasicClassField() {
        r = 42;
    }

    int set(int v) {
        int old = r;
        r = v;
        return old;
    }
    int get() const {
        return r;
    }
private:
    int r;
};

class MOQT_CORE_EXPORT BasicClass {
public:
    BasicClass(int x);

    void setFoo(int foo);
    int foo();

    enum UpdateType {
        Add2 = 0b1,
        Mul3 = 0b10,
        Div5 = 0b100,
    };

    struct InnerStruct {
        enum InnerEnum {
            Something = 42,
        };
        int field;
    };

    typedef QFlags<UpdateType> UpdateTypes;

    void updateFoo(UpdateTypes updateTypes);

    QVector<int> get_vector_int() const;
    QVector<BasicClassField> get_vector_class() const;

    operator int();
    operator QVector<int>();


    /*void overloaded_normal_const_and_static();
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


    BasicClass &overloaded_args_returns_ref(int) { return *this; }

    BasicClass &overloaded_args_returns_ref(double) { return *this; }

    void overloaded_exact_sized_args(uint32_t a) {}
    void overloaded_exact_sized_args(uint16_t a) {}

    void overloaded_platform_dependent(int a) {}
    void overloaded_platform_dependent(uint16_t b) {} */

    int int_field;
    int *intPointerField;
    int &intReference_field;
    BasicClassField class_field;

private:
    int m_foo;
};

QFlags<BasicClass::UpdateType> operator|(BasicClass::UpdateType f1, BasicClass::UpdateType f2);
