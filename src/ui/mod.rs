use crossterm::event::{self, Event, KeyCode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Span, Spans};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Terminal;
mod title;
use std::io;
use std::time::{Duration, Instant};
use title::title_spans;

use crate::runner::{dry_run_command, run_command};

pub struct App {
    pub project_actions: Vec<String>,
    pub server_actions: Vec<String>,
    pub tools_actions: Vec<String>,
    pub project_selected: usize,
    pub server_selected: usize,
    pub tools_selected: usize,
    pub focused_column: usize, // 0 project, 1 server, 2 tools
}

impl App {
    pub fn new() -> Self {
        Self {
            project_actions: vec![
                "New Project".into(),
                "Open Project".into(),
                "Build".into(),
                "Test".into(),
            ],
            server_actions: vec![
                "Start Dev Server".into(),
                "Stop Server".into(),
                "Restart".into(),
                "Logs".into(),
            ],
            tools_actions: vec!["Simulate Call".into(), "Lint".into(), "Format".into()],
            project_selected: 0,
            server_selected: 0,
            tools_selected: 0,
            focused_column: 0,
        }
    }

    fn move_up(&mut self) {
        match self.focused_column {
            0 => {
                if self.project_selected > 0 {
                    self.project_selected -= 1;
                }
            }
            1 => {
                if self.server_selected > 0 {
                    self.server_selected -= 1;
                }
            }
            2 => {
                if self.tools_selected > 0 {
                    self.tools_selected -= 1;
                }
            }
            _ => {}
        }
    }

    fn move_down(&mut self) {
        match self.focused_column {
            0 => {
                if self.project_selected + 1 < self.project_actions.len() {
                    self.project_selected += 1;
                }
            }
            1 => {
                if self.server_selected + 1 < self.server_actions.len() {
                    self.server_selected += 1;
                }
            }
            2 => {
                if self.tools_selected + 1 < self.tools_actions.len() {
                    self.tools_selected += 1;
                }
            }
            _ => {}
        }
    }

    fn focused_selection(&self) -> (String, usize) {
        match self.focused_column {
            0 => (
                self.project_actions[self.project_selected].clone(),
                self.project_selected,
            ),
            1 => (
                self.server_actions[self.server_selected].clone(),
                self.server_selected,
            ),
            2 => (
                self.tools_actions[self.tools_selected].clone(),
                self.tools_selected,
            ),
            _ => ("".into(), 0),
        }
    }
}

