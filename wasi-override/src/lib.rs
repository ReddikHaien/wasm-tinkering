pub mod wasi_threading;

use std::ffi::c_void;

// #[cfg(not(target_arch = "wasm32"))]
// pub use std::thread;
// #[cfg(target_arch = "wasm32")]
pub use wasi_threading as thread;


extern "C" {
    ///
    /// Spawns a new thread with the given entry point
    pub(crate) fn spawn_thread(entry_point: *mut c_void) -> i64;
    pub(crate) fn sleep_thread(seconds: i64, microseconds: i32);
}

#[no_mangle]
pub extern "C" fn __thread_entry_point(entry_ptr: *mut c_void) -> *const c_void{
    let closure: &mut &mut dyn FnMut() -> *const c_void = unsafe { std::mem::transmute(entry_ptr) };
    closure()
}

#[macro_export]
macro_rules! main_entry {
    ($method:ident) => {
        #[no_mangle]
        pub extern "C" fn __process_entry_point(){
            $method();
        }
    };
}