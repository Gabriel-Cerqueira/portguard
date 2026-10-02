//! Módulo de Estado da Aplicação (`src/app.rs`)
//!
//! Gerencia o ciclo de vida do dashboard TUI, eventos de teclado,
//! filtros de busca, paginação e confirmações de encerramento de processos.

use crate::model::PortEntry;
use crate::process::ProcessManager;
use crate::scanner::{scan_all_ports, scan_listening_ports};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};

/// Modos de interação da interface TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    /// Modo padrão de navegação com setas/j/k.
    Normal,
    /// Modo de digitação de filtro de texto (ativado com `/`).
    Filtering,
    /// Modal de confirmação para encerrar processo (ativado com `k`).
    ConfirmKill,
    /// Modal de ajuda com lista de atalhos (ativado com `?` ou `h`).
    Help,
}

/// Estado global da aplicação durante a execução do TUI.
pub struct App {
    /// Todas as portas lidas na última varredura.
    pub raw_ports: Vec<PortEntry>,
    /// Portas visíveis após aplicar o filtro de texto.
    pub filtered_ports: Vec<PortEntry>,
    /// Índice do item atualmente selecionado na tabela.
    pub selected_index: usize,
    /// Texto do filtro de busca atual.
    pub filter_input: String,
    /// Modo atual de navegação/interação.
    pub mode: AppMode,
    /// Se true, exibe todas as portas; se false, apenas portas em LISTENING.
    pub show_all_connections: bool,
    /// Gerenciador de processos para enriquecimento de metadados e encerramento.
    pub proc_manager: ProcessManager,
    /// Mensagem de status temporária exibida na barra inferior com timestamp de expiração.
    pub status_message: Option<(String, Instant)>,
    /// Flag indicando se a aplicação deve ser encerrada.
    pub should_quit: bool,
}

impl App {
    /// Inicializa a aplicação realizando a primeira varredura do sistema.
    pub fn new() -> Self {
        let mut proc_manager = ProcessManager::new();
        let mut raw_ports = scan_listening_ports().unwrap_or_default();
        proc_manager.enrich_ports(&mut raw_ports);

        let filtered_ports = raw_ports.clone();

        Self {
            raw_ports,
            filtered_ports,
            selected_index: 0,
            filter_input: String::new(),
            mode: AppMode::Normal,
            show_all_connections: false,
            proc_manager,
            status_message: Some((
                "Pronto. Pressione '?' para ajuda ou 'k' para encerrar a porta selecionada.".to_string(),
                Instant::now(),
            )),
            should_quit: false,
        }
    }

    /// Atualiza os dados de portas e processos do sistema.
    pub fn refresh_data(&mut self) {
        let mut ports = if self.show_all_connections {
            scan_all_ports().unwrap_or_default()
        } else {
            scan_listening_ports().unwrap_or_default()
        };

        self.proc_manager.enrich_ports(&mut ports);
        self.raw_ports = ports;
        self.apply_filter();
        self.set_status("Dados atualizados com sucesso.");
    }

    /// Alterna entre exibir apenas portas em LISTENING e TODAS as conexões ativas.
    pub fn toggle_listening_filter(&mut self) {
        self.show_all_connections = !self.show_all_connections;
        self.refresh_data();
    }

    /// Aplica o filtro de busca textual sobre a lista de portas.
    pub fn apply_filter(&mut self) {
        let query = self.filter_input.trim().to_lowercase();

        if query.is_empty() {
            self.filtered_ports = self.raw_ports.clone();
        } else {
            self.filtered_ports = self
                .raw_ports
                .iter()
                .filter(|p| {
                    // Busca por número da porta
                    p.port.to_string().contains(&query)
                        // Busca por PID
                        || p.pid.to_string().contains(&query)
                        // Busca por tag (ex.: postgres, react, etc.)
                        || p.dev_tag.map(|t| t.to_lowercase().contains(&query)).unwrap_or(false)
                        // Busca por nome do processo
                        || p.process.as_ref().map(|proc| proc.name.to_lowercase().contains(&query)).unwrap_or(false)
                        // Busca por protocolo
                        || p.protocol.to_string().to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }

        // Ajusta o cursor selecionado para não ficar fora dos limites
        if self.filtered_ports.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.filtered_ports.len() {
            self.selected_index = self.filtered_ports.len() - 1;
        }
    }

    /// Move o cursor de seleção para a próxima linha da tabela.
    pub fn next_row(&mut self) {
        if !self.filtered_ports.is_empty() {
            if self.selected_index + 1 < self.filtered_ports.len() {
                self.selected_index += 1;
            } else {
                self.selected_index = 0; // Volta para o início (loop)
            }
        }
    }

    /// Move o cursor de seleção para a linha anterior da tabela.
    pub fn previous_row(&mut self) {
        if !self.filtered_ports.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.filtered_ports.len() - 1; // Vai para o final
            }
        }
    }

