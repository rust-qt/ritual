use cpp_utils::cpp_iter;
use moqt_core::{BasicClassField, QVectorOfBasicClassField, QVectorOfInt};
use std::os::raw::c_int;

#[test]
fn qvector_int() {
    unsafe {
        let mut vec = QVectorOfInt::new();
        vec.push(10);
        vec.push(12);
        vec.push(14);
        vec.push(16);
        assert_eq!(vec.count(), 4);
        assert_eq!(*vec.at(2), 14);
    }
}

#[test]
fn qvector_class() {
    unsafe {
        let mut vec = QVectorOfBasicClassField::new();
        let mut f = BasicClassField::new();
        f.set(21);
        vec.push(f.as_ref());
        let mut f = BasicClassField::new();
        f.set(24);
        vec.push(f.as_ref());
        assert_eq!(vec.count(), 2);
        assert_eq!(vec.at(1).get(), 24);
    }
}

#[test]
fn qvector_iter() {
    unsafe {
        let mut vec = QVectorOfInt::new();
        vec.push(10);
        vec.push(12);
        vec.push(14);
        vec.push(16);

        let collected: Vec<c_int> = cpp_iter(vec.begin_mut(), vec.end_mut())
            .map(|mut_ref| *mut_ref)
            .collect();
        assert_eq!(collected, [10, 12, 14, 16]);

        let reversed: Vec<c_int> = cpp_iter(vec.begin_mut(), vec.end_mut())
            .map(|mut_ref| *mut_ref)
            .rev()
            .collect();
        assert_eq!(reversed, [16, 14, 12, 10]);
    }
}
