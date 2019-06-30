use cpp_utils::Ptr;
use qt_core::{QCoreApplication, QVariant, QVariantAnimation, RawSlotOfQVariantConstPtr};
use std::ffi::c_void;

extern "C" fn value_changed(_data: *mut c_void, value: *const QVariant) {
    unsafe {
        let value = value.as_ref().expect("value must not be null");
        println!("value_changed: {}", value.to_string().to_std_string());
    }
}

#[test]
fn variant_animation() {
    QCoreApplication::create_and_exit(|app| unsafe {
        let mut slot1 = RawSlotOfQVariantConstPtr::from_raw();
        slot1.set(Some(value_changed), Ptr::null());

        let mut animation = QVariantAnimation::new_0a();
        animation.value_changed().connect(&slot1);
        animation.finished().connect(app.slot_quit());
        animation.set_start_value(QVariant::new7(1).as_ptr());
        animation.set_end_value(QVariant::new7(5).as_ptr());
        animation.set_duration(5000);
        animation.start_0a();
        QCoreApplication::exec()
    })
}
