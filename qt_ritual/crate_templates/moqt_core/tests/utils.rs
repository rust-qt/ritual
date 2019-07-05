use moqt_core::{moqt_abs, moqt_core_version};

#[test]
fn abs() {
    unsafe {
        assert_eq!(moqt_abs(1), 1);
        assert_eq!(moqt_abs(0), 0);
        assert_eq!(moqt_abs(-2), 2);
    }
}

#[test]
fn version() {
    unsafe {
        let r = moqt_core_version();
        let str = r.to_c_str().to_str().unwrap();
        assert_eq!(str, "0.0.1");
    }
}
