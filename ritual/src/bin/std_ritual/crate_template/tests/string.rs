use cpp_std::cpp_core::SliceAsBeginEnd;
use cpp_std::String;
use std::ffi::CStr;

fn main() {}

#[test]
fn string_push() {
    unsafe {
        let mut s = String::new();
        s.push_back('t' as i8);
        s.push_back('e' as i8);
        s.push_back('s' as i8);
        s.push_back('t' as i8);

        assert_eq!(
            CStr::from_ptr(s.c_str().as_raw_ptr()).to_str().unwrap(),
            "test"
        );
    }
}

#[test]
fn string_from_slice() {
    unsafe {
        let data = "string";
        let s = String::from_char_usize(data.begin_ptr(), data.len());
        assert_eq!(s.length(), 6);

        assert_eq!(
            CStr::from_ptr(s.c_str().as_raw_ptr()).to_str().unwrap(),
            "string"
        );
    }
}
