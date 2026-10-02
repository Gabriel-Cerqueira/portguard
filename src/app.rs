//! Application state and event handling.

use crate::model::PortEntry;
use crate::process::ProcessManager;
use crate::scanner::{scan_all_ports, scan_listening_ports};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use std::time::{Duration, Instant};

/// Interactive TUI operational modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Filtering,
    ConfirmKill,
    Help,
}

/// Global TUI application state.
pub struct App {
    pub raw_ports: Vec<PortEntry>,
    pub filtered_ports: Vec<PortEntry>,
    pub selected_index: usize,
    pub table_state: TableState,
    pub filter_input: String,
    pub mode: AppMode,
    pub show_all_connections: bool,
    pub proc_manager: ProcessManager,
    pub status_message: Option<(String, Instant)>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let mut proc_manager = ProcessManager::new();
        let mut raw_ports = scan_listening_ports().unwrap_or_default();
        proc_manager.enrich_ports(&mut raw_ports);

        let filtered_ports = raw_ports.clone();

        let mut table_state = TableState::default();
        if !filtered_ports.is_empty() {
            table_state.select(Some(0));
        }

        Self {
            raw_ports,
            filtered_ports,
            selected_index: 0,
            table_state,
            filter_input: String::new(),
            mode: AppMode::Normal,
            show_all_connections: false,
            proc_manager,
            status_message: Some((
                "Ready. Press '?' for help or 'Shift+K' to terminate selected process.".to_string(),
                Instant::now(),
            )),
            should_quit: false,
        }
    }

    pub fn refresh_data(&mut self) {
        let mut ports = if self.show_all_connections {
            scan_all_ports().unwrap_or_default()
        } else {
            scan_listening_ports().unwrap_or_default()
        };

        self.proc_manager.enrich_ports(&mut ports);
        self.raw_ports = ports;
        self.apply_filter();
        self.set_status("Ports refreshed.");
    }

    pub fn toggle_listening_filter(&mut self) {
        self.show_all_connections = !self.show_all_connections;
        self.refresh_data();
    }

    pub fn apply_filter(&mut self) {
        let query = self.filter_input.trim().to_lowercase();

        if query.is_empty() {
            self.filtered_ports = self.raw_ports.clone();
        } else {
            self.filtered_ports = self
                .raw_ports
                .iter()
                .filter(|p| {
                    p.port.to_string().contains(&query)
                        || p.pid.to_string().contains(&query)
                        || p.dev_tag.map(|t| t.to_lowercase().contains(&query)).unwrap_or(false)
                        || p.process.as_ref().map(|proc| proc.name.to_lowercase().contains(&query)).unwrap_or(false)
                        || p.protocol.to_string().to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }

        if self.filtered_ports.is_empty() {
            self.selected_index = 0;
            self.table_state.select(None);
        } else {
            if self.selected_index >= self.filtered_ports.len() {
                self.selected_index = self.filtered_ports.len() - 1;
            }
            self.table_state.select(Some(self.selected_index));
        }
    }

    pub fn next_row(&mut self) {
        if !self.filtered_ports.is_empty() {
            if self.selected_index + 1 < self.filtered_ports.len() {
                self.selected_index += 1;
            } else {
                self.selected_index = 0;
            }
            self.table_state.select(Some(self.selected_index));
        }
    }

    pub fn previous_row(&mut self) {
        if !self.filtered_ports.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.filtered_ports.len() - 1;
            }
            self.table_state.select(Some(self.selected_index));
        }
    }

    pub fn selected_port(&self) -> Option<&PortEntry> {
        self.filtered_ports.get(self.selected_index)
    }

    pub fn set_status<S: Into<String>>(&mut self, msg: S) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    pub fn get_active_status(&self) -> Option<&str> {
        if let Some((msg, time)) = &self.status_message {
            if time.elapsed() < Duration::from_secs(5) {
                return Some(msg.as_str());
            }
        }
        None
    }

    pub fn kill_selected_process(&mut self) {
        if let Some(entry) = self.selected_port() {
            let pid = entry.pid;
            let port = entry.port;
            let proc_name = entry
                .process
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "unknown".to_string());

            match ProcessManager::kill_process(pid) {
                Ok(()) => {
                    self.set_status(format!(
                        "Process '{}' (PID {}) on port {} terminated.",
                        proc_name, pid, port
                    ));
                    self.refresh_data();
                }
                Err(err) => {
                    self.set_status(format!("Termination failed: {err}"));
                }
            }
        }
        self.mode = AppMode::Normal;
    }

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
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Down | KeyCode::Char('j') => self.next_row(),
            KeyCode::Up | KeyCode::Char('k') => self.previous_row(),
            KeyCode::Char('K') | KeyCode::Char('x') | KeyCode::Delete => {
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
                self.set_status("Operation cancelled.");
            }
            _ => {}
        }
    }
}
