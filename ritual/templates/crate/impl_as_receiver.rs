{condition_attribute}
impl {qt_core}::AsReceiver for {type_path} {{
    type Arguments = {args};
    fn as_receiver(&self) -> {qt_core}::Receiver<Self::Arguments> {{
        unsafe {{
            {qt_core}::Receiver::new(
                ::cpp_core::Ref::from_raw(self).expect("attempted to construct a null Ref"),
                ::std::ffi::CStr::from_bytes_with_nul_unchecked(b"{receiver_id}\0")
            )
        }}
    }}
}}
