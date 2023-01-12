mod memory;

use core::str;
use std::{sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}}, mem::MaybeUninit, cell::{RefCell, OnceCell}, collections::BTreeMap, thread::ThreadId};

use anyhow::Error;
use tokio::{task::JoinHandle, task_local, sync::RwLock};
use wasmtime::{Store, Engine, Module, Linker, Config, Caller, TypedFunc, Instance};
use wasmtime_wasi::{tokio::WasiCtxBuilder, WasiCtx};

use self::memory::UnsafeSharedMemoryCreator;



pub struct Process(InnerProcess);

struct InnerProcess{
    main_handle: JoinHandle<Result<(), Error>>
}

struct ThreadData{
    engine: Engine,
    module: Module,
    linker: Arc<Linker<WasiCtx>>,
    instance: Instance,
    thread_id: u64,
    thread_name: String,
    child_threads: Vec<(u64, JoinHandle<Result<i32, Error>>)>
}

impl ThreadData{
    pub fn new(engine: Engine, module: Module, linker: Arc<Linker<WasiCtx>>, instance: Instance, thread_id: u64, thread_name: String) -> Self{
        Self{
            engine,
            module,
            linker,
            instance,
            thread_id,
            thread_name,
            child_threads: Vec::new()
        }
    }
}

static THREAD_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

const THREAD_DATA_KEY:u32 = 8192;



impl Process{
    pub async fn new(module: &str) -> Result<Self, Error>{
        let mut config = Config::new();
        config.async_support(true);
        config.consume_fuel(true);

        let memory_creator = Arc::new(UnsafeSharedMemoryCreator::new());

        config.with_host_memory(memory_creator.clone());

        let engine = Engine::new(&config)?;

        memory_creator.set_engine(engine.clone())?;

        let module = Module::from_file(&engine, module)?;
        let mut linker = Linker::new(&engine);

        wasmtime_wasi::tokio::add_to_linker(&mut linker, |cx| cx)?;

        linker.func_wrap("env", "spawn_thread", move |mut caller: Caller<WasiCtx>,f_ptr: i32|{
            let thread_data: &mut ThreadData = caller.data_mut().table().get_mut(THREAD_DATA_KEY).unwrap();
            let engine = thread_data.engine.clone();
            let module = thread_data.module.clone();
            let linker = thread_data.linker.clone();
            let new_thread_id = THREAD_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

            println!("host received: {}",f_ptr);

            let handler: JoinHandle<Result<i32, Error>> = tokio::task::spawn(async move{

                let (mut store, instance) = Self::create_environment(&engine, &module, linker.as_ref(), "child").await?;
                
                store.data_mut().table().insert_at(THREAD_DATA_KEY, Box::new(ThreadData::new(engine, module, linker, instance, new_thread_id, "child".to_owned())));

                Self::run_async_with(store, instance, f_ptr).await
            });
            
            thread_data.child_threads.push((new_thread_id, handler));

            new_thread_id
        })?;

        
        linker.func_wrap1_async("env", "join_thread", |mut caller: Caller<WasiCtx>, thread_id: i64|{
            let data: &mut ThreadData = caller.data_mut().table.get_mut(THREAD_DATA_KEY).unwrap();

            let child_thread = data.child_threads.iter().enumerate().find_map(|(i,(ti,_))| if *ti == (thread_id as u64) {Some(i)}else{None}).expect("Thread should be owned for it to be joined");
            
            let (_, handle) = data.child_threads.swap_remove(child_thread);

            Box::new(async move {
                handle.await.expect("Thread to run without error")
            })
        })?;      

        let main_handle = tokio::task::spawn(async move{

            let thread_id = THREAD_ID_COUNTER.fetch_add(1, Ordering::SeqCst);



            //setup environment
            let (mut store, instance) = {
                Self::create_environment(&engine, &module, &linker, "main").await?
            };

            store.data_mut().table.insert_at(THREAD_DATA_KEY,Box::new(ThreadData::new(engine, module, Arc::new(linker), instance, thread_id, "main".to_owned())));

            //Start up process
            Self::run_async(store, instance, "__process_entry_point").await
        });

        Ok(Self(InnerProcess{
            main_handle
        }))
    }

    async fn create_environment(engine: &Engine, module: &Module, linker: &Linker<WasiCtx>, thread_name: &str) -> Result<(Store<WasiCtx>, Instance),Error>{
        let wasi = WasiCtxBuilder::new()
        .inherit_stdout()
        .inherit_stderr()
        .env("THREAD_NAME", thread_name)?
        .build();

        let mut store = Store::new(engine, wasi);

        store.out_of_fuel_async_yield(u64::MAX,1000);

        let instance = linker.instantiate_async(&mut store, module).await?;

       Ok((store, instance))
    }

    async fn run_async_with(mut store: Store<WasiCtx>, instance: Instance, method: i32) -> Result<i32, Error>{
        instance
        .get_typed_func::<i32,i32>(&mut store, "__thread_entry_point")?
        .call_async(&mut store, method)
        .await
    }

    async fn run_async(mut store: Store<WasiCtx>, instance: Instance, entry_point: &str) -> Result<(), Error> {
        instance
        .get_typed_func::<(),()>(&mut store, entry_point)?
        .call_async(&mut store, ())
        .await
    }

    pub async fn wait_for_completion(self) -> Result<(), Error> {
        self.0.main_handle.await??;
        Ok(())
    }
}
