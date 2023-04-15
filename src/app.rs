use std::{io::{self,Write}, fs::OpenOptions};

use tui::{widgets::ListState, backend::Backend, Terminal};
use zydis::DecodedInstruction;

use crate::vm::VM_STATE_V2;


use crate::disasm;




#[allow(non_camel_case_types)]
#[derive(Debug)]
pub struct App {
    pub vm_state: VM_STATE_V2,
    pub vm_choice_index: ListState,
    pub vm_instructions: Vec<String>,
    pub vm_instructions_state: ListState,
    pub vm_registers_state: ListState,
    pub original_instructions: Vec<DecodedInstruction>,


    pub cur_instr: ListState,
    pub vm_count: usize,
}

impl App {
    pub fn new() -> App {
        App {
            vm_state: VM_STATE_V2::new(),
            vm_instructions: Vec::with_capacity(30),
            vm_choice_index: ListState::default(),
            vm_instructions_state: ListState::default(),
            vm_registers_state: ListState::default(),
            original_instructions: Vec::new(),



            cur_instr: ListState::default(),
            vm_count: 1,
        }
    }

    pub fn next_vm(&mut self) {
        self.cur_instr.select(Some(0));
        if let Some(current_id) = self.vm_choice_index.selected() {
            if current_id + 1 < self.vm_state.vms_histories.len() {
                self.vm_choice_index.select(Some(current_id + 1));
                self.vm_state.vm_choice_index.select(Some(current_id + 1));
            }
        }
    }

    pub fn previous_vm(&mut self) {
        self.cur_instr.select(Some(0));
        if let Some(current_id) = self.vm_choice_index.selected() {
            if current_id != 0 {
                self.vm_choice_index.select(Some(current_id - 1));
                self.vm_state.vm_choice_index.select(Some(current_id - 1));
            }
        }
    }

    pub fn next_vm_instr_block(&mut self) {
        let current_vm_id = self.vm_choice_index.selected().unwrap_or(0);
        let current_choice = self.cur_instr.selected().unwrap_or(0);
        let block_length = self.vm_state.vms_histories[current_vm_id].ins_history.len();

        if current_choice != block_length - 1 {
            self.cur_instr.select(Some(current_choice + 1));
        } else {
            self.cur_instr.select(Some(0));
        }
    }

    pub fn previous_vm_instr_block(&mut self) {
        let current_vm_id = self.vm_choice_index.selected().unwrap_or(0);
        let current_instr_choice = self.cur_instr.selected().unwrap_or(0);

        let block_length = self.vm_state.vms_histories[current_vm_id].ins_history.len();

        let new_index = current_instr_choice
            .checked_sub(1)
            .unwrap_or(block_length - 1);

        self.cur_instr.select(Some(new_index));
    }


    pub fn dump_current_vm_to_file<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        terminal.clear()?;
        let current_vm_id = self.vm_choice_index.selected().unwrap_or(0);

        let filename = format!("vm_{}.asm", current_vm_id + 1);
        

        let mut file = OpenOptions::new().write(true).create(true).open(filename)?;

        for ins_block in &self.vm_state.vms_histories[current_vm_id].ins_history {

            for instruction in ins_block {
                let disassembly = disasm::better_print(instruction, None);
                writeln!(file, "{disassembly}")?;
            }
            writeln!(file, "nop")?;
        }

        Ok(())
    }
}
