use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::error::Error;
use std::io;
use std::path::PathBuf;

mod config;
mod runner;
mod ui;

use config::Config;
use ui::run_app as ui_run_app;
use ui::App as UiApp;

fn main() -> Result<(), Box<dyn Error>> {
    // Load configuration before initializing the terminal
    // Try multiple locations: current directory first, then next to executable
    let config_path = find_config_file()?;
    let config = Config::load(&config_path)?;

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
    let app = UiApp::new(config);
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

/// Find config.toml in current directory or next to executable
fn find_config_file() -> Result<PathBuf, Box<dyn Error>> {
    // Try current working directory first
    let cwd_config = PathBuf::from("config.toml");
    if cwd_config.exists() {
        return Ok(cwd_config);
    }

    // Try next to executable
    let exe_path = std::env::current_exe()?;
    if let Some(exe_dir) = exe_path.parent() {
        let exe_config = exe_dir.join("config.toml");
        if exe_config.exists() {
            return Ok(exe_config);
        }
    }

    Err(format!(
        "Configuration file 'config.toml' not found.\n\
         Searched in:\n\
         - Current directory: {}\n\
         - Executable directory: {}\n\n\
         Please create a config.toml file in one of these locations.",
        std::env::current_dir()?.display(),
        exe_path
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    )
    .into())
}
