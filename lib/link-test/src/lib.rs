#![cfg(test)]

use std::os::raw::{c_char, c_int};

type VALUE = usize;

extern "C" {
    fn ruby_setup() -> c_int;
    fn rb_eval_string_protect(_: *const c_char, _: *mut c_int) -> VALUE;
    fn ruby_cleanup(_: c_int) -> c_int;

    static mut rb_cArray: VALUE;
}

#[test]
fn test() {
    let script = "Array\0";
    unsafe {
        assert_eq!(ruby_setup(), 0);

        let mut err: c_int = 0;
        let val = rb_eval_string_protect(script.as_ptr() as _, &mut err);
        assert_eq!(val, rb_cArray);

        if err != 0 {
            panic!("An exception was thrown");
        }

        assert_eq!(ruby_cleanup(0), 0);
    }
}
