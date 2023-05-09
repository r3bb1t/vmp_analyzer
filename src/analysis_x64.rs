#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
pub mod analysis {

    use unicorn_engine::Unicorn;
    use zydis::DecodedOperand;
    use zydis::enums::generated::Mnemonic;
    use zydis::DecodedInstruction;
    use zydis::ElementType;
    use zydis::MemoryOperandType;
    use zydis::OperandAction;
    use zydis::OperandEncoding;

    use zydis::enums::generated::OperandType;

    use crate::vm::VM_STATE_V2;
    use zydis::enums::generated::Register;

    use crate::converter::zydis_to_unicorn;

    /// Check if jmp to register or RET instruction
    pub fn is_bb_end(instruction: &DecodedInstruction) -> bool {
        instruction.mnemonic == Mnemonic::RET
            || instruction.mnemonic == Mnemonic::JMP
                && instruction.operands[0].ty == OperandType::REGISTER
    }

    /// Checks whether instruction is:  
    ///
    /// mov dword_reg, qword ptr ss:[rsp+0x90]
    ///
    /// TODO: Replace it. Not valid for VMP 3.8
    pub fn is_vip_init(instruction: &DecodedInstruction) -> bool {
        // Byte representation of instruction should look like: 48 8B B4 24 90 00 00 00
        // Third byte (B4) represents the dword register and can be different for various VMs
        let first_operanad = &instruction.operands[0];
        let second_operand = &instruction.operands[1];

        instruction.mnemonic == Mnemonic::MOV
            && second_operand.ty == OperandType::MEMORY
            && second_operand.mem.base == Register::RSP
            && second_operand.mem.disp.displacement == 0x90
            && first_operanad.ty == OperandType::REGISTER
            && first_operanad.element_size == 64
    }

    pub fn is_vm_exit(instrs: &Vec<DecodedInstruction>) -> bool {

        let registers_written_from_stack = instrs
            .iter()
            .filter(|instr| is_real_stack_access(instr) && instr.operands.iter().any(|op| op.ty == OperandType::REGISTER && op.action == OperandAction::WRITE))
            .count();

        registers_written_from_stack > 13

    }

    /// Replace [rsp+reg] with [rsp+imm]
    ///
    /// TODO: Change return type to result enum
    pub fn craft_constant_v2(
        uc: &mut Unicorn<()>,
        vm_state: &VM_STATE_V2,
        instruction: &mut DecodedInstruction,
    ) -> DecodedInstruction {
        if let Some(pos) = instruction.operands.iter().position(|op| {
            op.mem.base == vm_state.vm_registers_base && op.mem.index != Register::NONE
        }) {
            let mut new_operand = instruction.operands[pos].clone();
            let const_value = uc
                .reg_read(zydis_to_unicorn(&new_operand.mem.index))
                .unwrap();

            new_operand.mem.index = Register::NONE;
            new_operand.mem.disp.has_displacement = true;
            new_operand.mem.disp.displacement = const_value as i64;

            instruction.operands[pos] = new_operand;
        }

        return instruction.clone();
    }

    pub fn unfold_constant(
        uc: &mut Unicorn<()>,
        instruction: &DecodedInstruction,
    ) -> DecodedInstruction {
        let mut new_instruction = instruction.clone();

        if let Some(pos) = new_instruction
            .operands
            .iter()
            .position(|op| op.action == OperandAction::READ && op.ty == OperandType::REGISTER)
        {
            let mut new_operand = new_instruction.operands[pos].clone();
            let const_value = uc.reg_read(zydis_to_unicorn(&new_operand.reg)).unwrap();

            // Waiting for zydis v4 to come to avoid all this dirty instruction/operand patching
            new_operand.ty = OperandType::IMMEDIATE;
            new_operand.encoding = OperandEncoding::UIMM16_32_64;
            new_operand.element_type = ElementType::INT;
            new_operand.element_count = 1;
            new_operand.reg = Register::NONE;

            new_operand.mem.ty = MemoryOperandType::INVALID;
            new_operand.mem.segment = Register::NONE;
            new_operand.mem.base = Register::NONE;
            new_operand.mem.scale = 0x0;
            new_operand.mem.disp.has_displacement = false;
            new_operand.mem.disp.displacement = 0;

            new_operand.ptr.segment = 0x0;
            new_operand.ptr.offset = 0x0;

            new_operand.imm.is_signed = false;
            new_operand.imm.is_relative = false;
            new_operand.imm.value = const_value;

            new_instruction.operands[pos] = new_operand;
            new_instruction.raw.imm[0].value = const_value;
        }
        return new_instruction;
    }

    pub fn get_stack_operand(
        stack_operands: &Vec<Register>,
        instruction: &DecodedInstruction,
    ) -> Option<DecodedOperand> {
        for operand in &instruction.operands {
            if stack_operands.contains(&operand.reg)
                || stack_operands.contains(&operand.mem.base)
                || stack_operands.contains(&operand.mem.index)
            {
                return Some(operand.clone());
            }
        }
        None
    }

    // Checks whether the real stack has been accesed
    pub fn is_real_stack_access(instruction: &DecodedInstruction) -> bool {
        instruction.operands.iter().any(|op| {
            op.ty == OperandType::MEMORY
                && op.mem.segment == Register::SS
                && op.mem.base == Register::RSP
        })
    }
}
