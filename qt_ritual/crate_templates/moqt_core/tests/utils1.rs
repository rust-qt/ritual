use moqt_core::moqt_abs;

#[test]
fn utils1() {
    unsafe {
        assert_eq!(moqt_abs(1), 1);
        assert_eq!(moqt_abs(0), 0);
        assert_eq!(moqt_abs(-2), 2);
    }
}
