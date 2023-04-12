QObjectLifetimeChecker::QObjectLifetimeChecker() {}

void QObjectLifetimeChecker::add(QObject* obj) {
    QObject::connect(
        obj, &QObject::destroyed,
        this, &QObjectLifetimeChecker::objectDestroyed,
        Qt::DirectConnection
    );
    m_objects.insert(obj);
}

bool QObjectLifetimeChecker::isAlive(QObject* obj) {
    return m_objects.contains(obj);
}

void QObjectLifetimeChecker::objectDestroyed(QObject* obj) {
    m_objects.remove(obj);
}

QObjectLifetimeChecker* QOBJECT_LIFETIME_CHECKER = new QObjectLifetimeChecker();
