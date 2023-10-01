use std::{fs::File, io::Read, cell::OnceCell, sync::OnceLock, future::Future};

use futures::future::FutureExt;
use dashmap::DashMap;
use noak::reader::cpool::Item;
use peg::parser;
use tokio::{task::JoinHandle, pin};
use zip::ZipArchive;

pub mod work;
pub mod data;

macro_rules! run {
    ($it:ident => $f:ident) => {
        {
            let mut handles = Vec::new();
            for i in $it{
                handles.push(
                    tokio::spawn(async move {
                        $f(i).await
                    })
                );
                
            }
            for h in handles{
                h.await??;
            }
        }   
    };
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error>{
    static class_data: OnceLock<DashMap<Box<[u8]>, Vec<u8>>> = OnceLock::new();
    class_data.get_or_init(|| DashMap::new());

    let mut names = Vec::new();
    for file in ["./jagexappletviewer.jar"]{
        let mut archive = ZipArchive::new(File::open(file)?)?;
        
        for i in 0..archive.len(){
            let mut file = archive.by_index(i)?;
            let name = file.name();
            if name.ends_with(".class") {
                let mut bytes = Vec::new();
                let name = name[..name.len()-"class".len()].bytes().collect::<Vec<_>>().into_boxed_slice();
                file.read_to_end(&mut bytes)?;
                class_data.get().unwrap().insert(name.clone(), bytes);
                names.push(name);
            }
        }
    }

    static PARSED_CLASSES: OnceLock<DashMap<Box<[u8]>, Vec<u8>>> = OnceLock::new();
    class_data.get_or_init(|| DashMap::new());
    async fn parse_class(name: Box<[u8]>) -> anyhow::Result<()>{
        if PARSED_CLASSES.get().unwrap().contains_key(&name){
            return Ok(());
        }

        if let Some(bytes) = class_data.get().unwrap().get(&name){
            let mut class = noak::reader::Class::new(&bytes.value())?;

            let pool = class.pool()?;

            for item in pool.iter(){
                match item {
                    Item::Class(c) => {
                        let name = pool.get(c.name)?;
                        let name = Vec::from_iter(name.content.as_bytes().iter().cloned()).into_boxed_slice();
                        spawn_parse_class(name).await??;
                    }
                    Item::FieldRef(f) => {
                        let reference = pool.get(f.name_and_type)?;
                        let descriptor = pool.get(reference.descriptor)?;
                        let type_ = descriptor_parser::field(descriptor.content.as_bytes())?;
                        match type_ {
                            JavaType::Reference(_, n) => {
                                spawn_parse_class(n).await??
                            },
                            _ => ()
                        }
                    },
                    Item::MethodRef(m) => {
                        let reference = pool.get(m.name_and_type)?;
                        let descriptor = pool.get(reference.descriptor)?;
                        let types = descriptor_parser::class_names(descriptor.content.as_bytes())?;
                        for type_ in types{
                            if let JavaType::Reference(_,n) = type_ {
                                spawn_parse_class(n).await??;
                            }
                        }
                    },
                    Item::InterfaceMethodRef(_) => todo!(),
                    Item::String(_) => todo!(),
                    Item::Integer(_) => todo!(),
                    Item::Long(_) => todo!(),
                    Item::Float(_) => todo!(),
                    Item::Double(_) => todo!(),
                    Item::NameAndType(_) => todo!(),
                    Item::Utf8(_) => todo!(),
                    Item::MethodHandle(_) => todo!(),
                    Item::MethodType(_) => todo!(),
                    Item::Dynamic(_) => todo!(),
                    Item::InvokeDynamic(_) => todo!(),
                    Item::Module(_) => todo!(),
                    Item::Package(_) => todo!(),
                }
            }
        }
        Ok(())
    }
    fn spawn_parse_class(name: Box<[u8]>) -> JoinHandle<anyhow::Result<()>>{
        tokio::spawn(async move{
            parse_class(name).await
        }.boxed())
    }
    run!(names => parse_class);
    
    Ok(())
}


parser!(
    grammar descriptor_parser() for [u8]{
        rule primitive(c: usize) -> JavaType
        = "B" {JavaType::Byte(c as u8)}
        / "C" {JavaType::Char(c as u8)}
        / "D" {JavaType::Double(c as u8)}
        / "F" {JavaType::Float(c as u8)}
        / "I" {JavaType::Int(c as u8)}
        / "J" {JavaType::Long(c as u8)}
        / "L" n:([^ 59]+) ";" {JavaType::Reference(c as u8, n.into_boxed_slice())}
        / "S" {JavaType::Short(c as u8)}
        / "Z" {JavaType::Bool(c as u8)}

        pub rule field() -> JavaType
        = n:$("["*) p:primitive({n.len()}) {p}

        pub rule method() -> (Vec<JavaType>, JavaType)
        = "(" x:field()* ")" r:field() {(x, r)}

        pub rule class_names() -> Vec<JavaType>
         = &"(" x: method() {let mut y = x.0; y.push(x.1); y}
         / x:field() {vec![x]}
    }
);



enum JavaType{
    Bool(u8),
    Byte(u8),
    Char(u8),
    Short(u8),
    Int(u8),
    Float(u8),
    Double(u8),
    Long(u8), 
    Reference(u8, Box<[u8]>)
}
