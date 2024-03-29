use noak::reader::{attributes::{Index, RawInstruction}, cpool::ConstantPool};

const BYTE: char = 'B';
const CHAR: char = 'C';
const INT: char = 'I';
const SHORT: char = 'S';
const BOOL: char = 'Z';

const FLOAT: char = 'F';
const DOUBLE: char = 'D';
const LONG: char = 'J';
const REF: char = 'L';
const ARR: char = '[';

pub struct Stack{
    inputs: Vec<Value>,
    values: Vec<Value>
}

impl Stack{

    pub fn new(inputs: &[Stack]) -> Self{

        let mut s = Self{
            inputs: Vec::new(),
            values: Vec::new()
        };

        if inputs.len() > 0{
            for value in inputs[0].values.iter(){
                s.inputs.push(*value);
                s.values.push(*value);
            }
        }

        for stack in inputs.iter().skip(1){
            assert_eq!(s.inputs.len(), stack.values.len(), "Stack mismatch!");
            for (stack_value, this_value) in stack.values.iter().zip(s.inputs.iter()){
                assert_eq!(this_value, stack_value, "Stack value mismatch!");
            }
        }

        s
    }


    pub fn pop_known(&mut self, expected: Value) -> Value{
        if let Some(value) = self.values.pop(){
            assert_eq!(expected, value, "{:?} is not the same as {:?}!", value, expected);
            value
        }
        else{
            self.inputs.push(expected);
            expected
        }
    }

    pub fn pop_unknown(&mut self) -> Value{
        self.values.pop().expect("Should be a value present in the stack!")
    }

    pub fn push(&mut self, value: Value){
        self.values.push(value)
    }

    pub fn bin_op(&mut self, type_: Value){
        self.pop_known(type_);
        self.pop_known(type_);
        self.push(type_);
    }

    pub fn array_load(&mut self, type_: Value){
        self.pop_known(Value::I32);
        self.pop_known(Value::Ref);
        self.push(type_)
    }

    pub fn array_store(&mut self, type_: Value){
        self.pop_known(type_);
        self.pop_known(Value::I32);
        self.pop_known(Value::Ref);
    }

