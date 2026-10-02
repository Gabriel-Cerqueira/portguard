//! Terminal user interface renderer using Ratatui.

use crate::app::{App, AppMode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table,
    },
    Frame,
};

/// Renders the complete TUI frame.
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    let has_filter = app.mode == AppMode::Filtering || !app.filter_input.is_empty();
    
    let constraints = if has_filter {
        vec![
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ]
    } else {
        vec![
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ]
    };

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(size);

    let mut chunk_idx = 0;

    render_header(frame, app, main_chunks[chunk_idx]);
    chunk_idx += 1;

    if has_filter {
        render_filter_bar(frame, app, main_chunks[chunk_idx]);
        chunk_idx += 1;
    }

    render_body(frame, app, main_chunks[chunk_idx]);
    chunk_idx += 1;

    render_footer(frame, app, main_chunks[chunk_idx]);

    match app.mode {
        AppMode::ConfirmKill => render_confirm_kill_modal(frame, app),
        AppMode::Help => render_help_modal(frame),
        _ => {}
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let mode_label = if app.show_all_connections {
        Span::styled(" [ALL CONNECTIONS] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" [LISTENING] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    };

    let total_ram: u64 = app
        .filtered_ports
        .iter()
        .filter_map(|p| p.process.as_ref().map(|proc| proc.memory_bytes))
        .sum();

    let formatted_total_ram = {
        let mb = total_ram as f64 / (1024.0 * 1024.0);
        if mb >= 1024.0 {
            format!("{:.2} GB", mb / 1024.0)
        } else {
            format!("{:.1} MB", mb)
        }
    };

    let title_line = Line::from(vec![
        Span::styled(" PortGuard ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("v0.1.0", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        mode_label,
        Span::raw(" | "),
        Span::styled(format!("Ports: {}", app.filtered_ports.len()), Style::default().fg(Color::White)),
        Span::raw(" | "),
        Span::styled(format!("Total Dev RAM: {}", formatted_total_ram), Style::default().fg(Color::Magenta)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let header_paragraph = Paragraph::new(title_line).block(block);
    frame.render_widget(header_paragraph, area);
}

fn render_filter_bar(frame: &mut Frame, app: &App, area: Rect) {
    let is_editing = app.mode == AppMode::Filtering;
    let border_color = if is_editing { Color::Yellow } else { Color::DarkGray };

    let text = if app.filter_input.is_empty() && is_editing {
        "Filter by port, PID, process name or tag... (Press Enter or Esc to exit)"
    } else {
        &app.filter_input
    };

    let filter_text = Line::from(vec![
        Span::styled("Filter: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(text, if is_editing && app.filter_input.is_empty() { Style::default().fg(Color::DarkGray) } else { Style::default().fg(Color::White) }),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let filter_p = Paragraph::new(filter_text).block(block);
    frame.render_widget(filter_p, area);
}

fn render_body(frame: &mut Frame, app: &mut App, area: Rect) {
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(area);

    render_ports_table(frame, app, body_chunks[0]);
    render_process_details(frame, app, body_chunks[1]);
}

fn render_ports_table(frame: &mut Frame, app: &mut App, area: Rect) {
    let header_cells = ["PORT", "PROTO", "PID", "PROCESS", "RAM", "TAG", "STATE"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = app
        .filtered_ports
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (proc_name, ram) = match &p.process {
                Some(proc) => (proc.name.as_str(), proc.formatted_memory()),
                None => ("-", "-".to_string()),
            };

            let tag = p.dev_tag.unwrap_or("-");

            let cells = vec![
                Cell::from(p.port.to_string()),
                Cell::from(p.protocol.to_string()),
                Cell::from(p.pid.to_string()),
                Cell::from(proc_name),
                Cell::from(ram),
                Cell::from(tag),
                Cell::from(p.state.to_string()),
            ];

            let row_style = if i % 2 == 0 {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Rgb(200, 200, 200))
            };

            Row::new(cells).style(row_style)
        })
        .collect();

    let widths = [
        Constraint::Length(7),
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Length(18),
        Constraint::Length(10),
        Constraint::Length(14),
        Constraint::Min(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(30, 60, 110))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .title(" Active Ports ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_process_details(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Process Details ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let content = if let Some(p) = app.selected_port() {
        let mut lines = Vec::new();

        lines.push(Line::from(vec![
            Span::styled("Local Port:   ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} ({})", p.port, p.protocol), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("IP Address:   ", Style::default().fg(Color::Yellow)),
            Span::raw(&p.local_ip),
        ]));

        lines.push(Line::from(vec![
            Span::styled("State:        ", Style::default().fg(Color::Yellow)),
            Span::styled(p.state.to_string(), Style::default().fg(Color::Green)),
        ]));

        if let Some(tag) = p.dev_tag {
            lines.push(Line::from(vec![
                Span::styled("Dev Tag:      ", Style::default().fg(Color::Yellow)),
                Span::styled(format!("[{tag}]"), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            ]));
        }

        lines.push(Line::raw(""));
        lines.push(Line::styled("--- Process Metadata ---", Style::default().fg(Color::DarkGray)));

        lines.push(Line::from(vec![
            Span::styled("PID:          ", Style::default().fg(Color::Cyan)),
            Span::styled(p.pid.to_string(), Style::default().fg(Color::White)),
        ]));

        if let Some(proc) = &p.process {
            lines.push(Line::from(vec![
                Span::styled("Name:         ", Style::default().fg(Color::Cyan)),
                Span::styled(&proc.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));

            lines.push(Line::from(vec![
                Span::styled("Memory RAM:   ", Style::default().fg(Color::Cyan)),
                Span::styled(proc.formatted_memory(), Style::default().fg(Color::Green)),
            ]));

            if let Some(path) = &proc.exe_path {
                lines.push(Line::raw(""));
                lines.push(Line::styled("Executable Path:", Style::default().fg(Color::Cyan)));
                lines.push(Line::styled(path, Style::default().fg(Color::DarkGray)));
            }

            if !proc.cmd_args.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::styled("Command Arguments:", Style::default().fg(Color::Cyan)));
                lines.push(Line::styled(proc.cmd_args.join(" "), Style::default().fg(Color::DarkGray)));
            }
        } else {
            lines.push(Line::styled("No additional process metadata available.", Style::default().fg(Color::DarkGray)));
        }

        lines
    } else {
        vec![Line::styled("No port selected.", Style::default().fg(Color::DarkGray))]
    };

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let content = if let Some(status) = app.get_active_status() {
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(status, Style::default().fg(Color::White)),
        ])
    } else {
        Line::from(vec![
            Span::styled("[q]", Style::default().fg(Color::Yellow)),
            Span::raw(" Quit  "),
            Span::styled("[Shift+K]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" Kill  "),
            Span::styled("[r]", Style::default().fg(Color::Green)),
            Span::raw(" Refresh  "),
            Span::styled("[/]", Style::default().fg(Color::Cyan)),
            Span::raw(" Filter  "),
            Span::styled("[Tab]", Style::default().fg(Color::Magenta)),
            Span::raw(" Mode  "),
            Span::styled("[?]", Style::default().fg(Color::White)),
            Span::raw(" Help"),
        ])
    };

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_confirm_kill_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(55, 30, frame.area());
    frame.render_widget(Clear, area);

    let (proc_name, pid, port) = if let Some(p) = app.selected_port() {
        let name = p.process.as_ref().map(|pr| pr.name.as_str()).unwrap_or("unknown");
        (name, p.pid, p.port)
    } else {
        ("unknown", 0, 0)
    };

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("WARNING: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw("Terminate this process?"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Process: ", Style::default().fg(Color::Yellow)),
            Span::styled(proc_name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("PID:     ", Style::default().fg(Color::Yellow)),
            Span::styled(pid.to_string(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Port:    ", Style::default().fg(Color::Yellow)),
            Span::styled(port.to_string(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(" [Enter/Y] Terminate ", Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw("   "),
            Span::styled(" [Esc/N] Cancel ", Style::default().bg(Color::DarkGray).fg(Color::White)),
        ]),
    ];

    let block = Block::default()
        .title(" Confirm Process Termination ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center).block(block);
    frame.render_widget(paragraph, area);
}

fn render_help_modal(frame: &mut Frame) {
    let area = centered_rect(50, 45, frame.area());
    frame.render_widget(Clear, area);

    let lines = vec![
        Line::raw(""),
        Line::styled("PortGuard Keybindings:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Line::raw(""),
        Line::from(vec![Span::styled("  Up / k, Down / j  ", Style::default().fg(Color::Yellow)), Span::raw("Navigate table rows")]),
        Line::from(vec![Span::styled("  Shift+K / x       ", Style::default().fg(Color::Red)), Span::raw("Terminate selected process")]),
        Line::from(vec![Span::styled("  /                 ", Style::default().fg(Color::Cyan)), Span::raw("Filter by name, PID, port or tag")]),
        Line::from(vec![Span::styled("  Tab               ", Style::default().fg(Color::Magenta)), Span::raw("Toggle LISTENING / ALL connections")]),
        Line::from(vec![Span::styled("  r                 ", Style::default().fg(Color::Green)), Span::raw("Refresh data")]),
        Line::from(vec![Span::styled("  ? / h             ", Style::default().fg(Color::White)), Span::raw("Open this help screen")]),
        Line::from(vec![Span::styled("  q / Esc           ", Style::default().fg(Color::DarkGray)), Span::raw("Quit PortGuard")]),
        Line::raw(""),
        Line::styled("Press [Esc], [Enter] or [q] to close", Style::default().fg(Color::DarkGray)),
    ];

    let block = Block::default()
        .title(" Help & Keybindings ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
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

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
