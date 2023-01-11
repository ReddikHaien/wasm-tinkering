use core::str;
use std::{sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}}, mem::MaybeUninit, cell::{RefCell, OnceCell}, collections::BTreeMap, thread::ThreadId};

use anyhow::Error;
use tokio::{task::JoinHandle, task_local, sync::RwLock};
use wasmtime::{Store, Engine, Module, Linker, Config, Caller, TypedFunc, Instance};
use wasmtime_wasi::{tokio::WasiCtxBuilder, WasiCtx};



pub struct Process(InnerProcess);

struct InnerProcess{
    main_handle: JoinHandle<Result<(), Error>>
}
#[derive(Clone)]
struct ThreadData{
    engine: Engine,
    module: Module,
    linker: Arc<Linker<WasiCtx>>,
    instance: Instance,
    thread_id: u64,
    thread_name: String,
}

static THREAD_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

const THREAD_DATA_KEY:u32 = 8192;

impl Process{
    pub fn new(module: &str) -> Result<Self, Error>{
        let mut config = Config::new();
        config.async_support(true);
        config.consume_fuel(true);
        
        let engine = Engine::new(&config)?;


        let module = Module::from_file(&engine, module)?;
        let mut linker = Linker::new(&engine);

        wasmtime_wasi::tokio::add_to_linker(&mut linker, |cx| cx)?;

        linker.func_wrap("env", "spawn_thread", move |mut caller: Caller<WasiCtx>,f_ptr: i32|{
            let thread_data: &ThreadData = caller.data_mut().table().get(THREAD_DATA_KEY).unwrap();
            let engine = thread_data.engine.clone();
            let module = thread_data.module.clone();
            let linker = thread_data.linker.clone();
            let handler : JoinHandle<Result<(), Error>>= tokio::task::spawn(async move{
                let thread_id = THREAD_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
                
                let (mut store, instance) = Self::create_environment(&engine, &module, linker.as_ref(), "child").await?;

                store.data_mut().table().insert_at(THREAD_DATA_KEY, Box::new(ThreadData{
                    engine,
                    instance,
                    linker,
                    module,
                    thread_id,
                    thread_name: "child".to_owned()
                }));

                

                Ok(())
            });
            
            0i64
        })?;

        let main_handle = tokio::task::spawn(async move{

            let thread_id = THREAD_ID_COUNTER.fetch_add(1, Ordering::SeqCst);



            //setup environment
            let (mut store, instance) = {
                Self::create_environment(&engine, &module, &linker, "main").await?
            };

            store.data_mut().table.insert_at(THREAD_DATA_KEY,Box::new(ThreadData{
                engine,
                module,
                linker: Arc::new(linker),
                thread_id,
                thread_name: "main".to_owned(),
                instance
            }));

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

    async fn run_async_with(mut store: Store<WasiCtx>, instance: Instance, method: i32) -> Result<(), Error>{
        instance
        .get_typed_func::<(i32,),()>(&mut store, "__thread_entry_point")?
        .call_async(&mut store, (method,))
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
