use crate::config::Config;
use crate::loader;

use unicorn_engine::unicorn_const::{Arch, Mode, Permission};
use unicorn_engine::RegisterX86;
use unicorn_engine::Unicorn;


pub fn prepare_emulator(config: &Config) -> Unicorn<'static, ()> {


    let data = loader::get_segments(&config.file_path).unwrap();
    return get_emulator(data).unwrap();
}

fn get_emulator(
    data: Vec<loader::SegmentData>,
) -> Result<Unicorn<'static, ()>, unicorn_engine::unicorn_const::uc_error> {
    const STACK_ADDRESS: u64 = 0x0; // 0x7ffffffde000;
    const STACK_SIZE: u64 = 0x40000;

    let mut unicorn: Unicorn<'static, ()> = Unicorn::new(Arch::X86, Mode::MODE_64)?;
    let uc = &mut unicorn;

    // setting up the stack
    uc.mem_map(STACK_ADDRESS, 1024 * 1024, Permission::all())?;
    uc.mem_write(STACK_ADDRESS, &[0; STACK_SIZE as usize])?;
    uc.reg_write(RegisterX86::RSP, STACK_ADDRESS + STACK_SIZE - 1)?;

    // loading the segments
    for element in data {
        uc.mem_map(element.start, element.size as usize, Permission::all())?;
        uc.mem_write(element.start, &element.data)?;
    }

    Ok(unicorn)
}
