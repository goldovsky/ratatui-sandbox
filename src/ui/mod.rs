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

// Helper to build substituted command for action (column index, action index)
fn build_substituted_command(app: &App, c: usize, a: usize) -> String {
    let template = app.columns[c].actions[a].template.clone();
    let mut out = template.clone();
    for (pidx, param) in app.columns[c].actions[a].parameters.iter().enumerate() {
        let val = if param.param_type == crate::config::ParameterType::Select {
            let sel = app.param_selected[c][a][pidx];
            param
                .options
                .get(sel)
                .map(|o| o.value.clone())
                .unwrap_or_default()
        } else {
            app.param_values[c][a][pidx].clone()
        };
        out = out.replace(&param.placeholder, &val);
    }
    out
}

pub struct App {
    pub config: Config,
    pub columns: Vec<ColumnState>,
    pub focused_column: usize,
    // when true, the middle area shows the details view for the focused action
    pub show_details: bool,
    // Index of focused parameter within the details view when open
    pub details_focused_param: usize,
    // text edit mode state when editing a text parameter in the details view
    pub details_in_edit: bool,
    pub details_edit_buffer: String,
    pub details_edit_original: String,
    // For each column -> action -> parameter (when select), the selected option index
    // Layout: [column_idx][action_idx][param_idx] => usize (option index or 0)
    pub param_selected: Vec<Vec<Vec<usize>>>,
    // Current parameter values (strings) for substitution: [col][action][param]
    pub param_values: Vec<Vec<Vec<String>>>,
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
            config: config.clone(),
            columns,
            focused_column: 0,
            show_details: false,
            details_focused_param: 0,
            details_in_edit: false,
            details_edit_buffer: String::new(),
            details_edit_original: String::new(),
            // initialize param_selected to match config structure
            // for select parameters, prefer the parameter.default value when present
            param_selected: config
                .columns
                .iter()
                .map(|col| {
                    col.actions
                        .iter()
                        .map(|act| {
                            act.parameters
                                .iter()
                                .map(|p| {
                                    if p.param_type == crate::config::ParameterType::Select {
                                        if let Some(ref def) = p.default {
                                            // find index of option whose value matches default
                                            p.options
                                                .iter()
                                                .position(|o| &o.value == def)
                                                .unwrap_or(0)
                                        } else {
                                            0usize
                                        }
                                    } else {
                                        0usize
                                    }
                                })
                                .collect()
                        })
                        .collect()
                })
                .collect(),
            // initialize parameter values: for selects prefer parameter.default -> matching option value; else first option.
            param_values: config
                .columns
                .iter()
                .map(|col| {
                    col.actions
                        .iter()
                        .map(|act| {
                            act.parameters
                                .iter()
                                .enumerate()
                                .map(|(_pidx, p)| {
                                    if p.param_type == crate::config::ParameterType::Select {
                                        if let Some(ref def) = p.default {
                                            p.options
                                                .iter()
                                                .find(|o| &o.value == def)
                                                .map(|o| o.value.clone())
                                                .or_else(|| {
                                                    p.options.get(0).map(|o| o.value.clone())
                                                })
                                                .unwrap_or_default()
                                        } else {
                                            p.options
                                                .get(0)
                                                .map(|o| o.value.clone())
                                                .unwrap_or_default()
                                        }
                                    } else {
                                        p.default.clone().unwrap_or_default()
                                    }
                                })
                                .collect()
                        })
                        .collect()
                })
                .collect(),
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

    // removed unused focused_selection

    fn focused_action(&self) -> Option<&Action> {
        self.columns
            .get(self.focused_column)
            .and_then(|col| col.list_state.selected().and_then(|i| col.actions.get(i)))
    }

    fn focused_action_index(&self) -> Option<(usize, usize)> {
        if let Some(col) = self.columns.get(self.focused_column) {
            if let Some(act_idx) = col.list_state.selected() {
                return Some((self.focused_column, act_idx));
            }
        }
        None
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

                // Use the action label as the window title when available. Add a leading
                // and trailing space for visual padding.
                let title_text = app
                    .focused_action()
                    .map(|a| format!(" {} ", a.label))
                    .unwrap_or_else(|| " Details ".to_string());

                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(title_text.as_str(), Style::default().add_modifier(Modifier::BOLD)));
                f.render_widget(block, area);

