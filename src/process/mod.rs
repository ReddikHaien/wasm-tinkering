use core::str;
use std::{sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}}, mem::MaybeUninit, cell::{RefCell, OnceCell}, collections::BTreeMap, thread::ThreadId};

use anyhow::Error;
use tokio::{task::JoinHandle, task_local, sync::RwLock};
use wasmtime::{Store, Engine, Module, Linker, Config, Caller, TypedFunc, Instance};
use wasmtime_wasi::{tokio::WasiCtxBuilder, WasiCtx};


pub struct Process(InnerProcess);

struct InnerProcess{
    main_handle: JoinHandle<()>
}

struct ThreadData{
    engine: Engine,
    module: Module,
    linker: Arc<Linker<WasiCtx>>
}

static DATA: RwLock<BTreeMap<u64,ThreadData>> = RwLock::const_new(BTreeMap::new());
static THREAD_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl Process{
    pub fn new(module: &str) -> Result<Self, Error>{
        let mut config = Config::new();
        config.async_support(true);
        config.consume_fuel(true);
        
        let engine = Engine::new(&config)?;


        let module = Module::from_file(&engine, module)?;
        let mut linker = Linker::new(&engine);

        wasmtime_wasi::tokio::add_to_linker(&mut linker, |cx| cx)?;

        linker.func_wrap("env", "spawn_thread", move |x: i32|{
            0
        })?;

        let main_handle = tokio::task::spawn(async move{

            let thread_id = THREAD_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

            //Move the processdata into the thread
            {
                let mut write_lock = DATA.write().await;
                write_lock.insert(thread_id, ThreadData{
                    engine,
                    module,
                    linker: Arc::new(linker),
                });
            }

            //Fire up the process
            let (store, instance) = {
                let read_lock = DATA.read().await;
                let env = &read_lock[&thread_id];
                Self::create_environment(env, "main").expect("Create Store and Instance")
            };
            
            Self::run_async(store, instance, "_start").await.unwrap();
        });

        Ok(Self(InnerProcess{
            main_handle
        }))
    }

    fn create_environment(env: &ThreadData, thread_name: &str) -> Result<(Store<WasiCtx>, Instance),Error>{
        let wasi = WasiCtxBuilder::new()
        .inherit_stdout()
        .inherit_stderr()
        .env("THREAD_NAME", thread_name)?
        .build();

        let mut store = Store::new(&env.engine, wasi);

        store.out_of_fuel_async_yield(u64::MAX,1000);

        let instance = env.linker.instantiate(&mut store, &env.module)?;

       Ok((store, instance))
    }

    async fn run_async(mut store: Store<WasiCtx>, instance: Instance, entry_point: &str) -> Result<(), Error> {
        instance
        .get_typed_func::<(),()>(&mut store, entry_point)?
        .call_async(&mut store, ())
        .await
    }
}
