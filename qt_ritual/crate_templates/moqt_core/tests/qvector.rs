use moqt_core::{QVectorOfCInt, QVectorOfBasicClassField, BasicClassField};

#[test]
fn qvector_int() {
    unsafe {
        let mut vec = QVectorOfCInt::new();
        vec.push(10);
        vec.push(12);
        vec.push(14);
        vec.push(16);
        assert_eq!(vec.count(), 4);
        assert_eq!(vec.at(2), &mut 14);
    }
}

#[test]
fn qvector_class() {
    unsafe {
        let mut vec = QVectorOfBasicClassField::new();
        let mut f = BasicClassField::new();
        f.set(21);
        vec.push(&f);
        let mut f = BasicClassField::new();
        f.set(24);
        vec.push(&f);
        assert_eq!(vec.count(), 2);
        assert_eq!(vec.at(1).get(), 24);
    }
}