                let inner = Rect {
                    x: area.x + 1,
                    y: area.y + 1,
                    width: area.width.saturating_sub(2),
                    height: area.height.saturating_sub(2),
                };

                // Build detailed content from the focused action (parameters only)
                let mut lines: Vec<Spans> = Vec::new();

                if let Some(action) = app.focused_action() {
                    if !action.parameters.is_empty() {
                        lines.push(Spans::from(Span::styled(
                            "Parameters:",
                            Style::default().add_modifier(Modifier::BOLD),
                        )));

                for (idx, param) in action.parameters.iter().enumerate() {
                            let required_marker = if param.required { " *" } else { "" };

                            // Parameter header line; omit type suffix for selects
                            let mut spans = vec![Span::raw("  "), Span::styled(&param.name, Style::default().fg(Color::Yellow))];
                            if param.param_type == crate::config::ParameterType::Select {
                                spans.push(Span::raw(format!("{}  ", required_marker)));
                            } else {
                                spans.push(Span::raw(format!(" {}  ", required_marker)));
                            }

                            // If select, render options inline with highlight for selected
                            if param.param_type == crate::config::ParameterType::Select {
                                if let Some((c, a)) = app.focused_action_index() {
                                    let sel = app.param_selected[c][a][idx];
                                    // Render options on a separate line under the parameter
                                    lines.push(Spans::from(vec![Span::raw("    ")]));
                                    let mut opt_spans: Vec<Span> = Vec::new();
                                    for (oi, opt) in param.options.iter().enumerate() {
                                        // color mapping for environment-like options
                                        let styled = match opt.value.as_str() {
                                            "qlf" => Style::default().fg(Color::Green),
                                            "pprod" | "pprod_legacy" => Style::default().fg(Color::Rgb(255, 165, 0)),
                                            v if v.starts_with("prod") => Style::default().fg(Color::Red),
                                            _ => Style::default(),
                                        };

                                        if oi == sel {
                                            // selected: bold + distinct fg
                                            opt_spans.push(Span::styled(format!("[{}] ", opt.label), styled.add_modifier(Modifier::BOLD)));
                                        } else {
                                            opt_spans.push(Span::styled(format!(" {}  ", opt.label), styled));
                                        }
                                    }
                                    lines.push(Spans::from(opt_spans));
                                }
                            } else {
                                // for text params, show current value
                                if let Some((c, a)) = app.focused_action_index() {
                                    let val = app.param_values[c][a][idx].clone();
                                    spans.push(Span::raw(format!(": {}", val)));
                                }
                            }

                            // indicate focus with a pointer glyph on the start of the line
                            if idx == app.details_focused_param {
                                let pointer_style = if app.details_in_edit { Style::default().fg(Color::Yellow).bg(Color::Rgb(40,40,40)) } else { Style::default().fg(Color::Yellow) };
                                let mut row = vec![Span::styled("➜ ", pointer_style)];
                                row.extend(spans);
                                lines.push(Spans::from(row));
                            } else {
                                lines.push(Spans::from(spans));
                            }

                            if let Some(ref desc) = param.description {
                                lines.push(Spans::from(vec![
                                    Span::raw("    "),
                                    Span::styled(desc, Style::default().fg(Color::Rgb(150, 150, 150))),
                                ]));
                            }
                        }
                    } else {
                        lines.push(Spans::from(Span::raw("No parameters")));
                    }
                } else {
                    lines.push(Spans::from(Span::raw("No action selected")));
                }

