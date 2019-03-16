use qt_core::Signal;
use qt_core::QCoreApplication;
use qt_core::QVariant;
use qt_core::QVariantAnimation;
use qt_core::RawSlotVariantRef;
use std::ffi::c_void;

extern "C" fn value_changed(_data: *mut c_void, value: *const Variant) {
    let value = unsafe { value.as_ref() }.expect("value must not be null");
    println!("value_changed: {}", value.to_string().to_std_string());
}

#[test]
fn variant_animation() {
    QCoreApplication::create_and_exit(|app| {
        let mut slot1 = RawSlotVariantRef::new();
        unsafe {
            slot1.set(value_changed, std::ptr::null_mut());
        }

        let mut animation = QVariantAnimation::new();
        animation.value_changed().connect(&slot1);
        animation.finished().connect(&app.quit());
        animation.set_start_value(&QVariant::new0(1));
        animation.set_end_value(&QVariant::new0(5));
        animation.set_duration(5000);
        animation.start(());
        QCoreApplication::exec()
    })
}
