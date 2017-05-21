extern crate qt_core;
use qt_core::core_application::CoreApplication;
use qt_core::variant::Variant;
use qt_core::variant_animation::VariantAnimation;
use qt_core::connection::Signal;
use qt_core::slots::SlotVariantRef;

#[test]
fn variant_animation2() {
  CoreApplication::create_and_exit(|app| {
    let slot1 = SlotVariantRef::new(|value| {
                                      println!("value_changed: {}",
                                               value.to_string().to_std_string());
                                    });

    let mut animation = VariantAnimation::new();
    animation.signals().value_changed().connect(&slot1);
    animation
      .signals()
      .finished()
      .connect(&app.slots().quit());
    animation.set_start_value(&Variant::new0(1));
    animation.set_end_value(&Variant::new0(5));
    animation.set_duration(5000);
    animation.start(());
    CoreApplication::exec()
  })
}
