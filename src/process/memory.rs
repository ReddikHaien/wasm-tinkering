use std::{cell::{OnceCell, UnsafeCell}, ops::Deref, sync::RwLock, alloc::Layout, slice, any};

use anyhow::{Error, anyhow,};
use wasmtime::{MemoryCreator, SharedMemory, Engine, LinearMemory, MemoryType};

pub struct UnsafeSharedMemoryCreator(RwLock<Option<Engine>>);


impl UnsafeSharedMemoryCreator{
    pub fn new() -> Self{
        Self(RwLock::default())
    }

    pub fn set_engine(&self, engine: Engine) -> Result<(),Error>{
        let mut wlock = self.0.write().map_err(|x| anyhow!("{}",x))?;

        match wlock.deref(){
            Some(_) => Err(anyhow!("Engine already added")),
            None => {
                *wlock = Some(engine);
                Ok(())
            },
        }
    }
}

unsafe impl MemoryCreator for UnsafeSharedMemoryCreator{
    fn new_memory(
        &self,
        ty: wasmtime::MemoryType,
        minimum: usize,
        maximum: Option<usize>,
        reserved_size_in_bytes: Option<usize>,
        guard_size_in_bytes: usize,
    ) -> anyhow::Result<Box<dyn wasmtime::LinearMemory>, String> {

        let rlock = self.0.read().map_err(|x| format!("{}",x))?;
        let reference = rlock.as_ref().ok_or("Missing Engine")?;

        let banana = MemoryType::shared(ty.minimum() as u32, ty.maximum().unwrap_or(65536*65536) as u32);

        Ok(Box::new(SharedLinearMemory::new(minimum, maximum, reserved_size_in_bytes, guard_size_in_bytes)))
    }


}

struct SharedLinearMemory{
    base: RwLock<Box<[u8]>>,
    minimum: usize,
    maximum: Option<usize>,
    num_bytes: usize,
    reservation: Option<usize>,
    guard: usize
}

impl SharedLinearMemory{
    pub fn new(minimum: usize, maximum: Option<usize>, reserved: Option<usize>, guard: usize) -> Self{
        let start_allocation = if let Some(reserved) = reserved{
            reserved
        }
        else{
            minimum*65536
        };

        let base = unsafe { Self::alloc_array(start_allocation, guard) };
        Self{
            base: RwLock::new(base),
            guard,
            maximum,
            minimum,
            num_bytes: 0,
            reservation: reserved
        }
    }

    unsafe fn alloc_array(bytes: usize, guard: usize) -> Box<[u8]>{
        let ptr = std::alloc::alloc_zeroed(Layout::from_size_align(bytes + guard, 16).unwrap());
        let bytes = bytes + guard;

        Vec::from_raw_parts(ptr, bytes, bytes).into_boxed_slice()
    }
}

unsafe impl LinearMemory for SharedLinearMemory{
    fn byte_size(&self) -> usize {
        self.num_bytes
    }

    fn maximum_byte_size(&self) -> Option<usize> {
        match (self.reservation,self.maximum){
            (Some(reservation),Some(max)) => Some(reservation.min(max*65536)),
            (_, Some(max)) => Some(max*65536),
            (Some(reservation),_) => Some(reservation),
            _ => None
        }
    }

    fn grow_to(&mut self, new_size: usize) -> anyhow::Result<()> {
        if let Some(max) = self.maximum_byte_size(){
            if new_size >= max{
                return ;
            }
            Err(anyhow!("New size is outside max size"));
        }
    }

    fn as_ptr(&self) -> *mut u8 {
        self.shared.data().as_ptr() as *mut u8
    }
}