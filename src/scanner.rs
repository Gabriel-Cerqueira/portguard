//! Windows IP Helper API network port scanner.

use crate::model::{PortEntry, PortState, Protocol};
use std::net::Ipv4Addr;

#[cfg(windows)]
mod win32 {
    use super::*;
    use windows_sys::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, NO_ERROR};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCPROW_OWNER_PID, MIB_UDPROW_OWNER_PID,
        TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
    };
    use windows_sys::Win32::Networking::WinSock::AF_INET;

    #[inline]
    fn parse_port(raw_port: u32) -> u16 {
        u16::from_be((raw_port & 0xFFFF) as u16)
    }

    fn parse_tcp_state(state: u32) -> PortState {
        match state {
            2 => PortState::Listening,
            5 => PortState::Established,
            8 => PortState::CloseWait,
            11 => PortState::TimeWait,
            other => PortState::Other(format!("STATE_{other}")),
        }
    }

    /// Retrieves active TCP IPv4 connections and owning PIDs via `GetExtendedTcpTable`.
    pub fn scan_tcp_ipv4() -> Result<Vec<PortEntry>, String> {
        let mut entries = Vec::new();
        let mut buffer_size: u32 = 0;

        unsafe {
            let _ = GetExtendedTcpTable(
                std::ptr::null_mut(),
                &mut buffer_size,
                0,
                AF_INET as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
        }

        if buffer_size == 0 {
            return Ok(entries);
        }

        let mut buffer: Vec<u8> = vec![0; buffer_size as usize];

        let result = unsafe {
            GetExtendedTcpTable(
                buffer.as_mut_ptr() as *mut _,
                &mut buffer_size,
                0,
                AF_INET as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        };

        if result != NO_ERROR && result != ERROR_INSUFFICIENT_BUFFER {
            return Err(format!("GetExtendedTcpTable failed with code {result}"));
        }

        unsafe {
            let num_entries = *(buffer.as_ptr() as *const u32);
            let rows_ptr = buffer.as_ptr().add(std::mem::size_of::<u32>()) as *const MIB_TCPROW_OWNER_PID;

            for i in 0..num_entries {
                let row = *rows_ptr.add(i as usize);
                let port = parse_port(row.dwLocalPort);
                let state = parse_tcp_state(row.dwState);
                let ip = Ipv4Addr::from(u32::from_be(row.dwLocalAddr)).to_string();
                let pid = row.dwOwningPid;

                entries.push(PortEntry::new(
                    port,
                    Protocol::Tcp,
                    state,
                    ip,
                    pid,
                    None,
                ));
            }
        }

        Ok(entries)
    }

    /// Retrieves active UDP IPv4 bindings and owning PIDs via `GetExtendedUdpTable`.
    pub fn scan_udp_ipv4() -> Result<Vec<PortEntry>, String> {
        let mut entries = Vec::new();
        let mut buffer_size: u32 = 0;

        unsafe {
            let _ = GetExtendedUdpTable(
                std::ptr::null_mut(),
                &mut buffer_size,
                0,
                AF_INET as u32,
                UDP_TABLE_OWNER_PID,
                0,
            );
        }

        if buffer_size == 0 {
            return Ok(entries);
        }

        let mut buffer: Vec<u8> = vec![0; buffer_size as usize];

        let result = unsafe {
            GetExtendedUdpTable(
                buffer.as_mut_ptr() as *mut _,
                &mut buffer_size,
                0,
                AF_INET as u32,
                UDP_TABLE_OWNER_PID,
                0,
            )
        };

        if result != NO_ERROR && result != ERROR_INSUFFICIENT_BUFFER {
            return Err(format!("GetExtendedUdpTable failed with code {result}"));
        }

        unsafe {
            let num_entries = *(buffer.as_ptr() as *const u32);
            let rows_ptr = buffer.as_ptr().add(std::mem::size_of::<u32>()) as *const MIB_UDPROW_OWNER_PID;

            for i in 0..num_entries {
                let row = *rows_ptr.add(i as usize);
                let port = parse_port(row.dwLocalPort);
                let ip = Ipv4Addr::from(u32::from_be(row.dwLocalAddr)).to_string();
                let pid = row.dwOwningPid;

                entries.push(PortEntry::new(
                    port,
                    Protocol::Udp,
                    PortState::Listening,
                    ip,
                    pid,
                    None,
                ));
            }
        }

        Ok(entries)
    }
}

/// Scans all active TCP and UDP ports on the host.
pub fn scan_all_ports() -> Result<Vec<PortEntry>, String> {
    #[cfg(windows)]
    {
        let mut all_ports = Vec::new();

        let tcp_ports = win32::scan_tcp_ipv4()?;
        all_ports.extend(tcp_ports);

        let udp_ports = win32::scan_udp_ipv4()?;
        all_ports.extend(udp_ports);

        all_ports.sort_by(|a, b| a.port.cmp(&b.port).then_with(|| a.protocol.cmp(&b.protocol)));

        Ok(all_ports)
    }

    #[cfg(not(windows))]
    {
        Err("PortGuard scanner is currently supported only on Windows.".to_string())
    }
}

/// Scans only ports in listening state.
pub fn scan_listening_ports() -> Result<Vec<PortEntry>, String> {
    let all = scan_all_ports()?;
    let listening = all
        .into_iter()
        .filter(|entry| entry.state == PortState::Listening)
        .collect();
    Ok(listening)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_all_ports_runs_without_panicking() {
        let result = scan_all_ports();
        assert!(result.is_ok());
    }
}
