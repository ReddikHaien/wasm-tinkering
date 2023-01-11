pub use wasi_override::*;

fn main() {
    println!("Helloo");
    let  handle = thread::spawn(worker);
    handle.join();
    println!("Done :) {}",0);
}

fn worker(){
    println!("Hello from worker!")
}
