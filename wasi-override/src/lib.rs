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

    ///
    /// Puts the thread to sleep for the given duration
    pub(crate) fn sleep_thread(seconds: i64, nanos: i32);

    ///
    /// Blocks the caller thread and waits for the other thread to finish
    pub(crate) fn join_thread(thread_id: u64) -> *const c_void;

}

#[no_mangle]
#[doc(hidden)]
pub extern "C" fn __thread_entry_point(entry_ptr: *mut c_void) -> *const c_void{

    println!("received {:?}",entry_ptr);

    let closure: Box<Box<dyn FnOnce() -> *const c_void>> = unsafe { std::mem::transmute(entry_ptr) };

    unsafe{
        let inner = std::mem::transmute_copy::<_,(usize,usize)>(closure.as_ref());
        let outer = std::mem::transmute_copy::<_,usize>(&closure);
        println!("Received closure. ptr: {}, content: {:?}",outer,inner);
    }
    
    closure()
}

#[macro_export]
macro_rules! main_entry {
    ($method:ident) => {
        #[no_mangle]
        #[doc(hidden)]
        pub extern "C" fn __process_entry_point(){
            $method();
        }
    };
}