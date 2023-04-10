use qt_core::{QCoreApplication, QTimer, SignalOfInt};

#[test]
fn timer_quit() {
    QCoreApplication::init(|_app| unsafe {
        let signal = SignalOfInt::new();

        let timer = QTimer::new_0a();
        let c = signal.connect(timer.slot_start());
        assert!(c.is_valid());

        signal.emit(100);
        assert_eq!(timer.interval(), 100);
        signal.emit(200);
        assert_eq!(timer.interval(), 200);
        0
    })
}
