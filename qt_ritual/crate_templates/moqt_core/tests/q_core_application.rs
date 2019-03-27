use cpp_utils::{ConstPtr, Ptr};
use moqt_core::{QCoreApplication, QCoreApplicationArgs, QString, SlotOfQStringConstPtr};

#[test]
fn q_core_application() {
    unsafe {
        let mut args = QCoreApplicationArgs::empty();
        let (argc, argv) = args.get();
        let obj1 = QCoreApplication::new_2a(Ptr::new(argc), Ptr::new(argv));

        let slot = SlotOfQStringConstPtr::new(|_name: ConstPtr<QString>| ());
        obj1.app_name_changed().connect(&slot);
    }
}
