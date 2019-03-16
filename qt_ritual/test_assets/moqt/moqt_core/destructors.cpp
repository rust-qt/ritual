#include "destructors.h"

Handle* HandleFactory::create() {
    return new Handle(this);
}
BaseHandle* HandleFactory::createBase() {
    return new BaseHandle(this);
}
DerivedHandle* HandleFactory::createDerived() {
    return new DerivedHandle(this);
}
DerivedHandle2* HandleFactory::createDerived2() {
    return new DerivedHandle2(this);
}

DestructorLess::DestructorLess() {

}
DestructorLess::~DestructorLess() {

}
