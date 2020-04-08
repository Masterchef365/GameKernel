mod ffi {
    extern "C" {
        pub fn debug(buf: *const u8, length: usize);
    }
}

pub fn debug(s: &str) {
    unsafe { ffi::debug(s.as_ptr(), s.len()) }
}
