//! Módulo de Renderização da Interface Gráfica de Terminal (`src/ui.rs`)
//!
//! Utiliza a biblioteca `ratatui` para desenhar o layout, tabelas,
//! painéis de detalhes e caixas de diálogo modais.

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

/// Função principal de renderização chamada a cada frame do loop de eventos.
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    // Divide a tela verticalmente: Cabeçalho, Filtro (se ativo), Corpo Principal e Rodapé
    let has_filter = app.mode == AppMode::Filtering || !app.filter_input.is_empty();
    
    let constraints = if has_filter {
        vec![
            Constraint::Length(3), // Cabeçalho
            Constraint::Length(3), // Barra de Filtro
            Constraint::Min(5),    // Corpo Principal
            Constraint::Length(3), // Rodapé
        ]
    } else {
        vec![
            Constraint::Length(3), // Cabeçalho
            Constraint::Min(5),    // Corpo Principal
            Constraint::Length(3), // Rodapé
        ]
    };

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(size);

    let mut chunk_idx = 0;

    // 1. Renderiza o Cabeçalho
    render_header(frame, app, main_chunks[chunk_idx]);
    chunk_idx += 1;

    // 2. Renderiza a Barra de Filtro se estiver ativa
    if has_filter {
        render_filter_bar(frame, app, main_chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // 3. Renderiza o Corpo Principal (Tabela de Portas + Painel de Detalhes)
    render_body(frame, app, main_chunks[chunk_idx]);
    chunk_idx += 1;

    // 4. Renderiza o Rodapé de Atalhos e Status
    render_footer(frame, app, main_chunks[chunk_idx]);

    // 5. Renderiza Modais Sobrepostos se ativos
    match app.mode {
        AppMode::ConfirmKill => render_confirm_kill_modal(frame, app),
        AppMode::Help => render_help_modal(frame),
        _ => {}
    }
}

/// Renderiza o cabeçalho superior com título, modo e contadores
fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let mode_label = if app.show_all_connections {
        Span::styled(" [TODAS AS CONEXÕES] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" [LISTENING (DEV)] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
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
        Span::styled(" 🛡️ PortGuard ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("v0.1.0", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        mode_label,
        Span::raw(" | "),
        Span::styled(format!("Portas: {}", app.filtered_ports.len()), Style::default().fg(Color::White)),
        Span::raw(" | "),
        Span::styled(format!("RAM Dev Total: {}", formatted_total_ram), Style::default().fg(Color::Magenta)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let header_paragraph = Paragraph::new(title_line).block(block);
    frame.render_widget(header_paragraph, area);
}

/// Renderiza o campo de busca quando a tecla `/` é pressionada
fn render_filter_bar(frame: &mut Frame, app: &App, area: Rect) {
    let is_editing = app.mode == AppMode::Filtering;
    let border_color = if is_editing { Color::Yellow } else { Color::DarkGray };

    let text = if app.filter_input.is_empty() && is_editing {
        "Digite para filtrar por porta, PID, processo ou tag... (Pressione Enter ou Esc para sair do filtro)"
    } else {
        &app.filter_input
    };

    let filter_text = Line::from(vec![
        Span::styled("🔍 Filtro: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(text, if is_editing && app.filter_input.is_empty() { Style::default().fg(Color::DarkGray) } else { Style::default().fg(Color::White) }),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let filter_p = Paragraph::new(filter_text).block(block);
    frame.render_widget(filter_p, area);
}

/// Divide o corpo da tela entre a Tabela de Portas (60%) e os Detalhes do Processo (40%)
fn render_body(frame: &mut Frame, app: &mut App, area: Rect) {
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(area);

    render_ports_table(frame, app, body_chunks[0]);
    render_process_details(frame, app, body_chunks[1]);
}

/// Renderiza a lista interativa de portas em formato de tabela com estilo moderno
fn render_ports_table(frame: &mut Frame, app: &App, area: Rect) {
    let header_cells = ["PORT", "PROTO", "PID", "PROCESSO", "RAM", "TAG", "ESTADO"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = app
        .filtered_ports
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let is_selected = i == app.selected_index;

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

            let row_style = if is_selected {
                Style::default()
                    .bg(Color::Rgb(30, 60, 110))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if i % 2 == 0 {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Rgb(200, 200, 200))
            };

            Row::new(cells).style(row_style)
        })
        .collect();

    let widths = [
        Constraint::Length(7),  // PORT
        Constraint::Length(6),  // PROTO
        Constraint::Length(8),  // PID
        Constraint::Length(18), // PROCESSO
        Constraint::Length(10), // RAM
        Constraint::Length(14), // TAG
        Constraint::Min(10),    // ESTADO
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(" Portas Ativas ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    frame.render_widget(table, area);
}

/// Renderiza o painel lateral com todos os detalhes do processo selecionado
fn render_process_details(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Detalhes do Processo ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let content = if let Some(p) = app.selected_port() {
        let mut lines = Vec::new();

        lines.push(Line::from(vec![
            Span::styled("Porta Local: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} ({})", p.port, p.protocol), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Endereço IP: ", Style::default().fg(Color::Yellow)),
            Span::raw(&p.local_ip),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Estado:      ", Style::default().fg(Color::Yellow)),
            Span::styled(p.state.to_string(), Style::default().fg(Color::Green)),
        ]));

        if let Some(tag) = p.dev_tag {
            lines.push(Line::from(vec![
                Span::styled("Tag Dev:     ", Style::default().fg(Color::Yellow)),
                Span::styled(format!("[{tag}]"), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            ]));
        }

        lines.push(Line::raw(""));
        lines.push(Line::styled("─── Processo OS ───", Style::default().fg(Color::DarkGray)));

        lines.push(Line::from(vec![
            Span::styled("PID:         ", Style::default().fg(Color::Cyan)),
            Span::styled(p.pid.to_string(), Style::default().fg(Color::White)),
        ]));

        if let Some(proc) = &p.process {
            lines.push(Line::from(vec![
                Span::styled("Nome:        ", Style::default().fg(Color::Cyan)),
                Span::styled(&proc.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));

            lines.push(Line::from(vec![
                Span::styled("Consumo RAM: ", Style::default().fg(Color::Cyan)),
                Span::styled(proc.formatted_memory(), Style::default().fg(Color::Green)),
            ]));

            if let Some(path) = &proc.exe_path {
                lines.push(Line::raw(""));
                lines.push(Line::styled("Caminho do Executável:", Style::default().fg(Color::Cyan)));
                lines.push(Line::styled(path, Style::default().fg(Color::DarkGray)));
            }

            if !proc.cmd_args.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::styled("Linha de Comando:", Style::default().fg(Color::Cyan)));
                lines.push(Line::styled(proc.cmd_args.join(" "), Style::default().fg(Color::DarkGray)));
            }
        } else {
            lines.push(Line::styled("Nenhuma informação extra disponível para este PID.", Style::default().fg(Color::DarkGray)));
        }

        lines
    } else {
        vec![Line::styled("Nenhuma porta selecionada.", Style::default().fg(Color::DarkGray))]
    };

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

/// Renderiza o rodapé com barra de atalhos e mensagens de status
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
            Span::raw(" Sair  "),
            Span::styled("[K/x]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" Encerrar  "),
            Span::styled("[r]", Style::default().fg(Color::Green)),
            Span::raw(" Atualizar  "),
            Span::styled("[/]", Style::default().fg(Color::Cyan)),
            Span::raw(" Filtrar  "),
            Span::styled("[Tab]", Style::default().fg(Color::Magenta)),
            Span::raw(" Modo  "),
            Span::styled("[?]", Style::default().fg(Color::White)),
            Span::raw(" Ajuda"),
        ])
    };

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

/// Renderiza modal de confirmação para matar processo
fn render_confirm_kill_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(55, 30, frame.area());
    frame.render_widget(Clear, area); // Limpa o fundo para o modal

    let (proc_name, pid, port) = if let Some(p) = app.selected_port() {
        let name = p.process.as_ref().map(|pr| pr.name.as_str()).unwrap_or("desconhecido");
        (name, p.pid, p.port)
    } else {
        ("desconhecido", 0, 0)
    };

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("⚠ ATENÇÃO: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw("Deseja realmente encerrar este processo?"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("• Processo: ", Style::default().fg(Color::Yellow)),
            Span::styled(proc_name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("• PID:      ", Style::default().fg(Color::Yellow)),
            Span::styled(pid.to_string(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("• Porta:    ", Style::default().fg(Color::Yellow)),
            Span::styled(port.to_string(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(" [Enter/S] Confirmar Encerramento ", Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw("   "),
            Span::styled(" [Esc/N] Cancelar ", Style::default().bg(Color::DarkGray).fg(Color::White)),
        ]),
    ];

    let block = Block::default()
        .title(" ⚠ Confirmar Finalização de Processo ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center).block(block);
    frame.render_widget(paragraph, area);
}

/// Renderiza modal com todos os atalhos de ajuda
fn render_help_modal(frame: &mut Frame) {
    let area = centered_rect(50, 45, frame.area());
    frame.render_widget(Clear, area);

    let lines = vec![
        Line::raw(""),
        Line::styled("Atalhos de Teclado do PortGuard:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Line::raw(""),
        Line::from(vec![Span::styled("  ↑ / k, ↓ / j    ", Style::default().fg(Color::Yellow)), Span::raw("Navegar entre as portas da tabela")]),
        Line::from(vec![Span::styled("  K (Shift+k) / x ", Style::default().fg(Color::Red)), Span::raw("Abrir modal para encerrar processo")]),
        Line::from(vec![Span::styled("  /               ", Style::default().fg(Color::Cyan)), Span::raw("Filtrar portas por nome, PID ou tag")]),
        Line::from(vec![Span::styled("  Tab             ", Style::default().fg(Color::Magenta)), Span::raw("Alternar entre apenas LISTENING e TODAS")]),
        Line::from(vec![Span::styled("  r               ", Style::default().fg(Color::Green)), Span::raw("Atualizar dados imediatamente")]),
        Line::from(vec![Span::styled("  ? / h           ", Style::default().fg(Color::White)), Span::raw("Abrir esta tela de ajuda")]),
        Line::from(vec![Span::styled("  q / Esc         ", Style::default().fg(Color::DarkGray)), Span::raw("Sair do PortGuard")]),
        Line::raw(""),
        Line::styled("Pressione [Esc], [Enter] ou [q] para fechar", Style::default().fg(Color::DarkGray)),
    ];

    let block = Block::default()
        .title(" Ajuda & Atalhos ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Função auxiliar para calcular um retângulo perfeitamente centralizado na tela
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
