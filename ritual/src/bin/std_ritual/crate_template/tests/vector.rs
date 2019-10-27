use cpp_std::cpp_core::{
    vector_ops::{VectorAsMutSlice, VectorAsSlice},
    Ref, SliceAsBeginEnd,
};
use cpp_std::{VectorOfI32, VectorOfI64, VectorOfInt};

#[test]
fn vector_push_pop_slice() {
    unsafe {
        let mut x = VectorOfI32::new();
        assert!(x.empty());
        x.push_back(Ref::from_raw_ref(&1));
        x.push_back(Ref::from_raw_ref(&2));
        x.push_back(Ref::from_raw_ref(&3));
        assert!(!x.empty());

        assert_eq!(x.vector_as_slice(), &[1, 2, 3]);

        assert_eq!(x.back(), 3);
        x.pop_back();
        assert_eq!(x.back(), 2);
        x.pop_back();
        assert_eq!(x.back(), 1);
        x.pop_back();
        assert!(x.empty());
    }
}

#[test]
fn vector_as_mut_slice() {
    unsafe {
        let mut x = VectorOfInt::new();
        x.resize_2a(10, Ref::from_raw_ref(&5));
        assert_eq!(x.size(), 10);

        let slice = x.vector_as_mut_slice();
        assert_eq!(slice, &vec![5; 10][..]);
        slice[1] = 1;
        slice[2] = 777;

        assert_eq!(x.at(0), 5);
        assert_eq!(x.at(1), 1);
        assert_eq!(x.at(2), 777);
        assert_eq!(x.at(3), 5);
    }
}

#[test]
fn vector_from_slice() {
    unsafe {
        let data = &[2, 4, 6, 8][..];
        let x = VectorOfI64::from_2_i64(data.begin_ptr(), data.end_ptr());
        assert_eq!(x.size(), 4);
        assert_eq!(x.at(0), 2);
        assert_eq!(x.at(1), 4);
        assert_eq!(x.at(2), 6);
        assert_eq!(x.at(3), 8);
    }
}