    pub fn convert(&mut self, in_: Value, out: Value){
        self.pop_known(in_);
        self.push(out);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value{
    I32,
    F32,
    I64,
    F64,
    Ref
}

impl Value{
    pub fn is_big(self) -> bool{
        matches!(self, Self::F64 | Self::I64)
    }

    pub fn is_small(self) -> bool{
        !self.is_big()
    }
}

pub fn generate_stack(block: &[(Index, RawInstruction)], cp: &ConstantPool, inputs: &[Stack]) -> Stack{
    let mut stack = Stack::new(inputs);
    
    for (_, instruction) in block.iter(){
        match instruction {
            RawInstruction::AALoad => stack.array_load(Value::Ref),
            RawInstruction::DALoad => stack.array_load(Value::F64),
            RawInstruction::IALoad |
            RawInstruction::CALoad |
            RawInstruction::BALoad => stack.array_load(Value::I32),
            RawInstruction::FALoad => stack.array_load(Value::F32),
            RawInstruction::LALoad => stack.array_load(Value::I64),
            RawInstruction::AAStore => stack.array_store(Value::Ref),
            RawInstruction::IAStore |
            RawInstruction::CAStore |
            RawInstruction::BAStore => stack.array_store(Value::I32),
            RawInstruction::DAStore => stack.array_store(Value::F64),
            RawInstruction::FAStore => stack.array_store(Value::F32),
            RawInstruction::LAStore => stack.array_store(Value::I64),
            RawInstruction::ALoad { .. } |
            RawInstruction::ALoadW { .. } |
            RawInstruction::ALoad0 |
            RawInstruction::ALoad1 |
            RawInstruction::ALoad2 |
            RawInstruction::ALoad3 |
            RawInstruction::AConstNull => stack.push(Value::Ref),
            RawInstruction::AStore { .. } |
            RawInstruction::AStoreW { .. } |
            RawInstruction::AStore0 |
            RawInstruction::AStore1 |
            RawInstruction::AStore2 |
            RawInstruction::AStore3 |
            RawInstruction::AThrow |
            RawInstruction::AReturn => {stack.pop_known(Value::Ref);}
            RawInstruction::ANewArray { .. } => stack.convert(Value::I32, Value::Ref),
            RawInstruction::ArrayLength => stack.convert(Value::Ref, Value::I32),
            RawInstruction::BIPush { .. } => stack.push(Value::I32),
            RawInstruction::CheckCast { .. } => stack.convert(Value::Ref, Value::Ref),
            RawInstruction::D2F => stack.convert(Value::F64, Value::F32),
            RawInstruction::D2I => stack.convert(Value::F64, Value::I32),
            RawInstruction::D2L => stack.convert(Value::F64, Value::I64),
            RawInstruction::DDiv |
            RawInstruction::DMul |
            RawInstruction::DRem |
            RawInstruction::DSub |
            RawInstruction::DAdd => stack.bin_op(Value::F64),
            RawInstruction::DCmpG |
            RawInstruction::DCmpL => {stack.pop_known(Value::F64); stack.pop_known(Value::F64); stack.push(Value::I32)},
            RawInstruction::DConst0 |
            RawInstruction::DConst1 |
            RawInstruction::DLoad { .. } |
            RawInstruction::DLoadW { .. } |
            RawInstruction::DLoad0 |
            RawInstruction::DLoad1 |
            RawInstruction::DLoad2 |
            RawInstruction::DLoad3 => stack.push(Value::F64),
            RawInstruction::DNeg => todo!(),
            RawInstruction::DStore {..} |
            RawInstruction::DStoreW {..} |
            RawInstruction::DStore0 |
            RawInstruction::DStore1 |
            RawInstruction::DStore2 |
            RawInstruction::DStore3 |
            RawInstruction::DReturn => {stack.pop_known(Value::F64);}
            RawInstruction::Dup => {
                let v = stack.pop_unknown(); 
                assert!(v.is_small(), "Dup only works with short values!");
                stack.push(v);
                stack.push(v);
            },
            RawInstruction::DupX1 => {
                let v1 = stack.pop_unknown();
                assert!(v1.is_small(), "DupX1 only works with short values!");
                let v2 = stack.pop_unknown();
                assert!(v2.is_small(), "DupX1 only works with short values!");
                stack.push(v1);
                stack.push(v2);
                stack.push(v1);
            }
            RawInstruction::DupX2 => {
                let v1 = stack.pop_unknown();
                assert!(v1.is_small(), "DupX2 expects top value to be small!");
                let v2 = stack.pop_unknown();
                if v2.is_small(){
                    let v3 = stack.pop_unknown();
                    assert!(v3.is_small(), "DupX2 expects the third value to be small!");
                    stack.push(v1);
                    stack.push(v3)
                }
                else{
                    stack.push(v1);
                }
                stack.push(v2);
                stack.push(v1);
            }
            RawInstruction::Dup2 => {
                let v1 = stack.pop_unknown();
                if v1.is_small(){
                    let v2 = stack.pop_unknown();
                    assert!(v2.is_small(), "Dup2 expects second value to be small!");
                    stack.push(v2);
                    stack.push(v1);
                    stack.push(v2)
                }
                else{
                    stack.push(v1);
                }
                stack.push(v1);
            },
            RawInstruction::Dup2X1 => {
                let v1 = stack.pop_unknown();
                let v2 = stack.pop_unknown();
                assert!(v2.is_small(), "Dup2X1 expects second value to be small");
                if v1.is_small(){
                    let v3 = stack.pop_unknown();
                    assert!(v3.is_small(), "Dup2X1 expects third value to be small");
                    stack.push(v2);
                    stack.push(v1);
                    stack.push(v3);
                }
                else {
                    stack.push(v1);
                }
                stack.push(v2);
                stack.push(v1);
            }
            RawInstruction::Dup2X2 => {
                let v1 = stack.pop_unknown();
                let v2 = stack.pop_unknown();
                if v1.is_big(){
                    if v2.is_small(){
                        let v3 = stack.pop_unknown();
                        assert!(v3.is_small());

                        stack.push(v1);
                        stack.push(v3);
                    }
                    else {
                        stack.push(v1);
                    }
                    stack.push(v2);
                    stack.push(v1);
                }
                else{
                    assert!(v2.is_small());
                    let v3 = stack.pop_unknown();
                    if v3.is_small(){
                        let v4 = stack.pop_unknown();
                        assert!(v4.is_small());
                        stack.push(v2);
                        stack.push(v1);
                        stack.push(v4);
                        
                    }
                    else{
                        stack.push(v2);
                        stack.push(v1);
                    }
                    stack.push(v3);
                    stack.push(v2);
                    stack.push(v1);
                }
            }
            RawInstruction::F2D => stack.convert(Value::F32, Value::F64),
            RawInstruction::F2I => stack.convert(Value::F32, Value::I32),
            RawInstruction::F2L => stack.convert(Value::F32, Value::I64),
            RawInstruction::FAdd |
            RawInstruction::FDiv |
            RawInstruction::FMul |
            RawInstruction::FSub |
            RawInstruction::FRem => stack.bin_op(Value::F32),
            RawInstruction::FCmpG |
            RawInstruction::FCmpL => {stack.pop_known(Value::F32); stack.pop_known(Value::F32); stack.push(Value::I32); }
            RawInstruction::FConst0 |
            RawInstruction::FConst1 |
            RawInstruction::FConst2 |
            RawInstruction::FLoad { .. } |
            RawInstruction::FLoadW { .. } |
            RawInstruction::FLoad0 |
            RawInstruction::FLoad1 |
            RawInstruction::FLoad2 |
            RawInstruction::FReturn |
            RawInstruction::FLoad3 => stack.push(Value::F32),
            RawInstruction::FNeg => stack.convert(Value::F32, Value::F32),
            RawInstruction::FStore { .. } |
            RawInstruction::FStoreW { .. } |
            RawInstruction::FStore0 |
            RawInstruction::FStore1 |
            RawInstruction::FStore2 |
            RawInstruction::FStore3 => {stack.pop_known(Value::F32);},
            RawInstruction::GetField { index } =>{
                stack.pop_known(Value::Ref);
                let fr = cp.get(*index).unwrap();
                let nt = cp.get(fr.name_and_type).unwrap();
                let utf8 = cp.get(nt.descriptor).unwrap();
                let descriptor = utf8.content.to_str().unwrap();
                match descriptor.chars().next() {
                    Some(BYTE | BOOL | SHORT | CHAR | INT) => stack.push(Value::I32),
                    Some(FLOAT) => stack.push(Value::F32),
                    Some(DOUBLE) => stack.push(Value::F64),
                    Some(LONG) => stack.push(Value::I64),
                    Some(REF | ARR) => stack.push(Value::Ref),

                    x => panic!("Invalid descriptor char {:?}", x)
                }
            }
            RawInstruction::GetStatic { index } => {
                let fr = cp.get(*index).unwrap();
                let nt = cp.get(fr.name_and_type).unwrap();
                let utf8 = cp.get(nt.descriptor).unwrap();
                let descriptor = utf8.content.to_str().unwrap();
                match descriptor.chars().next() {
                    Some(BYTE | BOOL | SHORT | CHAR | INT) => stack.push(Value::I32),
                    Some(FLOAT) => stack.push(Value::F32),
                    Some(DOUBLE) => stack.push(Value::F64),
                    Some(LONG) => stack.push(Value::I64),
                    Some(REF | ARR) => stack.push(Value::Ref),

                    x => panic!("Invalid descriptor char {:?}", x)
                }
            },
            RawInstruction::Goto { offset } => (),
            RawInstruction::GotoW { offset } => (),
            RawInstruction::I2S |
            RawInstruction::I2C |
            RawInstruction::I2B => stack.convert(Value::I32, Value::I32),
            RawInstruction::I2D => stack.convert(Value::I32, Value::F32),
            RawInstruction::I2F => stack.convert(Value::I32, Value::F32),
            RawInstruction::I2L => stack.convert(Value::I32, Value::I64),
            RawInstruction::IAnd |
            RawInstruction::IDiv |
            RawInstruction::IMul |
            RawInstruction::IOr |
            RawInstruction::IRem |
            RawInstruction::IAdd => stack.bin_op(Value::I32),
            RawInstruction::IConstM1 |
            RawInstruction::IConst0 |
            RawInstruction::IConst1 |
            RawInstruction::IConst2 |
            RawInstruction::IConst3 |
            RawInstruction::IConst4 |
            RawInstruction::ILoadW { .. } |
            RawInstruction::ILoad { .. } |
            RawInstruction::ILoad0 |
            RawInstruction::ILoad1 |
            RawInstruction::ILoad2 |
            RawInstruction::ILoad3 |
            RawInstruction::IConst5 => stack.push(Value::I32),
            RawInstruction::IfACmpNe { offset } |
            RawInstruction::IfACmpEq { offset } => {stack.pop_known(Value::Ref); stack.pop_known(Value::Ref);},
            RawInstruction::IfICmpEq { offset } |
            RawInstruction::IfICmpNe { offset } |
            RawInstruction::IfICmpLt { offset } |
            RawInstruction::IfICmpGe { offset } |
            RawInstruction::IfICmpGt { offset } |
            RawInstruction::IfICmpLe { offset } => {stack.pop_known(Value::I32); stack.pop_known(Value::I32);},
            RawInstruction::IfEq { offset } |
            RawInstruction::IfNe { offset } |
            RawInstruction::IfLt { offset } |
            RawInstruction::IfGe { offset } |
            RawInstruction::IfGt { offset } |
            RawInstruction::IfLe { offset } => {stack.pop_known(Value::I32);},
            RawInstruction::IfNull { offset } => {stack.pop_known(Value::Ref);}
            RawInstruction::IfNonNull { offset } => {stack.pop_known(Value::Ref);},
            RawInstruction::IInc { index, value } => (),
            RawInstruction::IIncW { index, value } => (),
            RawInstruction::INeg => stack.convert(Value::I32, Value::I32),
            RawInstruction::InstanceOf { index } => stack.convert(Value::Ref, Value::I32),
            RawInstruction::InvokeDynamic { index } => {
                let id = cp.get(*index).unwrap();
                let nt = cp.get(id.name_and_type).unwrap();
                let d = cp.get(nt.descriptor);
            }
            RawInstruction::InvokeInterface { index, count } => todo!(),
            RawInstruction::InvokeSpecial { index } => todo!(),
            RawInstruction::InvokeStatic { index } => todo!(),
            RawInstruction::InvokeVirtual { index } => todo!(),
            RawInstruction::IReturn => todo!(),
            RawInstruction::IShL => todo!(),
            RawInstruction::IShR => todo!(),
            RawInstruction::IStore { index } => todo!(),
            RawInstruction::IStoreW { index } => todo!(),
            RawInstruction::IStore0 => todo!(),
            RawInstruction::IStore1 => todo!(),
            RawInstruction::IStore2 => todo!(),
            RawInstruction::IStore3 => todo!(),
            RawInstruction::ISub => todo!(),
            RawInstruction::IUShR => todo!(),
            RawInstruction::IXor => todo!(),
            RawInstruction::JSr { offset } => todo!(),
            RawInstruction::JSrW { offset } => todo!(),
            RawInstruction::L2D => todo!(),
            RawInstruction::L2F => todo!(),
            RawInstruction::L2I => todo!(),
            RawInstruction::LAdd => todo!(),
            RawInstruction::LAnd => todo!(),
            RawInstruction::LCmp => todo!(),
            RawInstruction::LConst0 => todo!(),
            RawInstruction::LConst1 => todo!(),
            RawInstruction::LdC { index } => todo!(),
            RawInstruction::LdCW { index } => todo!(),
            RawInstruction::LdC2W { index } => todo!(),
            RawInstruction::LDiv => todo!(),
            RawInstruction::LLoad { index } => todo!(),
            RawInstruction::LLoadW { index } => todo!(),
            RawInstruction::LLoad0 => todo!(),
            RawInstruction::LLoad1 => todo!(),
            RawInstruction::LLoad2 => todo!(),
            RawInstruction::LLoad3 => todo!(),
            RawInstruction::LMul => todo!(),
            RawInstruction::LNeg => todo!(),
            RawInstruction::LookupSwitch(_) => todo!(),
            RawInstruction::LOr => todo!(),
            RawInstruction::LRem => todo!(),
            RawInstruction::LReturn => todo!(),
            RawInstruction::LShL => todo!(),
            RawInstruction::LShR => todo!(),
            RawInstruction::LStore { index } => todo!(),
            RawInstruction::LStoreW { index } => todo!(),
            RawInstruction::LStore0 => todo!(),
            RawInstruction::LStore1 => todo!(),
            RawInstruction::LStore2 => todo!(),
            RawInstruction::LStore3 => todo!(),
            RawInstruction::LSub => todo!(),
            RawInstruction::LUShR => todo!(),
            RawInstruction::LXor => todo!(),
            RawInstruction::MonitorEnter => todo!(),
            RawInstruction::MonitorExit => todo!(),
            RawInstruction::MultiANewArray { index, dimensions } => todo!(),
            RawInstruction::New { index } => todo!(),
            RawInstruction::NewArray { atype } => todo!(),
            RawInstruction::Nop => todo!(),
            RawInstruction::Pop => todo!(),
            RawInstruction::Pop2 => todo!(),
            RawInstruction::PutField { index } => todo!(),
            RawInstruction::PutStatic { index } => todo!(),
            RawInstruction::Ret { index } => todo!(),
            RawInstruction::RetW { index } => todo!(),
            RawInstruction::Return => todo!(),
            RawInstruction::SALoad => todo!(),
            RawInstruction::SAStore => todo!(),
            RawInstruction::SIPush { value } => todo!(),
            RawInstruction::Swap => todo!(),
            RawInstruction::TableSwitch(_) => todo!(),
        }
    }


    stack
} 

fn parse_method_descriptor(descriptor: &str) -> (Vec<char>, Option<char>){
    let mut iterator = descriptor.chars().peekable();
    assert_eq!(iterator.next(), Some('('));

    todo!()
}