use cpp_utils::ConstPtr;
use qt_core::QVectorOfInt;

#[test]
fn vector1() {
    unsafe {
        let mut vec = QVectorOfInt::new();
        vec.append(ConstPtr::new(&1));
        vec.append(ConstPtr::new(&2));
        vec.append(ConstPtr::new(&4));
        assert_eq!(vec.count2(), 3);
        assert_eq!(*vec.at(0), 1);
        assert_eq!(*vec.at(1), 2);
        assert_eq!(*vec.at(2), 4);
    }
}
