pub mod code;

use std::{fs::File, error::Error, io::Read, any, ops::Add};

use anyhow::bail;
use noak::reader::{Class, attributes::RawInstruction};
use zip::ZipArchive;

fn main() -> Result<(), anyhow::Error>{
    let mut archive = ZipArchive::new(File::open("./jagexappletviewer.jar")?)?;


    for index in 0..archive.len(){
        let mut file = archive.by_index(index)?;
        if file.name().ends_with(".class"){
            let mut bytes = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut bytes)?;
            
            let mut class = Class::new(&bytes)?;

            for method in class.methods()? {
                let method = method?;

                for attribute in method.attributes(){
                    let attribute = attribute?;
                    if class.pool()?.retrieve(attribute.name())?.as_bytes() == b"Code"{
                        let content = attribute.read_content(class.pool()?)?;
                        match content {
                            noak::reader::AttributeContent::Code(code) => {
                                let mut leaders = vec![0];
                                for exception in code.exception_handlers(){
                                    leaders.push(exception.handler().as_u32());
                                }

                                let mut instructions = Vec::with_capacity(10);
                                
                                for instruction in code.raw_instructions(){
                                    instructions.push(instruction?);
                                }

                                let mut l = 0;
                                for (i, _) in instructions.iter(){
                                    if i.as_u32() >= l{
                                        l = i.as_u32();
                                    }
                                    else {
                                        bail!("Invalid label order in method.")
                                    }
                                 }

                                for (index, instruction) in instructions.iter(){
                                    
                                    match instruction {
                                        noak::reader::attributes::RawInstruction::AReturn |
                                        noak::reader::attributes::RawInstruction::DReturn |
                                        noak::reader::attributes::RawInstruction::IReturn |
                                        noak::reader::attributes::RawInstruction::LReturn |
                                        noak::reader::attributes::RawInstruction::Return |
                                        noak::reader::attributes::RawInstruction::AThrow |
                                        noak::reader::attributes::RawInstruction::FReturn => {
                                            leaders.push(index.as_u32()+1);
                                        },
                                        noak::reader::attributes::RawInstruction::CheckCast {..} => leaders.push(index.as_u32()+2),
                                        
                                        noak::reader::attributes::RawInstruction::GotoW { offset } => {
                                            leaders.push(index.as_u32().wrapping_add_signed(*offset));
                                            leaders.push(index.as_u32().add(4));
                                        },
                                        noak::reader::attributes::RawInstruction::Goto { offset } |
                                        noak::reader::attributes::RawInstruction::IfACmpEq { offset } |
                                        noak::reader::attributes::RawInstruction::IfACmpNe { offset } |
                                        noak::reader::attributes::RawInstruction::IfICmpEq { offset } |
                                        noak::reader::attributes::RawInstruction::IfICmpNe { offset } |
                                        noak::reader::attributes::RawInstruction::IfICmpLt { offset } |
                                        noak::reader::attributes::RawInstruction::IfICmpGe { offset } |
                                        noak::reader::attributes::RawInstruction::IfICmpGt { offset } |
                                        noak::reader::attributes::RawInstruction::IfICmpLe { offset } |
                                        noak::reader::attributes::RawInstruction::IfEq { offset } |
                                        noak::reader::attributes::RawInstruction::IfNe { offset } |
                                        noak::reader::attributes::RawInstruction::IfLt { offset } |
                                        noak::reader::attributes::RawInstruction::IfGe { offset } |
                                        noak::reader::attributes::RawInstruction::IfGt { offset } |
                                        noak::reader::attributes::RawInstruction::IfLe { offset } |
                                        noak::reader::attributes::RawInstruction::IfNonNull { offset } |
                                        noak::reader::attributes::RawInstruction::IfNull { offset } => {
                                            leaders.push(index.as_u32().wrapping_add_signed(*offset as i32));
                                            leaders.push(index.as_u32().add(2));
                                        },
                                        noak::reader::attributes::RawInstruction::JSr { offset } => todo!(),
                                        noak::reader::attributes::RawInstruction::Ret { index } => todo!(),
                                        noak::reader::attributes::RawInstruction::JSrW { offset } => todo!(),
                                        noak::reader::attributes::RawInstruction::RetW { index } => todo!(),
                                        noak::reader::attributes::RawInstruction::LookupSwitch(lookup) => {
                                            leaders.push(index.as_u32().wrapping_add_signed(lookup.default_offset()));
                                            for jump in lookup.pairs(){
                                                leaders.push(index.as_u32().wrapping_add_signed(jump.offset()));
                                            }
                                        },
                                        noak::reader::attributes::RawInstruction::TableSwitch(table) => {
                                            leaders.push(index.as_u32().wrapping_add_signed(table.default_offset()));
                                            for jump in table.pairs(){
                                                leaders.push(index.as_u32().wrapping_add_signed(jump.offset()));
                                            }
                                        },
                                        _ => ()
                                    }
                                }
                                
                                leaders.retain(|x| instructions.binary_search_by_key(x, |y| y.0.as_u32()).is_ok());
                                leaders.sort();
                                leaders.dedup();

                                println!("{:?}",leaders);

                                let mut blocks = Vec::new();

                                for leader in leaders.iter(){
                                    let leader = *leader;
                                    let Ok(start) = instructions.binary_search_by_key(&leader, |x| x.0.as_u32()) else{ bail!("Invalid address {}", leader);};

                                    for (end, (_, i)) in instructions[start..].iter().enumerate(){
                                        if is_jump(i){
                                            blocks.push(&instructions[start..=(start + end)]);
                                            break;
                                        }

                                        if let Some((i, _)) = instructions.get(start + end+1){
                                            if let Ok(_)  = leaders.binary_search(&i.as_u32()){
                                                blocks.push(&instructions[start..(start + end+1)]);
                                            }
                                        } 
                                    }
                                }

                                println!("{:?}", blocks);
                            },
                            x => bail!("Unreachable, expected Code but got {:#?}",x)
                        }
                    }
                }
            }

        }
    }

    Ok(())
}

