use std::{collections::HashMap};
use tui::widgets::ListState;
use zydis::{enums::generated::Register, DecodedInstruction, OperandAction, Mnemonic, OperandType};

use crate::{disasm, analysis_x64::analysis::is_vip_init};

/// Before the execution of a vm, 0x180 bytes are being allocated on the stack by
/// using the following instruction:
/// sub rsp, 0x180
const MEMORY_SPACE: usize = 0x180;

const VREGS_SIZE: usize = MEMORY_SPACE - 0x140;
const VSTACK_SIZE: usize = MEMORY_SPACE - VREGS_SIZE;



#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VM_HISTORY {
    pub ins_history: Vec<Vec<DecodedInstruction>>,
    pub vsps: Vec<Register>,
    pub start_addr: u64,
    pub end_addr: u64,
    pub i: usize,
}

impl VM_HISTORY {
    pub fn new() -> VM_HISTORY {
        VM_HISTORY {
            ins_history: Vec::with_capacity(4),
            i: 0,
            start_addr: 0,
            end_addr: 0,
            vsps: Vec::new(),
        }
    }


}

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub struct VM_STATE_V2 {
    /// VM's stack pointer
    pub vsp: Register,
    /// Instruction pointer, not using it, but keeping track of it for some reason
    pub vip: Register,
    /// Vmprotect 3.5 accesses VM's registers by \[rsp+reg\], newer versions don't necessary use rsp anymore
    pub vm_registers_base: Register,
    pub vm_count: usize,
    /// Using it to keep real register locations for future lifting
    pub real_register_locations: HashMap<u64, Register>,
    /// Used for analysis of a single virtual instruction
    pub instruction_history: Vec<DecodedInstruction>,
    /// Addresses and values of VM's registers. Assuming that they are all 64 bit. (May be wrong)
    pub vm_regs_state: HashMap<u64, u64>,
    pub vm_stack: Vec<u8>,
    pub vms_histories: Vec<VM_HISTORY>,

    /// Final output
    pub final_output: Option<Vec<Vec<String>>>,

    pub vms_histories_debug: Vec<Vec<String>>,


    pub in_vm: bool,
    pub vm_choice_index: ListState,
}

impl VM_STATE_V2 {
    pub fn new() -> VM_STATE_V2 {
        VM_STATE_V2 {
            vsp: Register::NONE,
            vip: Register::NONE,
            vm_registers_base: Register::RSP,
            vm_count: 0,
            real_register_locations: HashMap::with_capacity(17), // Not sure which capacity is the best
            instruction_history: Vec::new(),
            vm_regs_state: HashMap::with_capacity(VREGS_SIZE / 8),
            vm_stack: Vec::with_capacity(VSTACK_SIZE),
            vms_histories: Vec::with_capacity(4),
            vms_histories_debug: Vec::new(),
            final_output: None,
            vm_choice_index: ListState::default(),

            in_vm: false,
        }
    }


    /// Find VSP and pointer to virtual registers
    /// 
    /// Searches for the pattern: mov (some 64 bit register), rsp
    /// 
    /// Old because newest versions have a different vm entry
    pub fn parse_old_vm_entry(&mut self) -> Vec<DecodedInstruction> {
        
        // TODO: Make it proper
        let mut entry_clean_code: Vec<DecodedInstruction> = Vec::new(); 
    
        // TODO: Add check for false positives.
        for instruction in &self.instruction_history {


            if instruction.mnemonic == Mnemonic::PUSH || instruction.mnemonic == Mnemonic::PUSHFQ { entry_clean_code.push(instruction.clone()); }
    
            // Finding the VSP
            if instruction.operands.iter().any(|op| op.reg == Register::RSP && op.action == OperandAction::READ) {
                if let Some(op) = instruction.operands.iter().find(|op| op.ty == OperandType::REGISTER && op.action == OperandAction::WRITE) {
                    self.vsp = op.reg;
                    entry_clean_code.push(instruction.clone());
                }
            }
    
            // Finding the VIP
            else if is_vip_init(&instruction) {
                self.vip = instruction.operands[0].reg;
                entry_clean_code.push(instruction.clone());
            }

            else if instruction.operands.iter().any(|op| op.reg == Register::RSP && op.action == OperandAction::READ) {
                if let Some(operand) = instruction.operands.iter().find(|op| op.reg != Register::NONE && op.action == OperandAction::WRITE) {
                    self.vm_registers_base = operand.reg;
                    entry_clean_code.push(instruction.clone());
                }
            }
            
        }
        self.in_vm = true;
        self.vm_count += 1;
    
        // Check if we didn't find VIP or VSP
        assert!(self.vsp != Register::NONE || self.vip != Register::NONE, "Unable to parse VM_ENTRY");

        while let Some(last_instruction) = entry_clean_code.last() {
            if !last_instruction.operands.iter().any(|op| op.reg == self.vsp || op.reg == self.vip) {
                entry_clean_code.pop();
            }
            else {
                break;
            }
        }
    
        entry_clean_code
    
    }

    // Checks if we encountered the VM_EXIT.
    // If so, returning the vector of it
    pub fn is_vm_exit(&mut self) -> Option<Vec<DecodedInstruction>> {

        let mut output: Vec<DecodedInstruction> = Vec::new();

        for instruction in &self.instruction_history {
            if [Mnemonic::POP, Mnemonic::POPFQ].contains(&instruction.mnemonic) {
                output.push(instruction.clone());
            }

            if instruction.operands.iter().any(|op| op.reg == Register::RSP && op.action == OperandAction::WRITE) {
                output.push(instruction.clone());
            }
        }

        if output.len() > 10 { Some(output) } else { None }
    }


    pub fn fill_final_output(&mut self) {
        if self.final_output.is_some() {
            return;
        }

        let vms_count = self.vms_histories.len();

        let mut to_push = Vec::new();
        for i in 0..vms_count {
            let mut imm: Vec<String> = Vec::new();

            for history in self.vms_histories[i].ins_history.clone() {
                imm.push(self.convert_to_strings(history));
            }

            to_push.push(imm);
        }


        self.final_output = Some(to_push);

    }

    fn convert_to_strings(&mut self, block: Vec<DecodedInstruction>) -> String {
        let mut _block = block.clone();
        let imm: Vec<String> = block
            .into_iter()
            .map(|instruction| disasm::better_print(&instruction, None))
            .collect();

        format!("{:#?}", imm)
    }

    




    
}




