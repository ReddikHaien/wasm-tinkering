pub mod wasi_threading;

use std::ffi::c_void;

// #[cfg(not(target_arch = "wasm32"))]
// pub use std::thread;
// #[cfg(target_arch = "wasm32")]
pub use wasi_threading as thread;


extern "C" {
    pub(crate) fn spawn_thread(entry_point: extern "C" fn(arg: *mut c_void) -> *const c_void, arg: *mut c_void) -> i64;
    pub(crate) fn sleep_thread(seconds: i64, microseconds: i32);
}