#ifndef QOBJECT_H
#define QOBJECT_H

#include "moqt_core_exports.h"
#include "QString.h"
#include "QVector.h"

#define Q_SIGNALS public
#define Q_SLOTS

class QMetaObject {
public:
    class Connection {
    public:
        typedef void *Connection::*RestrictedBool;
        operator RestrictedBool() const {
            return (RestrictedBool) &Connection::x;
        }

    private:
        void* x;
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
    QObject *parent() const { return nullptr; }

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

    template<typename T>
    inline T findChild(const QString &aName = QString()) const {
        return nullptr;
    }

    template<typename T>
    inline QVector<T> findChildren(int arg1, int arg2 = 0) const {
        return QVector<T>();
    }

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

template<class T>
class QPointer {
public:
    QPointer() = default;
    QPointer(T *ptr) : m_ptr(ptr) { }
    bool isNull() const { return m_ptr == nullptr; }

private:
    T* m_ptr;
};

#endif //QOBJECT_H
