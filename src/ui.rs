use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};


use crate::app::{App};

pub fn ui_v2<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(15), Constraint::Percentage(50)].as_ref())
        .split(f.size());

    // if app.vm_state.vms_histories.len() != 0 {
    //     f.render_stateful_widget(render_instructions(app), chunks[0], &mut app.vm_instructions_state);
    // }
    // f.render_stateful_widget(render_registers(app), chunks[1], &mut app.vm_registers_state);

    let mut vm_choice_index = app.vm_choice_index.clone();
    let mut unk_inst_state = app.cur_instr.clone();

    f.render_stateful_widget(render_vms_list(app), chunks[0], &mut vm_choice_index);

    f.render_stateful_widget(
        test_show_already_calculated(app),
        chunks[1],
        &mut unk_inst_state,
    );
}

fn render_vms_list(app: &mut App) -> List {
    let mut vms: Vec<ListItem> = Vec::new();

    let mut i = 0;
    for _ in &app.vm_state.vms_histories {
        i += 1;
        vms.push(ListItem::new(format!("{:?}", i)));
    }

    let items = List::new(vms)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "VM {} out of {}",
            app.vm_choice_index.selected().unwrap_or(0) + 1,
            app.vm_state.vms_histories.len()
        )))
        .highlight_style(
            Style::default()
                .bg(Color::Green)
                .add_modifier(Modifier::UNDERLINED),
        )
        .highlight_symbol("  ");

    items
}

fn test_show_already_calculated(app: &mut App) -> List {
    let mut instrs_items: Vec<ListItem> = Vec::new();
    if app.vm_state.final_output.is_none() {
        app.vm_state.fill_final_output();
    }

    let final_output = app.vm_state.final_output.clone().unwrap();
    // dbg!(app.vm_choice_index.selected());
    let chosen_index = app.vm_choice_index.selected().unwrap_or(0);

    for string_vector in &final_output[chosen_index] {
        instrs_items.push(ListItem::new(string_vector.clone()));
    }

    let items = List::new(instrs_items.clone())
        .block(Block::default().borders(Borders::ALL).title(format!(
            "instr {} out of {}",
            app.cur_instr.selected().unwrap_or(0) + 1,
            &final_output[chosen_index].len()
        )))
        .highlight_style(
            Style::default()
                .bg(Color::Red)
                .add_modifier(Modifier::UNDERLINED),
        )
        .highlight_symbol("  ");

    items
}

