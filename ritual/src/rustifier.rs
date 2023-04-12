use crate::{
    cpp_parser::Context2,
    crate_writer::{Code, FFI_MODULE, ROOT_MODULE},
};
use ritual_common::errors::{format_err, Result};

use self::rules::Rules;

pub mod rules;

pub fn run(ctx: Context2<'_>) -> Result<Code> {
    let mut rustifier = Rustifier {
        ctx,
        rules: Rules::default(),
        code: Code::default(),
    };
    let hook = rustifier
        .ctx
        .config
        .rustifier_hook()
        .ok_or_else(|| format_err!("missing rustifier_hook"))?;
    hook(&mut rustifier)?;
    Ok(rustifier.code)
}

pub struct Rustifier<'a> {
    ctx: Context2<'a>,
    rules: Rules,
    code: Code,
}

impl Rustifier<'_> {
    pub fn add_cpp_code(&mut self, code: &str) {
        self.code.cpp.push_str(code);
        self.code.cpp.push('\n');
    }

    pub fn add_rust_lib_code(&mut self, code: &str) {
        self.add_rust_code(ROOT_MODULE, code);
    }

    pub fn add_rust_ffi_code(&mut self, code: &str) {
        self.add_rust_code(FFI_MODULE, code);
    }

    fn add_rust_code(&mut self, module: &str, code: &str) {
        let s = self.code.rust.get_mut(module).unwrap();
        s.push_str(code);
        s.push('\n');
    }
}
