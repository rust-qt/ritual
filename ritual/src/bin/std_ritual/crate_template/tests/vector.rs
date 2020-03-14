use cpp_std::cpp_core::EndPtr;
use cpp_std::{VectorOfI32, VectorOfI64, VectorOfInt};

#[test]
fn vector_push_pop_slice() {
    unsafe {
        let x = VectorOfI32::new();
        assert!(x.empty());
        x.push_back(&1);
        x.push_back(&2);
        x.push_back(&3);
        assert!(!x.empty());

        assert_eq!(x.as_slice(), &[1, 2, 3]);

        assert_eq!(*x.back(), 3);
        x.pop_back();
        assert_eq!(*x.back(), 2);
        x.pop_back();
        assert_eq!(*x.back(), 1);
        x.pop_back();
        assert!(x.empty());
    }
}

#[test]
fn vector_as_mut_slice() {
    unsafe {
        let x = VectorOfInt::new();
        x.resize_2a(10, &5);
        assert_eq!(x.size(), 10);

        let slice = x.as_mut_slice();
        assert_eq!(slice, &vec![5; 10][..]);
        slice[1] = 1;
        slice[2] = 777;

        assert_eq!(*x.at(0), 5);
        assert_eq!(*x.at(1), 1);
        assert_eq!(*x.at(2), 777);
        assert_eq!(*x.at(3), 5);
    }
}

#[test]
fn vector_from_slice() {
    unsafe {
        let data = &[2, 4, 6, 8][..];
        let x = VectorOfI64::from_2_i64(data.as_ptr(), data.end_ptr());
        assert_eq!(x.size(), 4);
        assert_eq!(*x.at(0), 2);
        assert_eq!(*x.at(1), 4);
        assert_eq!(*x.at(2), 6);
        assert_eq!(*x.at(3), 8);
    }
}