                lines.push(Spans::from(Span::raw("")));
                lines.push(Spans::from(Span::styled(
                    " Press Enter to run or Esc to return to the main page ",
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
            // Build preview_line by substituting parameter placeholders with current values
            let mut preview_line = String::new();
                    if let Some((c, a)) = app.focused_action_index() {
                        preview_line = build_substituted_command(&app, c, a);
                    }

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
                // If we're in text edit mode, handle editing keys separately
                if app.details_in_edit {
                    if let Some((c, a)) = app.focused_action_index() {
                        let pidx = app.details_focused_param;
                        match key.code {
                            KeyCode::Char(ch) => {
                                // append character to buffer and update param_values
                                app.details_edit_buffer.push(ch);
                                app.param_values[c][a][pidx] = app.details_edit_buffer.clone();
                            }
                            KeyCode::Backspace => {
                                app.details_edit_buffer.pop();
                                app.param_values[c][a][pidx] = app.details_edit_buffer.clone();
                            }
                            KeyCode::Enter => {
                                // accept edit
                                app.details_in_edit = false;
                                app.details_edit_original.clear();
                                app.details_edit_buffer.clear();
                            }
                            KeyCode::Esc => {
                                // cancel edit, revert original value
                                app.param_values[c][a][pidx] = app.details_edit_original.clone();
                                app.details_in_edit = false;
                                app.details_edit_buffer.clear();
                                app.details_edit_original.clear();
                            }
                            _ => {}
                        }
                    }
                    continue;
                }

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
                        if app.show_details {
                            if app.details_focused_param > 0 {
                                app.details_focused_param -= 1;
                            }
                        } else {
                            app.move_up()
                        }
                    }
                    KeyCode::Down => {
                        if app.show_details {
                            // move to next parameter if available
                            if let Some((_c, a)) = app.focused_action_index() {
                                let params_len =
                                    app.columns[app.focused_column].actions[a].parameters.len();
                                if app.details_focused_param + 1 < params_len {
                                    app.details_focused_param += 1;
                                }
                            }
                        } else {
                            app.move_down()
                        }
                    }
                    KeyCode::Left => {
                        if app.show_details {
                            if let Some((c, a)) = app.focused_action_index() {
                                if let Some(param) = app.columns[c].actions[a]
                                    .parameters
                                    .get(app.details_focused_param)
                                {
                                    if param.param_type == crate::config::ParameterType::Select {
                                        let opts_len = param.options.len();
                                        if opts_len > 0 {
                                            let cur = &mut app.param_selected[c][a]
                                                [app.details_focused_param];
                                            if *cur > 0 {
                                                *cur -= 1;
                                                // sync param_values with new selection
                                                app.param_values[c][a][app.details_focused_param] =
                                                    param.options[*cur].value.clone();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Right => {
                        if app.show_details {
                            if let Some((c, a)) = app.focused_action_index() {
                                if let Some(param) = app.columns[c].actions[a]
                                    .parameters
                                    .get(app.details_focused_param)
                                {
                                    if param.param_type == crate::config::ParameterType::Select {
                                        let opts_len = param.options.len();
                                        if opts_len > 0 {
                                            let cur = &mut app.param_selected[c][a]
                                                [app.details_focused_param];
                                            if *cur + 1 < opts_len {
                                                *cur += 1;
                                                // sync param_values with new selection
                                                app.param_values[c][a][app.details_focused_param] =
                                                    param.options[*cur].value.clone();
                                            }
                                        }
                                    }
                                }
                            }
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
                        // If details view is not shown, open it. If it is shown and the
                        // focused parameter is text, enter edit mode. Otherwise toggle details.
                        if !app.show_details {
                            app.show_details = true;
                        } else if let Some((c, a)) = app.focused_action_index() {
                            if let Some(param) = app.columns[c].actions[a]
                                .parameters
                                .get(app.details_focused_param)
                            {
                                if param.param_type == crate::config::ParameterType::Text {
                                    // enter edit mode
                                    app.details_in_edit = true;
                                    app.details_edit_original =
                                        app.param_values[c][a][app.details_focused_param].clone();
                                    app.details_edit_buffer = app.details_edit_original.clone();
                                } else {
                                    // non-text: no-op for Enter while in details
                                }
                            }
                        }
                    }
                    KeyCode::Esc => {
                        // close details view if open
                        if app.show_details {
                            app.show_details = false;
                        }
                    }
                    KeyCode::Char('r') => {
                        // when details are shown, run the substituted command
                        if app.show_details {
                            if let Some((c, a)) = app.focused_action_index() {
                                let cmd = build_substituted_command(&app, c, a);
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

// removed old modal preview helper

// removed centered_rect helper
