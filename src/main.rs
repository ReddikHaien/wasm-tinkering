#![feature(once_cell)]
use anyhow::Error;
use process::Process;

mod process;

#[tokio::main]
async fn main() -> Result<(),Error>{

    let process = Process::new("wasm-test.wasm")?;

    process.wait_for_completion().await
}
