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
                    // collect lines and trim leading/trailing empty lines produced by some figlet fonts
                    let mut lines: Vec<String> = s.lines().map(|l| l.to_string()).collect();
                    while lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
                        lines.pop();
                    }
                    while lines.first().map(|l| l.trim().is_empty()).unwrap_or(false) {
                        lines.remove(0);
                    }
                    return lines
                        .into_iter()
                        .map(|l| {
                            Spans::from(Span::styled(
                                l,
                                Style::default().fg(Color::Rgb(255, 165, 0)),
                            ))
                        })
                        .collect();
                }
            }
        }
    }

    // If a bundled font exists in assets/fonts (e.g. "ANSI Shadow.flf"), try that.
    let bundled_font = "assets/fonts/ANSI Shadow.flf";
    if std::path::Path::new(bundled_font).exists() {
        if let Ok(output) = Command::new("figlet")
            .arg("-f")
            .arg(bundled_font)
            .arg("CALLBOT")
            .output()
        {
            if output.status.success() {
                if let Ok(s) = String::from_utf8(output.stdout) {
                    let mut lines: Vec<String> = s.lines().map(|l| l.to_string()).collect();
                    while lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
                        lines.pop();
                    }
                    while lines.first().map(|l| l.trim().is_empty()).unwrap_or(false) {
                        lines.remove(0);
                    }
                    return lines
                        .into_iter()
                        .map(|l| {
                            Spans::from(Span::styled(
                                l,
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
                let mut lines: Vec<String> = s.lines().map(|l| l.to_string()).collect();
                while lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
                    lines.pop();
                }
                while lines.first().map(|l| l.trim().is_empty()).unwrap_or(false) {
                    lines.remove(0);
                }
                return lines
                    .into_iter()
                    .map(|l| {
                        Spans::from(Span::styled(
                            l,
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
