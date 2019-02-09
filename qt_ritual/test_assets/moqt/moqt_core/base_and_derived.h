#include "moqt_core_exports.h"

class MOQT_CORE_EXPORT BaseClass1 {
public:
    BaseClass1() {}
    virtual ~BaseClass1() {}
    virtual int virtualFunction() const { return 42; }
    int baseFunction() {
        m_baseFunctionResult += 1;
        return m_baseFunctionResult;
    }

    int baseConstFunction() const {
        return m_baseFunctionResult;
    }

private:
    int m_baseFunctionResult = 0;
};

class MOQT_CORE_EXPORT DerivedClass1 : public BaseClass1 {
public:
    DerivedClass1() {}
    int virtualFunction() const override { return 43; }
};

class MOQT_CORE_EXPORT DerivedClass2 : public BaseClass1 {
public:
    DerivedClass2() {}
    int virtualFunction() const override { return 44; }
};



class MOQT_CORE_EXPORT AbstractBaseClass1 {
public:
    AbstractBaseClass1() {}
    virtual ~AbstractBaseClass1() {}
    virtual int* virtualFunction() = 0;
};

class MOQT_CORE_EXPORT DerivedClass3 : public AbstractBaseClass1 {
public:
    DerivedClass3() {}
    int* virtualFunction() override { return new int(45); }
};