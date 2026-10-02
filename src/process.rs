//! Process introspection and management routines.

use crate::model::{PortEntry, ProcessInfo};
use sysinfo::{Pid, ProcessesToUpdate, System};

/// Manages system process inspection and lifecycle operations.
pub struct ProcessManager {
    sys: System,
}

impl ProcessManager {
    /// Creates a new instance and performs the initial process tree scan.
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        Self { sys }
    }

    /// Refreshes the internal snapshot of running processes.
    pub fn refresh(&mut self) {
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
    }

    /// Retrieves detailed metadata for a given PID.
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

    /// Enriches a list of port entries with process metadata.
    pub fn enrich_ports(&mut self, ports: &mut [PortEntry]) {
        self.refresh();

        for entry in ports.iter_mut() {
            if entry.pid > 0 {
                if let Some(info) = self.get_process_info(entry.pid) {
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

    /// Terminates a process by its PID.
    pub fn kill_process(pid: u32) -> Result<(), String> {
        if pid == 0 {
            return Err("Cannot terminate System Idle Process (PID 0).".to_string());
        }

        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, FALSE};
            use windows_sys::Win32::System::Threading::{
                OpenProcess, TerminateProcess, PROCESS_TERMINATE,
            };

            unsafe {
                let handle = OpenProcess(PROCESS_TERMINATE, FALSE, pid);
                if handle.is_null() {
                    let err = GetLastError();
                    return Err(format!(
                        "Failed to open process {pid} (Win32 error {err})"
                    ));
                }

                let success = TerminateProcess(handle, 1);
                CloseHandle(handle);

                if success == 0 {
                    let err = GetLastError();
                    return Err(format!(
                        "Failed to terminate process {pid} (Win32 error {err})"
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
                    Err(format!("Could not terminate process {pid}."))
                }
            } else {
                Err(format!("Process {pid} not found."))
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

        assert!(info.is_some());
        let proc = info.unwrap();
        assert_eq!(proc.pid, current_pid);
    }
}
