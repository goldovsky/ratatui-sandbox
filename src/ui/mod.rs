use crossterm::event::{self, Event, KeyCode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Span, Spans};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
mod title;
use std::io;
use std::time::{Duration, Instant};
use title::title_spans;

use crate::config::{Action, Config};
use crate::runner::run_command;

/// Column state: tracks selection within a column
pub struct ColumnState {
    pub title: String,
    pub actions: Vec<Action>,
    pub list_state: ListState,
}

pub struct App {
    pub config: Config,
    pub columns: Vec<ColumnState>,
    pub focused_column: usize,
    // when true, the middle area shows the details view for the focused action
    pub show_details: bool,
}

impl App {
    pub fn new(config: Config) -> Self {
        let columns: Vec<ColumnState> = config
            .columns
            .iter()
            .map(|col| {
                let mut ls = ListState::default();
                if col.actions.is_empty() {
                    ls.select(None);
                } else {
                    ls.select(Some(0));
                }
                ColumnState {
                    title: col.title.clone(),
                    actions: col.actions.clone(),
                    list_state: ls,
                }
            })
            .collect();

        Self {
            config,
            columns,
            focused_column: 0,
            show_details: false,
        }
    }

    fn move_up(&mut self) {
        if let Some(col) = self.columns.get_mut(self.focused_column) {
            if let Some(curr) = col.list_state.selected() {
                if curr > 0 {
                    let new = curr - 1;
                    col.list_state.select(Some(new));
                }
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(col) = self.columns.get_mut(self.focused_column) {
            if let Some(curr) = col.list_state.selected() {
                if curr + 1 < col.actions.len() {
                    let new = curr + 1;
                    col.list_state.select(Some(new));
                }
            }
        }
    }

    fn focused_selection(&self) -> (String, usize) {
        if let Some(col) = self.columns.get(self.focused_column) {
            if let Some(idx) = col.list_state.selected() {
                if let Some(action) = col.actions.get(idx) {
                    return (action.label.clone(), idx);
                }
            }
        }
        ("".into(), 0)
    }

    fn focused_action(&self) -> Option<&Action> {
        self.columns
            .get(self.focused_column)
            .and_then(|col| col.list_state.selected().and_then(|i| col.actions.get(i)))
    }

    fn column_count(&self) -> usize {
        self.columns.len()
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

            // Obtain the title lines (figlet or fallback) so we can size the top (header) chunk
            let title_lines = title_spans(&app.config.app.title);
            // reserve one extra row for the subtitle we append below
            let title_height = (title_lines.len() as u16).saturating_add(1).max(3);

            // Layout: header (title + subtitle), middle (columns or details), footer (preview + help)
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

            // Build header content: figlet lines, subtitle and a blank line below
            let mut title_body: Vec<Spans> = Vec::new();
            title_body.extend(title_lines.clone());
            // subtitle from config
            title_body.push(Spans::from(Span::styled(
                app.config.app.subtitle.clone(),
                Style::default().fg(Color::Rgb(150, 150, 150)),
            )));
            // one empty row below subtitle
            title_body.push(Spans::from(Span::raw("")));

            let header = Paragraph::new(title_body).alignment(Alignment::Center);
            f.render_widget(header, chunks[0]);

            // Middle area: either the columns or a details view depending on state
            if !app.show_details {
                // Columns layout - dynamic based on config
                let num_columns = app.column_count();
                let column_constraints: Vec<Constraint> = (0..num_columns)
                    .map(|_| Constraint::Ratio(1, num_columns as u32))
                    .collect();

                let middle_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(column_constraints)
                    .split(chunks[1]);

                // Render each column dynamically
                for col_idx in 0..app.columns.len() {
                    // snapshot small bits so we don't keep immutable borrows while taking a
                    // mutable borrow for the ListState below
                    let actions = app.columns[col_idx].actions.clone();
                    let title_text = app.columns[col_idx].title.clone();
                    let focused = app.focused_column == col_idx;

                    let items: Vec<ListItem> = actions
                        .iter()
                        .enumerate()
                        .map(|(_i, action)| {
                            let content = vec![Spans::from(Span::raw(format!("  {}  ", action.label)))];
                            ListItem::new(content)
                        })
                        .collect();

                    let col_title = {
                        let inner = middle_chunks[col_idx].width as usize;
                        let core = &title_text;
                        if inner > core.len() + 2 {
                            format!(" {} ", core)
                        } else {
                            core.clone()
                        }
                    };

                    let mut list = List::new(items)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(Span::styled(
                                    col_title,
                                    Style::default().add_modifier(Modifier::BOLD),
                                ))
                                .title_alignment(Alignment::Center),
                        )
                        // highlight the selected item; visually stronger when focused
                        .highlight_style(if focused {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Rgb(150, 150, 150))
                        });

                    if focused {
                        list = list.highlight_symbol("► ");
                    } else {
                        list = list.highlight_symbol("  ");
                    }

                    // render statefully so the List will scroll to keep the selected item visible
                    f.render_stateful_widget(
                        list,
                        middle_chunks[col_idx],
                        &mut app.columns[col_idx].list_state,
                    );
                }
            } else {
                // Details view replaces the columns in the middle area while keeping header/footer
                let area = chunks[1];
                let block = Block::default().borders(Borders::ALL).title(Span::styled(
                    " Details ",
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                f.render_widget(block, area);

                let inner = Rect {
                    x: area.x + 1,
                    y: area.y + 1,
                    width: area.width.saturating_sub(2),
                    height: area.height.saturating_sub(2),
                };

                // Build detailed content from the focused action
                let mut lines: Vec<Spans> = Vec::new();

                if let Some(action) = app.focused_action() {
                    // Action label
                    lines.push(Spans::from(vec![
                        Span::styled("Action: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(&action.label),
                    ]));
                    lines.push(Spans::from(Span::raw("")));

                    // Description if present
                    if let Some(ref desc) = action.description {
                        lines.push(Spans::from(vec![
                            Span::styled(
                                "Description: ",
                                Style::default().add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(desc),
                        ]));
                        lines.push(Spans::from(Span::raw("")));
                    }

                    // Command template
                    lines.push(Spans::from(vec![
                        Span::styled("Command: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(&action.template, Style::default().fg(Color::Cyan)),
                    ]));
                    lines.push(Spans::from(Span::raw("")));

                    // Parameters
                    if !action.parameters.is_empty() {
                        lines.push(Spans::from(Span::styled(
                            "Parameters:",
                            Style::default().add_modifier(Modifier::BOLD),
                        )));

                        for param in &action.parameters {
                            let required_marker = if param.required { " *" } else { "" };
                            let param_type = format!("{:?}", param.param_type).to_lowercase();

                            lines.push(Spans::from(vec![
                                Span::raw("  "),
                                Span::styled(&param.name, Style::default().fg(Color::Yellow)),
                                Span::raw(format!(" ({}){}", param_type, required_marker)),
                            ]));

                            if let Some(ref desc) = param.description {
                                lines.push(Spans::from(vec![
                                    Span::raw("    "),
                                    Span::styled(desc, Style::default().fg(Color::Rgb(150, 150, 150))),
                                ]));
                            }
                        }
                    }
                } else {
                    lines.push(Spans::from(Span::raw("No action selected")));
                }

                lines.push(Spans::from(Span::raw("")));
                lines.push(Spans::from(Span::styled(
                    "Press Enter or Esc to close | r: Run",
                    Style::default().fg(Color::Rgb(100, 100, 100)),
                )));

                let text = Paragraph::new(lines)
                    .alignment(Alignment::Left)
                    .wrap(Wrap { trim: true });
                f.render_widget(text, inner);
            }

            // Footer area: preview + help. Always present even when details are shown
            let bottom_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Length(3)].as_ref())
                .split(chunks[2]);

            // show the action template in the preview
            let preview_line = app
                .focused_action()
                .map(|a| a.template.clone())
                .unwrap_or_default();

            // Draw bordered preview and render a single-line paragraph inside
            let preview_area = bottom_chunks[0];
            let block = Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    " Preview ",
                    Style::default().add_modifier(Modifier::BOLD),
                ))
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
                Span::raw(preview_line.clone()),
                Span::raw("  "),
            ])])
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false });
            f.render_widget(inner_para, inner);

            // Help bar content
            let help_text =
                "Tab: switch column   Up/Down: navigate   Enter: details   r:Run   q: quit | *: Optional";

            // If the help area is tall enough, render a bordered block and draw the
            // help text inside the block inner rect. Otherwise render the help line
            // directly (no border) so it remains visible on small terminals.
            let help_area = bottom_chunks[1];
            if help_area.height >= 3 {
                let block = Block::default().borders(Borders::ALL).title(Span::styled(
                    " Help ",
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                f.render_widget(block, help_area);

                let inner = Rect {
                    x: help_area.x + 1,
                    y: help_area.y + 1,
                    width: help_area.width.saturating_sub(2),
                    height: help_area.height.saturating_sub(2),
                };
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
                    Span::styled(help_text, Style::default().fg(Color::Rgb(150, 150, 150))),
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
                    KeyCode::Tab => {
                        // Only switch columns when details view is not open
                        if !app.show_details {
                            let num_cols = app.column_count();
                            if num_cols > 0 {
                                app.focused_column = (app.focused_column + 1) % num_cols;
                            }
                        }
                    }
                    KeyCode::Up => {
                        if !app.show_details {
                            app.move_up()
                        }
                    }
                    KeyCode::Down => {
                        if !app.show_details {
                            app.move_down()
                        }
                    }
                    KeyCode::PageUp => {
                        // When details view is open, PageUp is reserved for details navigation;
                        // ignore it here so the columns don't change.
                        if !app.show_details {
                            // move up by one page in the focused column
                            let size = terminal.size()?;
                            let title_lines = title_spans(&app.config.app.title);
                            let title_height = (title_lines.len() as u16).saturating_add(1).max(3);
                            // account for outer margin (1 top + 1 bottom)
                            let middle_height = size
                                .height
                                .saturating_sub(2)
                                .saturating_sub(title_height)
                                .saturating_sub(6);
                            let page = middle_height.saturating_sub(2).max(1) as usize; // inner height minus block borders

                            if let Some(col) = app.columns.get_mut(app.focused_column) {
                                if !col.actions.is_empty() {
                                    if let Some(curr) = col.list_state.selected() {
                                        let new = curr.saturating_sub(page);
                                        col.list_state.select(Some(new));
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::PageDown => {
                        // When details view is open, PageDown is reserved; ignore here
                        if !app.show_details {
                            // move down by one page in the focused column
                            let size = terminal.size()?;
                            let title_lines = title_spans(&app.config.app.title);
                            let title_height = (title_lines.len() as u16).saturating_add(1).max(3);
                            let middle_height = size
                                .height
                                .saturating_sub(2)
                                .saturating_sub(title_height)
                                .saturating_sub(6);
                            let page = middle_height.saturating_sub(2).max(1) as usize;

                            if let Some(col) = app.columns.get_mut(app.focused_column) {
                                if !col.actions.is_empty() {
                                    if let Some(curr) = col.list_state.selected() {
                                        let new =
                                            (curr + page).min(col.actions.len().saturating_sub(1));
                                        col.list_state.select(Some(new));
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Home => {
                        // jump to top (only when not showing details)
                        if !app.show_details {
                            if let Some(col) = app.columns.get_mut(app.focused_column) {
                                if !col.actions.is_empty() {
                                    col.list_state.select(Some(0));
                                }
                            }
                        }
                    }
                    KeyCode::End => {
                        // jump to bottom (only when not showing details)
                        if !app.show_details {
                            if let Some(col) = app.columns.get_mut(app.focused_column) {
                                if !col.actions.is_empty() {
                                    col.list_state
                                        .select(Some(col.actions.len().saturating_sub(1)));
                                }
                            }
                        }
                    }
                    KeyCode::Enter => {
                        // toggle details view in the middle area (header/footer remain)
                        app.show_details = !app.show_details;
                    }
                    KeyCode::Esc => {
                        // close details view if open
                        if app.show_details {
                            app.show_details = false;
                        }
                    }
                    KeyCode::Char('r') => {
                        // when details are shown, allow Run
                        if app.show_details {
                            if let Some(cmd) = app.focused_action().map(|a| a.template.clone()) {
                                let _ = run_command(terminal, &cmd);
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
            let area = centered_rect(70, 60, size);
            let block = Block::default().borders(Borders::ALL).title(Span::styled(
                " Details ",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            f.render_widget(block, area);

            let inner = Rect {
                x: area.x + 1,
                y: area.y + 1,
                width: area.width.saturating_sub(2),
                height: area.height.saturating_sub(2),
            };

            // Build detailed content from the focused action
            let mut lines: Vec<Spans> = Vec::new();

            if let Some(action) = app.focused_action() {
                // Action label
                lines.push(Spans::from(vec![
                    Span::styled("Action: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&action.label),
                ]));
                lines.push(Spans::from(Span::raw("")));

                // Description if present
                if let Some(ref desc) = action.description {
                    lines.push(Spans::from(vec![
                        Span::styled(
                            "Description: ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(desc),
                    ]));
                    lines.push(Spans::from(Span::raw("")));
                }

                // Command template
                lines.push(Spans::from(vec![
                    Span::styled("Command: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(&action.template, Style::default().fg(Color::Cyan)),
                ]));
                lines.push(Spans::from(Span::raw("")));

                // Parameters
                if !action.parameters.is_empty() {
                    lines.push(Spans::from(Span::styled(
                        "Parameters:",
                        Style::default().add_modifier(Modifier::BOLD),
                    )));

                    for param in &action.parameters {
                        let required_marker = if param.required { " *" } else { "" };
                        let param_type = format!("{:?}", param.param_type).to_lowercase();

                        lines.push(Spans::from(vec![
                            Span::raw("  "),
                            Span::styled(&param.name, Style::default().fg(Color::Yellow)),
                            Span::raw(format!(" ({}){}", param_type, required_marker)),
                        ]));

                        if let Some(ref desc) = param.description {
                            lines.push(Spans::from(vec![
                                Span::raw("    "),
                                Span::styled(desc, Style::default().fg(Color::Rgb(150, 150, 150))),
                            ]));
                        }
                    }
                }
            } else {
                lines.push(Spans::from(Span::raw("No action selected")));
            }

            lines.push(Spans::from(Span::raw("")));
            lines.push(Spans::from(Span::styled(
                "Press Enter or Esc to close",
                Style::default().fg(Color::Rgb(100, 100, 100)),
            )));

            let text = Paragraph::new(lines)
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