    /// Retorna uma referência à porta atualmente selecionada.
    pub fn selected_port(&self) -> Option<&PortEntry> {
        self.filtered_ports.get(self.selected_index)
    }

    /// Define uma mensagem de status temporária no rodapé.
    pub fn set_status<S: Into<String>>(&mut self, msg: S) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    /// Retorna a mensagem de status ativa se ainda não tiver expirado (5 segundos).
    pub fn get_active_status(&self) -> Option<&str> {
        if let Some((msg, time)) = &self.status_message {
            if time.elapsed() < Duration::from_secs(5) {
                return Some(msg.as_str());
            }
        }
        None
    }

    /// Executa o encerramento do processo atualmente selecionado.
    pub fn kill_selected_process(&mut self) {
        if let Some(entry) = self.selected_port() {
            let pid = entry.pid;
            let port = entry.port;
            let proc_name = entry
                .process
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "desconhecido".to_string());

            match ProcessManager::kill_process(pid) {
                Ok(()) => {
                    self.set_status(format!(
                        "✔ Processo '{}' (PID {}) na porta {} encerrado!",
                        proc_name, pid, port
                    ));
                    self.refresh_data();
                }
                Err(err) => {
                    self.set_status(format!("✖ Falha ao encerrar: {err}"));
                }
            }
        }
        self.mode = AppMode::Normal;
    }

    /// Processa os eventos de teclado disparados pelo usuário.
    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match self.mode {
            AppMode::Normal => self.handle_normal_keys(key),
            AppMode::Filtering => self.handle_filtering_keys(key),
            AppMode::ConfirmKill => self.handle_confirm_kill_keys(key),
            AppMode::Help => {
                if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') || key.code == KeyCode::Enter {
                    self.mode = AppMode::Normal;
                }
            }
        }
    }

    fn handle_normal_keys(&mut self, key: KeyEvent) {
        // Ctrl+C sempre sai
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Down | KeyCode::Char('j') => self.next_row(),
            KeyCode::Up | KeyCode::Char('k') => self.previous_row(),
            KeyCode::Char('K') => {
                if self.selected_port().is_some() {
                    self.mode = AppMode::ConfirmKill;
                }
            }
            KeyCode::Char('x') | KeyCode::Delete => {
                if self.selected_port().is_some() {
                    self.mode = AppMode::ConfirmKill;
                }
            }
            KeyCode::Char('r') => self.refresh_data(),
            KeyCode::Char('/') => {
                self.mode = AppMode::Filtering;
            }
            KeyCode::Tab => self.toggle_listening_filter(),
            KeyCode::Char('?') | KeyCode::Char('h') => self.mode = AppMode::Help,
            _ => {}
        }
    }

    fn handle_filtering_keys(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.mode = AppMode::Normal;
            }
            KeyCode::Backspace => {
                self.filter_input.pop();
                self.apply_filter();
            }
            KeyCode::Char(c) => {
                self.filter_input.push(c);
                self.apply_filter();
            }
            _ => {}
        }
    }

    fn handle_confirm_kill_keys(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Char('s') | KeyCode::Char('S') => {
                self.kill_selected_process();
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('q') => {
                self.mode = AppMode::Normal;
                self.set_status("Operação de encerramento cancelada.");
            }
            _ => {}
        }
    }
}
