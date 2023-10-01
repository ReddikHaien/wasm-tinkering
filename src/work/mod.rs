use std::{sync::atomic::AtomicUsize, array};

use tokio::sync::Mutex;

pub type ClassIdentifier = Box<[u8]>;

pub enum Work{
    ParseClass(ClassIdentifier),
}