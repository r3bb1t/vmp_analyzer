extern crate zydis;

use zydis::*;

pub fn decode_instruction(code: &[u8]) -> Result<DecodedInstruction> {
    let decoder = Decoder::new(MachineMode::LONG_64, AddressWidth::_64)?;
    let decoded_instr: DecodedInstruction = decoder.decode(code)?.unwrap();

    Ok(decoded_instr)
}

pub fn better_print(instruction: &DecodedInstruction, address: Option<u64>) -> String {
    let mut buffer = [0u8; 64];
    // A wrapped version of the buffer allowing nicer access.
    let mut buffer = OutputBuffer::new(&mut buffer[..]);

    let formatter = Formatter::new(FormatterStyle::INTEL).unwrap();
    formatter
        .format_instruction(&instruction, &mut buffer, address, None)
        .unwrap();

    buffer.as_str().unwrap().to_string()
}
