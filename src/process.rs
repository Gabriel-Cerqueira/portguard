//! Módulo de Gerenciamento de Processos (`src/process.rs`)
//!
//! Responsável por inspecionar o sistema operacional para obter metadados ricos
//! dos processos donos de portas (nome do executável, consumo de RAM, caminho no disco)
//! e fornecer a funcionalidade segura de encerramento (`kill_process`).

use crate::model::{PortEntry, ProcessInfo};
use sysinfo::{Pid, ProcessesToUpdate, System};

/// Gerenciador de processos do sistema que mantém o estado da árvore de processos.
pub struct ProcessManager {
    sys: System,
}

impl ProcessManager {
    /// Cria uma nova instância do gerenciador e realiza a varredura inicial de processos.
    pub fn new() -> Self {
        let mut sys = System::new();
        // Atualiza todos os processos do sistema para coletar nomes, memória e caminhos
        sys.refresh_processes(ProcessesToUpdate::All, true);
        Self { sys }
    }

    /// Atualiza a lista de processos em execução no sistema operacional.
    /// Chamado periodicamente ou antes de cada varredura de portas para obter dados frescos.
    pub fn refresh(&mut self) {
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
    }

    /// Busca as informações detalhadas de um processo pelo seu PID.
    pub fn get_process_info(&self, pid: u32) -> Option<ProcessInfo> {
        let sys_pid = Pid::from_u32(pid);
        let proc = self.sys.process(sys_pid)?;

        let name = proc.name().to_string_lossy().to_string();
        let exe_path = proc.exe().map(|p| p.to_string_lossy().to_string());
        let memory_bytes = proc.memory();
        let cmd_args = proc
            .cmd()
            .iter()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();

        Some(ProcessInfo {
            pid,
            name,
            exe_path,
            memory_bytes,
            cmd_args,
        })
    }

    /// Recebe uma lista de portas descobertas e enriquece cada uma com as informações
    /// do processo correspondente (nome, memória, caminho e tags de dev).
    pub fn enrich_ports(&mut self, ports: &mut [PortEntry]) {
        self.refresh();

        for entry in ports.iter_mut() {
            if entry.pid > 0 {
                if let Some(info) = self.get_process_info(entry.pid) {
                    // Atualiza a tag com base no novo nome do processo descoberto
                    let dev_tag = crate::model::PortEntry::new(
                        entry.port,
                        entry.protocol,
                        entry.state.clone(),
                        entry.local_ip.clone(),
                        entry.pid,
                        Some(info.clone()),
                    )
                    .dev_tag;

                    entry.process = Some(info);
                    entry.dev_tag = dev_tag;
                }
            }
        }
    }

    /// Encerra forçadamente um processo pelo PID.
    ///
    /// Retorna `Ok(())` se o sinal de término foi emitido com sucesso,
    /// ou `Err(String)` caso o processo não exista ou o acesso seja negado.
    pub fn kill_process(pid: u32) -> Result<(), String> {
        if pid == 0 {
            return Err("Não é permitido encerrar o System Idle Process (PID 0).".to_string());
        }

        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, FALSE};
            use windows_sys::Win32::System::Threading::{
                OpenProcess, TerminateProcess, PROCESS_TERMINATE,
            };

            unsafe {
                // Abre o processo solicitando permissão para encerrá-lo (PROCESS_TERMINATE)
                let handle = OpenProcess(PROCESS_TERMINATE, FALSE, pid);
                if handle.is_null() {
                    let err = GetLastError();
                    return Err(format!(
                        "Falha ao abrir processo com PID {pid}. Erro Win32: {err} (pode exigir privilégios de Administrador)"
                    ));
                }

                // Código de saída 1 indica término forçado por ferramenta externa
                let success = TerminateProcess(handle, 1);
                CloseHandle(handle);

                if success == 0 {
                    let err = GetLastError();
                    return Err(format!(
                        "Falha ao encerrar processo PID {pid}. Erro Win32: {err}"
                    ));
                }

                Ok(())
            }
        }

        #[cfg(not(windows))]
        {
            let mut sys = System::new();
            sys.refresh_processes(ProcessesToUpdate::All, true);
            if let Some(proc) = sys.process(Pid::from_u32(pid)) {
                if proc.kill() {
                    Ok(())
                } else {
                    Err(format!("Não foi possível encerrar o processo PID {pid}."))
                }
            } else {
                Err(format!("Processo PID {pid} não encontrado."))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_manager_creation_and_lookup() {
        let manager = ProcessManager::new();
        let current_pid = std::process::id();
        let info = manager.get_process_info(current_pid);

        assert!(info.is_some(), "Deve conseguir obter informações do processo atual");
        let proc = info.unwrap();
        assert_eq!(proc.pid, current_pid);
        println!("Processo atual detectado: {} (PID: {})", proc.name, proc.pid);
    }
}
