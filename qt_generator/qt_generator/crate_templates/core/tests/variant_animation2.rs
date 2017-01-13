extern crate qt_core;
use qt_core::core_application::CoreApplication;
use qt_core::variant::Variant;
use qt_core::variant_animation::VariantAnimation;
use qt_core::cpp_utils::*;
use qt_core::connections::Signal;
use qt_core::slots::SlotVariantVariantRef;

#[test]
fn variant_animation2() {
  CoreApplication::create_and_exit(|app| {
    let slot1 = SlotVariantVariantRef::new(|value| {
      println!("value_changed: {}",
               value.to_string(AsStruct).to_std_string());
    });

    let mut animation = VariantAnimation::new(AsBox);
    animation.signals().value_changed().connect(&slot1);
    animation.signals().finished().connect(&app.slots().quit());
    animation.set_start_value(&Variant::new((1, AsStruct)));
    animation.set_end_value(&Variant::new((5, AsStruct)));
    animation.set_duration(5000);
    animation.start(());
    CoreApplication::exec()
  })
}
