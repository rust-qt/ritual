#include "QObject.h"

QObject::QObject(QObject* parent) {

}
QObject::~QObject() {

}

void QObject::destroyed(QObject *objectName) {

}

void QObject::objectNameChanged(const std::string &objectName, QPrivateSignal) {

}

void QObject::deleteLater() {

}

QMetaObject::Connection QObject::connect(
    const QObject* sender, const char* signal,
    const QObject* receiver, const char* method)
{
    return QMetaObject::Connection();
}
