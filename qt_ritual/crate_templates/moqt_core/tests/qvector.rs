use cpp_core::cpp_iter;
use moqt_core::{BasicClassField, QVectorOfBasicClassField, QVectorOfInt};
use std::os::raw::c_int;

#[test]
fn qvector_int() {
    unsafe {
        let vec = QVectorOfInt::new_0a();
        vec.append_int(&10);
        vec.append_int(&12);
        vec.append_int(&14);
        vec.append_int(&16);
        assert_eq!(vec.count(), 4);
        assert_eq!(*vec.at(2), 14);
    }
}

#[test]
fn qvector_class() {
    unsafe {
        let vec = QVectorOfBasicClassField::new_0a();
        let f = BasicClassField::new();
        f.set(21);
        vec.append_basic_class_field(f.as_ref());
        let f = BasicClassField::new();
        f.set(24);
        vec.append_basic_class_field(f.as_ref());
        assert_eq!(vec.count(), 2);
        assert_eq!(vec.at(1).get(), 24);
    }
}

#[test]
fn qvector_iter() {
    unsafe {
        let vec = QVectorOfInt::new_0a();
        vec.append_int(&10);
        vec.append_int(&12);
        vec.append_int(&14);
        vec.append_int(&16);

        let collected: Vec<c_int> = cpp_iter(vec.begin(), vec.end())
            .map(|mut_ref| *mut_ref)
            .collect();
        assert_eq!(collected, [10, 12, 14, 16]);

        let reversed: Vec<c_int> = cpp_iter(vec.begin(), vec.end())
            .map(|mut_ref| *mut_ref)
            .rev()
            .collect();
        assert_eq!(reversed, [16, 14, 12, 10]);

        for x in vec.iter_mut() {
            let _ = *x;
        }
        for x in vec.as_ptr().iter_mut() {
            let _ = *x;
        }
    }
}
