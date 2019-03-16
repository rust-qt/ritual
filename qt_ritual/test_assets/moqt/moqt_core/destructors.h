#include "moqt_core_exports.h"

class Handle;
class BaseHandle;
class DerivedHandle;
class DerivedHandle2;

class MOQT_CORE_EXPORT HandleFactory {
public:
    Handle* create();
    BaseHandle* createBase();
    DerivedHandle* createDerived();
    DerivedHandle2* createDerived2();
    int counter() { return m_counter; }

private:
    int m_counter = 0;
    friend class Handle;
    friend class BaseHandle;
    friend class DerivedHandle;
    friend class DerivedHandle2;
};

class MOQT_CORE_EXPORT Handle {
public:
    Handle(HandleFactory* factory) : m_factory(factory) {
        m_factory->m_counter++;
    }
    ~Handle() {
        m_factory->m_counter--;
    }

private:
    HandleFactory* m_factory;
};

class MOQT_CORE_EXPORT BaseHandle {
public:
    BaseHandle(HandleFactory* factory) : m_factory(factory) {
        m_factory->m_counter++;
    }
    virtual ~BaseHandle() {
        m_factory->m_counter--;
    }

protected:
    HandleFactory* m_factory;
};

class MOQT_CORE_EXPORT DerivedHandle : public BaseHandle {
public:
    DerivedHandle(HandleFactory* factory) : BaseHandle(factory) {
        m_factory->m_counter++;
    }
    ~DerivedHandle() {
        m_factory->m_counter--;
    }
};

class MOQT_CORE_EXPORT DerivedHandle2 : public BaseHandle {
public:
    DerivedHandle2(HandleFactory* factory) : BaseHandle(factory) {
        m_factory->m_counter += 2;
    }
    ~DerivedHandle2() {
        m_factory->m_counter -= 2;
    }
};

class MOQT_CORE_EXPORT DestructorLess {
public:
    DestructorLess();
private:
    ~DestructorLess();

};
