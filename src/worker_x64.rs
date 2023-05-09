/// TODO: Write better optimizations and make the code look more tidy
use std::sync::mpsc::Sender;

use unicorn_engine::unicorn_const::uc_error;
use unicorn_engine::{RegisterX86, Unicorn};
use zydis::{DecodedInstruction, OperandVisibility};

use crate::analysis_x64::analysis;
use crate::config::Config;
use crate::converter::zydis_to_unicorn;

use crate::vm::{VM_HISTORY, VM_STATE_V2};
use crate::{disasm, emulator_backend};

use crate::optimizations::first_level_optimizations;

use zydis::enums::{generated::Register, Mnemonic, OperandAction, OperandType};

pub fn simple_devirt(unknown_instr_tx: Sender<Vec<DecodedInstruction>>, config: &Config) {
    let begin = config.begin;
    let until = config.until;
    let mut uc: Unicorn<'static, ()> = emulator_backend::prepare_emulator(&config);

    // Ignoring it's return value to not crash the whole program if emulator crashes
    let _ = run(&mut uc, begin, until, unknown_instr_tx);
}

fn run(
    uc: &mut Unicorn<()>,
    begin: u64,
    until: u64,
    tx: Sender<Vec<DecodedInstruction>>,
) -> Result<(), uc_error> {


    // TODO: Start ignoring instructions with the rolling key

    let mut vm_state: VM_STATE_V2 = VM_STATE_V2::new();

    let mut stack_writes_amount = 0;

    let mut stack_operands: Vec<Register> = Vec::new();

    let mut block_instructions: Vec<(DecodedInstruction, String)> = Vec::new();

    let mut vip_operands: Vec<Register> = Vec::new();

    let mut must_be: Vec<DecodedInstruction> = Vec::new();

    let mut sendable: VM_HISTORY = VM_HISTORY::new();

    let mut return_address = 0;

    let processing = move |uc: &mut Unicorn<()>, address: u64, size: u32| {
        // Shit happens, sometimes emulation can crash when reading memory. So we are skipping such cases
        let code = uc.mem_read_as_vec(address, size as usize);
        if code.is_err() { return; }

        let code = code.unwrap();

        let mut instruction = disasm::decode_instruction(&code).unwrap();


        vm_state.instruction_history.push(instruction.clone());


        if analysis::is_real_stack_access(&instruction) && instruction.operands.iter().any(|op| op.ty == OperandType::REGISTER && op.action == OperandAction::READ) {

            stack_writes_amount += 1;
        }

        // If in VM_ENTRY, process it
        if analysis::is_bb_end(&instruction) && stack_writes_amount > 12 && vm_state.instruction_history.iter().any(|ins| ins.operands.iter().any(|op| op.reg == Register::RSP ) ){


            must_be = vm_state.parse_old_vm_entry();

            let offset = 152;
            let vsp_value = uc.reg_read(zydis_to_unicorn(&vm_state.vsp)).unwrap();


            if return_address == 0 {
                let mut buf: [u8; 8] = [0; 8];
                uc.mem_read(vsp_value + offset, &mut buf).unwrap();
                return_address = u64::from_le_bytes(buf);
            }
        }
        /*********************************************************************************************************/
        /*********************************************************************************************************/
        /*********************************************************************************************************/
        // Tracking accesses to the VM's registers
        else if instruction
            .operands
            .iter()
            .any(|op| op.mem.base == vm_state.vm_registers_base && op.ty != OperandType::REGISTER)
        {
            let patched_instr = analysis::craft_constant_v2(uc, &vm_state, &mut instruction);
            must_be.push(patched_instr);
        }

        // Tracking the operands originating from VIP
        if instruction
            .operands
            .iter()
            .any(|op| op.mem.base == vm_state.vip)
        {
            if let Some(vip_encrypted_operand) = instruction
                .operands
                .iter()
                .find(|op| op.ty == OperandType::REGISTER && op.action == OperandAction::WRITE)
            {
                vip_operands.push(vip_encrypted_operand.reg);
            }
        }

        // Tracking the stack operands
        if let Some(operand) = analysis::get_stack_operand(&stack_operands, &instruction) {
            must_be.push(instruction.clone());
            match operand.action {
                OperandAction::READ => {
                    if let Some(operand) = instruction
                        .operands
                        .iter()
                        .find(|op| op.action == OperandAction::WRITE)
                    {
                        if operand.ty == OperandType::REGISTER && operand.reg != Register::RFLAGS {
                            stack_operands.push(operand.reg);
                        }
                    }
                }
                _ => {}
            }
        }
        // Tracking the vsp accesses
        else if let Some(operand) = instruction.operands.iter().find(|op| {
            op.reg == vm_state.vsp || op.mem.base == vm_state.vsp || op.mem.index == vm_state.vsp
        }) {
            must_be.push(instruction.clone());
            match operand.action {
                OperandAction::READ => {
                    if let Some(operand) = instruction
                        .operands
                        .iter()
                        .find(|op| op.action == OperandAction::WRITE)
                    {
                        if operand.ty == OperandType::REGISTER && operand.reg != Register::RFLAGS {
                            stack_operands.push(operand.reg);
                        }
                    }
                }
                OperandAction::WRITE => {
                    if vm_state.in_vm && instruction.operands.iter().any(|op| {
                        op.action == OperandAction::READ && op.ty == OperandType::REGISTER
                    }) && instruction
                        .operands
                        .iter()
                        .any(|operand| vip_operands.contains(&operand.reg)) {
                        let new_instr = analysis::unfold_constant(uc, &mut instruction);
                        let last_element_id = must_be.len() - 1;
                        must_be[last_element_id] = new_instr;
                    }
                                
                }
                _ => (),
            }
        }
        // Tracking accesses to the VM's registers
        else if instruction
            .operands
            .iter()
            .any(|op| op.mem.base == vm_state.vm_registers_base && op.ty != OperandType::REGISTER)
        {
            let patched_instr = analysis::craft_constant_v2(uc, &vm_state, &mut instruction);
            must_be.push(patched_instr);
        }

        //////////////////////////////////////////////////////////////////////////////////////////////
        //////////////////////////////////////////////////////////////////////////////////////////////
        if analysis::is_bb_end(&instruction) {
            if let Some(to_send) = vm_state.is_vm_exit() {


                // Skipping calls outside of vm if they are not returning from vm
                let rsp_value = uc.reg_read(RegisterX86::RSP).unwrap();
                if instruction.mnemonic == Mnemonic::RET {
                    if rsp_value != return_address {
                        uc.reg_write(RegisterX86::RSP, rsp_value + 8).unwrap();
                    }
                } else if instruction.mnemonic == Mnemonic::JMP {
                    if let Some(operand) = instruction.operands.iter().find(|op| {
                        op.ty == OperandType::REGISTER
                            && op.visibility == OperandVisibility::EXPLICIT
                    }) {
                        let jumping_to = uc.reg_read(zydis_to_unicorn(&operand.reg)).unwrap();
                        if jumping_to != return_address {
                            uc.reg_write(zydis_to_unicorn(&operand.reg), rsp_value + 8)
                                .unwrap();
                        }
                    }
                }
                

                tx.send(to_send).unwrap();

                must_be.clear();

                // TODO: Find the number which fits better than random (10)
                // Probably should be as much pops as pushes in VM_ENTRY.
                // Don't forget the push order
                // tx.send(format!("{:#x} VM_EXIT", address)).unwrap();
                vm_state.in_vm = false;
                vm_state.real_register_locations.clear();

                sendable.end_addr = address;
                sendable.ins_history.clear();
                sendable.i += 1;
                sendable.start_addr = 0;
                sendable.end_addr = 0;
            }

            // TODO: Remove this
            must_be.retain(|instr| instr.mnemonic != Mnemonic::TEST);
            must_be.retain(|instr| instr.mnemonic != Mnemonic::CMP); // Сомнительное решение
            // must_be.retain(|instr| instr.operands.iter().any(|op| op.reg != Register::RIP));
            must_be.retain(|instr| instr.mnemonic != Mnemonic::RET);


            must_be.dedup();
            first_level_optimizations::pre_optimization(&mut must_be, &vm_state, &stack_operands);

            tx.send(must_be.clone()).unwrap();

            // Not sure if it's a good idea to skip empty blocks.
            // Todo: log unknown blocks to file

            sendable.ins_history.push(must_be.clone());
            sendable.i = vm_state.vm_count;

            // Cleanup
            block_instructions.clear();

            vip_operands.clear();

            stack_writes_amount = 0;
            stack_operands.clear();
            vm_state.instruction_history.clear();

            must_be.clear();
        }
    };

    uc.add_code_hook(begin, 0, processing)?;
    uc.emu_start(begin, until, 0, 1_000_000)?;

    Ok(())
}
