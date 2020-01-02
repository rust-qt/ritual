use cpp_core::NullPtr;
use qt_core::{QCoreApplication, QVariant, QVariantAnimation, RawSlotOfQVariant};
use std::ffi::c_void;

extern "C" fn value_changed(_data: *mut c_void, value: *const QVariant) {
    unsafe {
        let value = value.as_ref().expect("value must not be null");
        println!("value_changed: {}", value.to_string().to_std_string());
    }
}

#[test]
fn variant_animation() {
    QCoreApplication::init(|app| unsafe {
        let mut slot1 = RawSlotOfQVariant::new();
        slot1.set(Some(value_changed), NullPtr);

        let mut animation = QVariantAnimation::new_0a();
        let c = animation.value_changed().connect(&slot1);
        assert!(c.is_valid());
        let c = animation.finished().connect(app.slot_quit());
        assert!(c.is_valid());
        animation.set_start_value(&QVariant::from_int(1));
        animation.set_end_value(&QVariant::from_int(5));
        animation.set_duration(5000);
        animation.start_0a();
        QCoreApplication::exec()
    })
}
