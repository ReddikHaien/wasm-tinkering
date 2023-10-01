pub struct ParsedClass{
    name: Box<[u8]>,
    inherited: Option<Box<[u8]>>,
    interfaces: Vec<Box<[u8]>>,   
}