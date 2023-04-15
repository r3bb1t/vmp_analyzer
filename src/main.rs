use app::App;
use config::Config;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use vm::VM_HISTORY;
use zydis::DecodedInstruction;

use std::{
    env,
    error::Error,
    io,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    // time::{Duration, Instant},
    time::Duration,
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

mod analysis_x64;
mod app;
mod converter;
mod disasm;
mod emulator_backend;
mod loader;
mod ui;

mod vm;
mod worker_x64;

mod config;

mod optimizations;

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal

    let args: Vec<String> = env::args().collect();

    let arg1 = args[1].clone();
    let arg2 = args[2].as_str().trim_start_matches("0x");
    let arg3 = args[3].as_str().trim_start_matches("0x");

    let conf = config::Config {
        file_path: arg1,
        begin: u64::from_str_radix(&arg2, 16)?,
        until: u64::from_str_radix(&arg3, 16)?,
    };

    println!("{conf:#x?}");

    std::thread::sleep(Duration::from_secs(2));

    let mut app = App::new();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let res = run_app(&mut app, &mut terminal, conf);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    res.unwrap();
    Ok(())
}

fn run_app<B: Backend>(
    app: &mut App,
    terminal: &mut Terminal<B>,
    config: Config,
) -> io::Result<()> {
    let (tx, rx): (
        Sender<Vec<DecodedInstruction>>,
        Receiver<Vec<DecodedInstruction>>,
    ) = mpsc::channel();

    thread::spawn(move || worker_x64::simple_devirt(tx, &config))
        .join()
        .unwrap();

    let mut vm_history = VM_HISTORY::new();

    while let Ok(history) = rx.try_recv() {
        if !analysis_x64::analysis::is_vm_exit(&history) {
            vm_history.ins_history.push(history.clone());
        } else {
            vm_history.ins_history.push(history.clone());
            app.vm_state.vms_histories.push(vm_history.clone());
            vm_history.ins_history.clear();
        }
    }

    // If we crashed and got no vm_exit, pushing what we could collect

    let mut print_err_msg = || {
        println!("We crashed during emulation. Outputting what we could capture");
        std::thread::sleep(Duration::from_secs(3));
        terminal.clear().unwrap();
    };

    if let Some(data) = app.vm_state.vms_histories.last() {
        if data != &vm_history {
            app.vm_state.vms_histories.push(vm_history.clone());
            print_err_msg();
        }
    } else {
        app.vm_state.vms_histories.push(vm_history.clone());
        print_err_msg();
    }

    app.vm_choice_index.select(Some(0));

    loop {
        terminal.draw(|f| ui::ui_v2(f, app))?;

        if let Ok(Event::Key(key)) = event::read() {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('Q') | KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Left => app.previous_vm(),
                KeyCode::Right => app.next_vm(),
                KeyCode::Down => app.next_vm_instr_block(),
                KeyCode::Up => app.previous_vm_instr_block(),

                KeyCode::Char('S') | KeyCode::Char('s') => app.dump_current_vm_to_file(terminal)?,

                _ => {}
            }
        }
    }
}
