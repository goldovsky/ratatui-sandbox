use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::error::Error;
use std::io;
use std::process::Command;

pub fn dry_run_command(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    command: &str,
) -> Result<(), Box<dyn Error>> {
    // Temporarily leave alternate screen and restore cooked mode to print the command
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    println!("Dry run: {}", command);
    println!("Press Enter to continue...");

    // wait for Enter on stdin
    let mut buf = String::new();
    let _ = std::io::stdin().read_line(&mut buf)?;

    // restore alternate screen and raw mode
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    enable_raw_mode()?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    Ok(())
}

pub fn run_command(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    command: &str,
) -> Result<(), Box<dyn Error>> {
    // Restore terminal to normal mode and hand over TTY to child process
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Spawn a shell to run the command so shell features are available
    let status = Command::new("sh").arg("-c").arg(command).status()?;

    eprintln!("Command exited with: {}", status);

    // Re-enter the alternate screen and raw mode
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    enable_raw_mode()?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    Ok(())
}
