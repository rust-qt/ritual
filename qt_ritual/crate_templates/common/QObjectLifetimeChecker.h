class QObjectLifetimeChecker : public QObject {
    Q_OBJECT
public:
    QObjectLifetimeChecker();
    void add(QObject* obj);
    bool isAlive(QObject* obj);

private slots:
    void objectDestroyed(QObject* obj);

private:
    QSet<QObject*> m_objects;
};
