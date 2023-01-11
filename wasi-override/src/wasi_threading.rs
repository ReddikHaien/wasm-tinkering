use std::{ffi::c_void, marker::PhantomData, time::Duration};

use crate::{spawn_thread, sleep_thread, spawn_handler};

pub fn sleep(duration: Duration){
    let secs = duration.as_secs();
    let micros = duration.subsec_micros();
    unsafe{
        sleep_thread(secs as i64, micros as i32)
    }
}

pub fn spawn<T, F>(f: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static
{
    let cb = ||{
        let value = f();
        let boxed = Box::new(value);
        let ptr = Box::into_raw(boxed);
        ptr as *const c_void
    };

    let cb_ptr = Box::new(Box::new(cb));

    let thread_id = unsafe {
        spawn_thread(spawn_handler, Box::into_raw(cb_ptr) as *mut c_void) as u64
    };

    let thread = Thread{
        id: thread_id
    };

    let handle = JoinHandle{
        thread,
        marker: PhantomData
    };

    handle
}



pub struct ThreadId(u64);

pub struct Thread{
    id: u64,
}

impl Thread{
    pub fn id(&self) -> ThreadId{
        todo!("Thread::id")
    }
    pub fn name(&self) -> Option<&str>{
        todo!("Thread::name")
    }
}

pub struct JoinHandle<T>{
    thread: Thread,
    marker: PhantomData<T>
}

impl<T> JoinHandle<T>{
    pub fn thread(&self) -> &Thread{
        &self.thread
    }

    pub fn join(&self) -> T{
        todo!("JoinHandle::join")
    }

    pub fn is_finished(&self) -> bool{
        todo!("JoinHandle::is_finished")
    }
}