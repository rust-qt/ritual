use cpp_utils::{CppBox, CppDeletable, Ptr, StaticUpcast};
use moqt_core::{BaseHandle, HandleFactory};

#[test]
fn basic_destructors() {
    unsafe {
        let mut factory = HandleFactory::new();
        assert_eq!(factory.counter(), 0);

        let h1 = CppBox::new(factory.create());
        let mut h2 = factory.create();
        assert_eq!(factory.counter(), 2);
        drop(h1);
        assert_eq!(factory.counter(), 1);
        h2.delete();
        assert_eq!(factory.counter(), 0);
    }
}

#[test]
fn virtual_destructors() {
    unsafe {
        let mut factory = HandleFactory::new();
        assert_eq!(factory.counter(), 0);

        let h1 = CppBox::new(factory.create_derived());
        assert_eq!(factory.counter(), 2);
        let mut h2 = factory.create_derived2();
        assert_eq!(factory.counter(), 5);
        drop(h1);
        assert_eq!(factory.counter(), 3);
        let mut h2_base: Ptr<BaseHandle> = h2.static_upcast_mut();
        h2_base.delete();
        assert_eq!(factory.counter(), 0);

        let h3 = CppBox::new(factory.create_base());
        assert_eq!(factory.counter(), 1);
        drop(h3);
        assert_eq!(factory.counter(), 0);
    }
}
