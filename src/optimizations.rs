#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(unused)]
pub mod first_level_optimizations {
    use std::collections::HashMap;
    use unicorn_engine::unicorn_const::uc_error;
    use unicorn_engine::RegisterX86;
    use unicorn_engine::Unicorn;
    use zydis::enums::generated::Mnemonic;
    use zydis::DecodedInstruction;
    use zydis::DecodedOperand;
    use zydis::OperandAction;
    // use zydis::enums::generated::Register;
    use zydis::enums::generated::OperandType;

    use crate::disasm;
    use crate::disasm::decode_instruction;
    use crate::vm::VM_STATE_V2;
    use zydis::enums::generated::Register;

    use crate::converter::zydis_to_unicorn;

    // Removes everything after xor [rsp], reg
    pub fn pre_optimization(
        instruction_history: &mut Vec<DecodedInstruction>,
        vm_state: &VM_STATE_V2,
        stack_operands: &Vec<Register>,
    ) {
        let important = [vm_state.vsp, vm_state.vip];

        // TODO: After "xor [rsp], reg" there is usually nothing usefull, 
        // however i need to write a better optimiztions and remove both this and stop skipping instrs like clc
        
        if let Some(pos) = instruction_history.iter().rposition(|instr| {
            instr.mnemonic == Mnemonic::XOR
                && instr.operands[0].mem.base == Register::RSP
                && instr.operands[0].mem.index == Register::NONE
        }) {
            instruction_history.drain(pos..);
        }

        
        if let Some(instruction) =  instruction_history.last() {
            if instruction.mnemonic == Mnemonic::PUSH && ! instruction.operands.iter().any(|op| important.contains(&op.reg)) {
                instruction_history.pop();
            }
        }

        if let Some(pos) = instruction_history.iter().rposition(|instr| {
            instr.operands.iter().any(|op| {
                important.contains(&op.mem.index)
                    || important.contains(&op.mem.base)
                    || important.contains(&op.reg)
                    || op.mem.base == vm_state.vm_registers_base // && op.mem.disp.displacement != 0
                    // || op.mem.index == vm_state.vm_registers_base
            })
        }) {
            instruction_history.drain(pos+1..);
        }



    }
}
