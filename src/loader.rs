use std::collections::HashMap;
use std::io;
use std::io::prelude::*;
use std::fs::File;

use goblin::error;
use goblin::pe::PE;

pub fn align(mut var: usize) -> usize {
    while var % 0x1000 != 0 {
        var += 1;
    }
    var
} 

#[derive(Debug)]
pub struct SegmentData {
    pub name: String,
    pub size: usize,
    pub start: u64,
    pub imports: HashMap<String, usize>,
    pub data: Vec<u8>,
}

impl SegmentData {
    fn new() -> SegmentData {
        SegmentData { name: String::from(""), size: 0, start: 0, imports: HashMap::new(), data: vec![0;0], }
    }
}



fn give_module_buffers(executable: &mut File, size: u32, offset: u64) -> Result<Vec<u8>, io::Error> {
    let mut buffer: Vec<u8> = vec![0; size as usize];
    executable.seek(io::SeekFrom::Start(offset))?;
    executable.read_exact(&mut buffer)?;
    Ok(buffer)
}



/// Dumps all the segments to new files
pub fn get_segments(file_path: &str) -> error::Result<Vec<SegmentData>>   {


    // let mut results: Vec<Vec<u8>> = Vec::new();
    let mut results: Vec<SegmentData> = Vec::new();

    let mut file = File::open(file_path)?;
    let file_size = file.metadata()?.len();
    let mut file_buf: Vec<u8> = vec![0; file_size as usize];
    

    file.read_exact(&mut file_buf)?;



    let pe = PE::parse(& file_buf)?;
    for section in pe.sections {

        let mut current_section = SegmentData::new();


        let name = section.name()?;
        let offset = section.pointer_to_raw_data as u64;
        let size = section.size_of_raw_data as usize;

        if size == 0 {continue};


        current_section.data = give_module_buffers( &mut file,  size as u32, offset)?;
        current_section.name = String::from(name);
        current_section.size = align(size);
        current_section.start = pe.image_base as u64 + section.virtual_address as u64;
        
        results.push(current_section);
    }
    Ok(results)
}


