#[cfg(target_arch = "wasm32")]
mod ffi {
    extern "C" {
        pub fn debug(buf: *const u8, length: usize);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn debug(s: &str) {
    unsafe { ffi::debug(s.as_ptr(), s.len()) }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn debug(s: &str) {
    println!("Module debug {}", s);
}
