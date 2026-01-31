use ratatui::style::{Color, Style};
use ratatui::text::{Span, Spans};
use std::process::Command;

// Try to render the title using the `figlet` program. If unavailable, fall back to
// a built-in ASCII art. Returns lines already wrapped as `Spans` so the caller can
// render them directly in a Paragraph. The function does NOT include the subtitle
// line; the UI appends that explicitly to guarantee it's visible.
pub fn title_spans() -> Vec<Spans<'static>> {
    // If CALLBOT_FIGLET_FONT is set, try to use that font first.
    if let Ok(font) = std::env::var("CALLBOT_FIGLET_FONT") {
        if let Ok(output) = Command::new("figlet")
            .arg("-f")
            .arg(&font)
            .arg("CALLBOT")
            .output()
        {
            if output.status.success() {
                if let Ok(s) = String::from_utf8(output.stdout) {
                    return s
                        .lines()
                        .map(|l| {
                            Spans::from(Span::styled(
                                l.to_string(),
                                Style::default().fg(Color::Rgb(255, 165, 0)),
                            ))
                        })
                        .collect();
                }
            }
        }
    }

    // Try figlet without font (system default)
    if let Ok(output) = Command::new("figlet").arg("CALLBOT").output() {
        if output.status.success() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                return s
                    .lines()
                    .map(|l| {
                        Spans::from(Span::styled(
                            l.to_string(),
                            Style::default().fg(Color::Rgb(255, 165, 0)),
                        ))
                    })
                    .collect();
            }
        }
    }

    // Fallback static ASCII
    let ascii = [
        r"  ____    _    _ _     ____   ____  ",
        r" / ___|  / \\  | | |   | __ ) | __ ) ",
        r"| |     / _ \\ | | |   |  _ \\ |  _ ",
        r"| |___ / ___ \\| | |___| |_) || |_) |",
        r" \\____/_/   \\_\\_|_____|____/ |____/",
    ];

    ascii
        .iter()
        .map(|l| {
            Spans::from(Span::styled(
                l.to_string(),
                Style::default().fg(Color::Rgb(255, 165, 0)),
            ))
        })
        .collect()
}