fn is_jump(instruction: &RawInstruction) -> bool{
    match instruction {
        noak::reader::attributes::RawInstruction::JSr { .. } => todo!(),
        noak::reader::attributes::RawInstruction::Ret { .. } => todo!(),
        noak::reader::attributes::RawInstruction::JSrW { .. } => todo!(),
        noak::reader::attributes::RawInstruction::RetW { .. } => todo!(),
        noak::reader::attributes::RawInstruction::AReturn |
        noak::reader::attributes::RawInstruction::DReturn |
        noak::reader::attributes::RawInstruction::IReturn |
        noak::reader::attributes::RawInstruction::LReturn |
        noak::reader::attributes::RawInstruction::Return |
        noak::reader::attributes::RawInstruction::AThrow |
        noak::reader::attributes::RawInstruction::FReturn |
        noak::reader::attributes::RawInstruction::CheckCast {..} |
        noak::reader::attributes::RawInstruction::GotoW { .. } |
        noak::reader::attributes::RawInstruction::Goto { ..} |
        noak::reader::attributes::RawInstruction::IfACmpEq { .. } |
        noak::reader::attributes::RawInstruction::IfACmpNe { .. }|
        noak::reader::attributes::RawInstruction::IfICmpEq { .. } |
        noak::reader::attributes::RawInstruction::IfICmpNe { .. } |
        noak::reader::attributes::RawInstruction::IfICmpLt { .. } |
        noak::reader::attributes::RawInstruction::IfICmpGe { .. } |
        noak::reader::attributes::RawInstruction::IfICmpGt { .. } |
        noak::reader::attributes::RawInstruction::IfICmpLe { .. } |
        noak::reader::attributes::RawInstruction::IfEq { .. } |
        noak::reader::attributes::RawInstruction::IfNe { .. } |
        noak::reader::attributes::RawInstruction::IfLt { .. } |
        noak::reader::attributes::RawInstruction::IfGe { .. } |
        noak::reader::attributes::RawInstruction::IfGt { .. } |
        noak::reader::attributes::RawInstruction::IfLe { .. } |
        noak::reader::attributes::RawInstruction::IfNonNull { .. } |
        noak::reader::attributes::RawInstruction::IfNull { .. } |
        noak::reader::attributes::RawInstruction::LookupSwitch(_) |
        noak::reader::attributes::RawInstruction::TableSwitch(_) => true,
        _ => false
    }
}