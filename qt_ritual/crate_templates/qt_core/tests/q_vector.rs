use qt_core::QVectorOfInt;

#[test]
fn vector1() {
    unsafe {
        let vec = QVectorOfInt::new_0a();
        vec.append_int(&1);
        vec.append_int(&2);
        vec.append_int(&4);
        assert_eq!(vec.count_0a(), 3);
        assert_eq!(*vec.at(0), 1);
        assert_eq!(*vec.at(1), 2);
        assert_eq!(*vec.at(2), 4);
    }
}
