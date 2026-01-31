use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::error::Error;
use std::io;

mod runner;
mod ui;

use ui::run_app as ui_run_app;
use ui::App as UiApp;

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // switch to the alternate screen and enable mouse capture so the app does not
    // leave UI artifacts on the main terminal when it exits
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    // ensure the alternate screen is cleared and hide the cursor while the app runs
    terminal.clear()?;
    terminal.hide_cursor()?;

    // create the UI app and hand off to the ui module
    let app = UiApp::new();
    let res = ui_run_app(&mut terminal, app);

    // restore terminal state
    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {}", err);
    }

    Ok(())
}
