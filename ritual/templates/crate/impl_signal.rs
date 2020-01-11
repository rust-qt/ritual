impl {type_path} {{
    /// Connects this signal to another signal or slot.
    ///
    /// This is a shortcut for `self.signal().connect(receiver)`.
    {condition_attribute}
    pub unsafe fn connect<R>(&self, receiver: R)
        -> cpp_core::CppBox<{qt_core}::q_meta_object::Connection>
    where
        R: {qt_core}::AsReceiver,
        {args}: {qt_core}::ArgumentsCompatible<R::Arguments>,
    {{
        self.signal().connect(receiver)
    }}
}}
