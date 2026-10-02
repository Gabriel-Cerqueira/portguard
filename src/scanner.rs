//! Módulo de Scanner de Portas (`src/scanner.rs`)
//!
//! Este módulo realiza a comunicação direta com a API do Windows (IP Helper API)
//! para descobrir todas as portas TCP e UDP ativas e seus respectivos PIDs proprietários.
//!
//! O código foi estruturado de forma didática e segura, encapsulando as chamadas FFI (Foreign Function Interface)
//! do Win32 em funções puramente seguras e idiomáticas em Rust.

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

    /// Converte um número de porta retornado pela API do Windows (em Network Byte Order / Big Endian)
    /// para o formato nativo da máquina (Host Byte Order).
    #[inline]
    fn parse_port(raw_port: u32) -> u16 {
        // Os 16 bits menos significativos contêm a porta em ordem de bytes da rede (Big Endian)
        u16::from_be((raw_port & 0xFFFF) as u16)
    }

    /// Converte o código numérico de estado TCP da MIB do Windows para o enum de domínio `PortState`.
    fn parse_tcp_state(state: u32) -> PortState {
        // Valores padrão definidos pela MIB-II (RFC 1213 / Windows SDK):
        // 1 = CLOSED, 2 = LISTEN, 3 = SYN_SENT, 4 = SYN_RCVD, 5 = ESTAB,
        // 6 = FIN_WAIT1, 7 = FIN_WAIT2, 8 = CLOSE_WAIT, 9 = CLOSING, 10 = LAST_ACK, 11 = TIME_WAIT
        match state {
            2 => PortState::Listening,
            5 => PortState::Established,
            8 => PortState::CloseWait,
            11 => PortState::TimeWait,
            other => PortState::Other(format!("STATE_{other}")),
        }
    }

    /// Coleta todas as portas TCP IPv4 ativas e seus PIDs através da função `GetExtendedTcpTable`.
    pub fn scan_tcp_ipv4() -> Result<Vec<PortEntry>, String> {
        let mut entries = Vec::new();
        let mut buffer_size: u32 = 0;

        // Primeira chamada: passamos ponteiro nulo para descobrir o tamanho de buffer necessário.
        unsafe {
            let _ = GetExtendedTcpTable(
                std::ptr::null_mut(),
                &mut buffer_size,
                0, // Não precisa ordenar a tabela no driver C (faremos no Rust)
                AF_INET as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
        }

        if buffer_size == 0 {
            return Ok(entries);
        }

        // Alocamos um buffer de bytes com o tamanho exato solicitado pelo Windows.
        let mut buffer: Vec<u8> = vec![0; buffer_size as usize];

        // Segunda chamada: preenche o buffer alocado com a tabela de conexões TCP.
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
            return Err(format!("Erro ao chamar GetExtendedTcpTable: código {result}"));
        }

        // A estrutura `MIB_TCPTABLE_OWNER_PID` na memória do Windows possui o seguinte layout:
        // [dwNumEntries: u32] seguido imediatamente pelo array de [MIB_TCPROW_OWNER_PID; dwNumEntries]
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
                    None, // Será enriquecido posteriormente pelo módulo de processos
                ));
            }
        }

        Ok(entries)
    }

    /// Coleta todas as portas UDP IPv4 ativas e seus PIDs através da função `GetExtendedUdpTable`.
    pub fn scan_udp_ipv4() -> Result<Vec<PortEntry>, String> {
        let mut entries = Vec::new();
        let mut buffer_size: u32 = 0;

        // Primeira chamada para obter o tamanho do buffer necessário
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
            return Err(format!("Erro ao chamar GetExtendedUdpTable: código {result}"));
        }

        unsafe {
            let num_entries = *(buffer.as_ptr() as *const u32);
            let rows_ptr = buffer.as_ptr().add(std::mem::size_of::<u32>()) as *const MIB_UDPROW_OWNER_PID;

            for i in 0..num_entries {
                let row = *rows_ptr.add(i as usize);
                let port = parse_port(row.dwLocalPort);
                let ip = Ipv4Addr::from(u32::from_be(row.dwLocalAddr)).to_string();
                let pid = row.dwOwningPid;

                // UDP é connectionless, então portas vinculadas atuam como ouvintes
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

/// Função principal que executa o scanner completo de portas TCP e UDP no sistema.
///
/// Retorna uma lista de todas as portas ativas no sistema.
pub fn scan_all_ports() -> Result<Vec<PortEntry>, String> {
    #[cfg(windows)]
    {
        let mut all_ports = Vec::new();

        let tcp_ports = win32::scan_tcp_ipv4()?;
        all_ports.extend(tcp_ports);

        let udp_ports = win32::scan_udp_ipv4()?;
        all_ports.extend(udp_ports);

        // Ordena primeiro por número de porta, depois por protocolo
        all_ports.sort_by(|a, b| a.port.cmp(&b.port).then_with(|| a.protocol.cmp(&b.protocol)));

        Ok(all_ports)
    }

    #[cfg(not(windows))]
    {
        Err("O scanner de portas do PortGuard é atualmente otimizado para Windows.".to_string())
    }
}

/// Retorna apenas as portas em modo `Listening` (servidores de desenvolvimento e bancos de dados).
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
        assert!(result.is_ok(), "O scanner de portas deve rodar com sucesso");
        let ports = result.unwrap();
        // Em um sistema Windows ativo, normalmente existem dezenas de portas abertas
        println!("Portas encontradas no teste: {}", ports.len());
    }
}
