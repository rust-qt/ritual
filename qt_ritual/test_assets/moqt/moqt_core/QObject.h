#ifndef QOBJECT_H
#define QOBJECT_H

#include "moqt_core_exports.h"

#define Q_SIGNALS public
#define Q_SLOTS

class QMetaObject {
public:
    class Connection {
    public:
    };
};

#define Q_OBJECT

class MOQT_CORE_EXPORT QObject {
public:
    QObject(QObject* parent = nullptr);
    virtual ~QObject();

    static QMetaObject::Connection connect(
        const QObject* sender, const char* signal,
        const QObject* receiver, const char* method);

    class ConnectArgs {
    public:
        const QObject* sender;
        const char* signal;
        const QObject* receiver;
        const char* method;
    };

    static ConnectArgs nextConnectArgs();

private:
    struct QPrivateSignal {};

Q_SIGNALS:
    void destroyed(QObject *objectName = nullptr);
    void objectNameChanged(int objectName, QPrivateSignal);

public Q_SLOTS:
    void deleteLater();
};

#endif //QOBJECT_H
