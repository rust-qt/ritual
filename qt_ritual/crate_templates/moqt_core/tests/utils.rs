use moqt_core::{moqt_abs, moqt_core_version};
use std::ffi::CStr;

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
        let str = CStr::from_ptr(r).to_str().unwrap();
        assert_eq!(str, "0.0.1");
    }
}
