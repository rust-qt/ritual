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

namespace Qt {
    enum ConnectionType {
        AutoConnection,
        DirectConnection,
        QueuedConnection,
        BlockingQueuedConnection,
        UniqueConnection =  0x80
    };
}

class MOQT_CORE_EXPORT QMetaMethod {};

class MOQT_CORE_EXPORT QObject {
public:
    QObject(QObject* parent = nullptr);
    virtual ~QObject();

    static QMetaObject::Connection connect(const QObject *sender, const char *signal,
                                           const QObject *receiver, const char *member, Qt::ConnectionType = Qt::AutoConnection);

    static QMetaObject::Connection connect(const QObject *sender, const QMetaMethod &signal,
                                           const QObject *receiver, const QMetaMethod &method,
                                           Qt::ConnectionType type = Qt::AutoConnection);

    inline QMetaObject::Connection connect(const QObject *sender, const char *signal,
                                           const char *member, Qt::ConnectionType type = Qt::AutoConnection) const {
        return QMetaObject::Connection();
    }

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

    QObject(const QObject& other) = delete;
    QObject &operator=(const QObject& other) = delete;

Q_SIGNALS:
    void destroyed(QObject *objectName = nullptr);
    void objectNameChanged(int objectName, QPrivateSignal);

public Q_SLOTS:
    void deleteLater();
};

#endif //QOBJECT_H
