#include "moqt_core_exports.h"

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
