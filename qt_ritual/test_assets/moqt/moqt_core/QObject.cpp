#include "QObject.h"
#include <mutex>
#include <string>
#include <deque>
#include <cstring>

thread_local std::mutex connectArgsMutex;
thread_local std::deque<QObject::ConnectArgs> connectArgs;

const char* copyToHeap(const char* string) {
    auto buf = new char[strlen(string) + 1];
    return strcpy(buf, string);
}

QObject::QObject(QObject* parent) {

}

QObject::~QObject() {

}

void QObject::destroyed(QObject *objectName) {

}

void QObject::objectNameChanged(int objectName, QPrivateSignal) {

}

void QObject::deleteLater() {

}

QMetaObject::Connection QObject::connect(const QObject *sender, const char *signal,
                                       const QObject *receiver, const char *member, Qt::ConnectionType) {
    std::lock_guard<std::mutex> lock(connectArgsMutex);
    ConnectArgs args;
    args.sender = sender;
    args.signal = copyToHeap(signal);
    args.receiver = receiver;
    args.method = copyToHeap(member);
    connectArgs.push_back(args);

    return QMetaObject::Connection();
}

QMetaObject::Connection QObject::connect(const QObject *sender, const QMetaMethod &signal,
                                       const QObject *receiver, const QMetaMethod &method,
                                       Qt::ConnectionType type) {
    return QMetaObject::Connection();
}

QObject::ConnectArgs QObject::nextConnectArgs() {
    std::lock_guard<std::mutex> lock(connectArgsMutex);
    if (connectArgs.empty()) {
        printf("nextConnectArgs: no data\n");
        exit(1);
    }
    auto result = connectArgs.front();
    connectArgs.pop_front();
    return result;
}
