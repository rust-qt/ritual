use moqt_core::{HandleFactory, Handle, BaseHandle, DerivedHandle, DerivedHandle2};
use cpp_utils::{CppBox, CppDeletable};

#[test]
fn basic_class() {
    unsafe {
        let mut factory = HandleFactory::new();
        assert_eq!(factory.counter(), 0);

        let h1 = factory.create().to_box();
        let mut h2 = factory.create();
        assert_eq!(factory.counter(), 2);
        drop(h1);
        assert_eq!(factory.counter(), 1);
        h2.delete();
        assert_eq!(factory.counter(), 0);
    }
}