pub fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    mut app: App,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| {
            let size = f.size();

            // Obtain the title lines (figlet or fallback) so we can size the top chunk
            let title_lines = title_spans();
            // reserve one extra row for the subtitle we append below
            let title_height = (title_lines.len() as u16).saturating_add(1).max(3);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(title_height),
                        Constraint::Min(10),
                    Constraint::Length(6),
                    ]
                    .as_ref(),
                )
                .split(size);

            // To ensure the subtitle is always visible and positioned right below the
            // figlet output, render the figlet lines then the subtitle then a blank line.
            let mut title_body: Vec<Spans> = Vec::new();
            title_body.extend(title_lines.clone());
            // subtitle
            title_body.push(Spans::from(Span::styled(
                "Handy scripts launcher for project, servers and tooling",
                Style::default().fg(Color::Rgb(150, 150, 150)),
            )));
            // one empty row below subtitle
            title_body.push(Spans::from(Span::raw("")));

            let title = Paragraph::new(title_body).alignment(Alignment::Center);
            f.render_widget(title, chunks[0]);

            // Middle columns
            let middle_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(33),
                        Constraint::Percentage(34),
                        Constraint::Percentage(33),
                    ]
                    .as_ref(),
                )
                .split(chunks[1]);

            // Project column
            let project_items: Vec<ListItem> = app
                .project_actions
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    // add left/right padding inside the list so items don't touch the border
                    let content = vec![Spans::from(Span::raw(format!("  {}  ", a.clone())))];
                    ListItem::new(content).style(
                        if app.focused_column == 0 && app.project_selected == i {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        },
                    )
                })
                .collect();

            // Use short literal title and let Block center it via title_alignment so
            // the top border's horizontal lines remain visible.
            let project_title = {
                let inner = middle_chunks[0].width as usize;
                let core = "Projects";
                if inner > core.len() + 2 {
                    format!(" {} ", core)
                } else {
                    core.to_string()
                }
            };
            let project_list = List::new(project_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        project_title,
                        Style::default().add_modifier(Modifier::BOLD),
                    ))
                    .title_alignment(Alignment::Center),
            );
            f.render_widget(project_list, middle_chunks[0]);

            // Server column
            let server_items: Vec<ListItem> = app
                .server_actions
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let content = vec![Spans::from(Span::raw(format!("  {}  ", a.clone())))];
                    ListItem::new(content).style(
                        if app.focused_column == 1 && app.server_selected == i {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        },
                    )
                })
                .collect();

            let server_title = {
                let inner = middle_chunks[1].width as usize;
                let core = "Servers";
                if inner > core.len() + 2 {
                    format!(" {} ", core)
                } else {
                    core.to_string()
                }
            };
            let server_list = List::new(server_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        server_title,
                        Style::default().add_modifier(Modifier::BOLD),
                    ))
                    .title_alignment(Alignment::Center),
            );
            f.render_widget(server_list, middle_chunks[1]);

            // Tools column
            let tools_items: Vec<ListItem> = app
                .tools_actions
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let content = vec![Spans::from(Span::raw(format!("  {}  ", a.clone())))];
                    ListItem::new(content).style(
                        if app.focused_column == 2 && app.tools_selected == i {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        },
                    )
                })
                .collect();

            let tools_title = {
                let inner = middle_chunks[2].width as usize;
                let core = "Tools";
                if inner > core.len() + 2 {
                    format!(" {} ", core)
                } else {
                    core.to_string()
                }
            };
            let tools_list = List::new(tools_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        tools_title,
                        Style::default().add_modifier(Modifier::BOLD),
                    ))
                    .title_alignment(Alignment::Center),
            );
            f.render_widget(tools_list, middle_chunks[2]);

            // Bottom preview and help
            // Reserve 3 rows for the preview (border + 1 inner line + border)
            // and 3 rows for the help bar so the preview keeps its border and title
            let bottom_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Length(3)].as_ref())
                .split(chunks[2]);

            let (sel_text, _sel_index) = app.focused_selection();
            // show only the previewed command (no redundant 'Preview:' label)
            let preview_line = format!("  {}  ", sel_text);

            // Draw bordered preview and render a single-line paragraph inside
            let preview_area = bottom_chunks[0];
            let block = Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(" Preview ", Style::default().add_modifier(Modifier::BOLD)))
                .title_alignment(Alignment::Left);
            f.render_widget(block, preview_area);

            let inner = Rect {
                x: preview_area.x + 1,
                y: preview_area.y + 1,
                width: preview_area.width.saturating_sub(2),
                // force a single-line inner area so only one row is displayed
                height: 1,
            };
            let inner_para = Paragraph::new(vec![Spans::from(vec![
                Span::raw("  "),
                Span::raw(sel_text.clone()),
                Span::raw("  "),
            ])])
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false });
            f.render_widget(inner_para, inner);

            // Single-line concise help bar
            let help_line = Spans::from(Span::styled(
                "↹  switch column   ↑ /↓  navigate   Enter: details (e:Echo r:Run)   q: quit   Esc: close",
                Style::default().fg(Color::Rgb(150, 150, 150)),
            ));
            let help = Paragraph::new(help_line).alignment(Alignment::Left);

            // If the help area is tall enough, render a bordered block and draw the
            // help text inside the block inner rect. Otherwise render the help line
            // directly (no border) so it remains visible on small terminals.
            let help_area = bottom_chunks[1];
            if help_area.height >= 3 {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(" Help ", Style::default().add_modifier(Modifier::BOLD)));
                f.render_widget(block, help_area);

                let inner = Rect {
                    x: help_area.x + 1,
                    y: help_area.y + 1,
                    width: help_area.width.saturating_sub(2),
                    height: help_area.height.saturating_sub(2),
                };
                let help_text = "↹  switch column   ↑ /↓  navigate   Enter: details (e:Echo r:Run)   q: quit   Esc: close";
                let inner_para = Paragraph::new(vec![Spans::from(vec![
                    Span::raw("  "),
                    Span::styled(help_text, Style::default().fg(Color::Rgb(150, 150, 150))),
                    Span::raw("  "),
                ])])
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false });
                f.render_widget(inner_para, inner);
            } else {
                // cramped: render help text plainly so it's visible
                let compact = Paragraph::new(vec![Spans::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "↹  switch column   ↑ /↓  navigate   Enter: details (e:Echo r:Run)   q: quit   Esc: close",
                        Style::default().fg(Color::Rgb(150, 150, 150)),
                    ),
                    Span::raw("  "),
                ])])
                .alignment(Alignment::Left);
                f.render_widget(compact, help_area);
            }
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Tab => app.focused_column = (app.focused_column + 1) % 3,
                    KeyCode::Up => app.move_up(),
                    KeyCode::Down => app.move_down(),
                    KeyCode::Enter => {
                        // show modal preview
                        let (sel_text, _sel_index) = app.focused_selection();
                        let command = sel_text; // placeholder: in future build full command string

                        // show preview modal
                        show_preview(terminal, &app)?;

                        // after preview, simple prompt: Echo (dry-run) on 'e', Run on 'r'
                        terminal.draw(|f| {
                            let size = f.size();
                            let msg = Paragraph::new(Spans::from(Span::raw(
                                "Press 'e' to Echo (dry-run), 'r' to Run, any other key to cancel",
                            )));
                            f.render_widget(msg, size);
                        })?;

                        if crossterm::event::poll(Duration::from_millis(5000))? {
                            if let Event::Key(k) = event::read()? {
                                match k.code {
                                    KeyCode::Char('e') => {
                                        let _ = dry_run_command(terminal, &command);
                                    }
                                    KeyCode::Char('r') => {
                                        let _ = run_command(terminal, &command);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn show_preview(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            let size = f.size();
            let area = centered_rect(60, 40, size);
            let block = Block::default().borders(Borders::ALL).title(Span::styled(
                " Details ",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            f.render_widget(block, area);

            let inner = Rect {
                x: area.x + 1,
                y: area.y + 1,
                width: area.width - 2,
                height: area.height - 2,
            };
            let (sel_text, _sel_index) = app.focused_selection();
            let text = Paragraph::new(Spans::from(vec![Span::raw(format!(
                "Detailed preview for: {}",
                sel_text
            ))]))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });
            f.render_widget(text, inner);
        })?;

        if crossterm::event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    let vertical = popup_layout[1];

    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical);

    horizontal_layout[1]
}
