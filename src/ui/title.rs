use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Span, Spans};
use std::process::Command;

// Try to render the title using the `figlet` program. If unavailable, fall back to
// a built-in ASCII art. Returns lines already wrapped as `Spans` so the caller can
// render them directly in a Paragraph.
pub fn title_spans() -> Vec<Spans<'static>> {
    // Try figlet
    if let Ok(output) = Command::new("figlet").arg("CALLBOT").output() {
        if output.status.success() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                let mut spans: Vec<Spans> = s
                    .lines()
                    .map(|l| {
                        Spans::from(Span::styled(
                            l.to_string(),
                            // use orange from the spec, not yellow
                            Style::default().fg(Color::Rgb(255, 165, 0)),
                        ))
                    })
                    .collect();
                spans.push(Spans::from(Span::styled(
                    "Handy scripts launcher for project, servers and tooling",
                    Style::default()
                        .fg(Color::Rgb(255, 165, 0))
                        .add_modifier(Modifier::BOLD),
                )));
                return spans;
            }
        }
    }

    // Fallback static ASCII
    let ascii = [
        r"  ____    _    _ _     ____   ____  ",
        r" / ___|  / \  | | |   | __ ) | __ ) ",
        r"| |     / _ \ | | |   |  _ \ |  _ \ ",
        r"| |___ / ___ \| | |___| |_) || |_) |",
        r" \____/_/   \_\_|_____|____/ |____/",
    ];

    let mut spans: Vec<Spans> = ascii
        .iter()
        .map(|l| {
            Spans::from(Span::styled(
                l.to_string(),
                // use orange color to match spec
                Style::default().fg(Color::Rgb(255, 165, 0)),
            ))
        })
        .collect();
    spans.push(Spans::from(Span::styled(
        "Handy scripts launcher for project, servers and tooling",
        Style::default()
            .fg(Color::Rgb(255, 165, 0))
            .add_modifier(Modifier::BOLD),
    )));

    spans
}
