//! Domain model and data structures for network ports and OS processes.

use std::fmt;

/// Transport layer protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Protocol {
    Tcp,
    Udp,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "TCP"),
            Protocol::Udp => write!(f, "UDP"),
        }
    }
}

/// TCP connection state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortState {
    Listening,
    Established,
    TimeWait,
    CloseWait,
    Other(String),
}

impl fmt::Display for PortState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PortState::Listening => write!(f, "LISTENING"),
            PortState::Established => write!(f, "ESTABLISHED"),
            PortState::TimeWait => write!(f, "TIME_WAIT"),
            PortState::CloseWait => write!(f, "CLOSE_WAIT"),
            PortState::Other(s) => write!(f, "{s}"),
        }
    }
}

/// Process metadata retrieved from the operating system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub exe_path: Option<String>,
    pub memory_bytes: u64,
    pub cmd_args: Vec<String>,
}

impl ProcessInfo {
    /// Formats memory bytes into a human-readable string (e.g. "142.5 MB").
    pub fn formatted_memory(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if self.memory_bytes >= GB {
            return format!("{:.2} GB", self.memory_bytes as f64 / GB as f64);
        }
        if self.memory_bytes >= MB {
            return format!("{:.1} MB", self.memory_bytes as f64 / MB as f64);
        }
        if self.memory_bytes >= KB {
            return format!("{} KB", self.memory_bytes / KB);
        }
        if self.memory_bytes > 0 {
            return format!("{} B", self.memory_bytes);
        }
        "0 B".to_string()
    }
}

/// Active port entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortEntry {
    pub port: u16,
    pub protocol: Protocol,
    pub state: PortState,
    pub local_ip: String,
    pub pid: u32,
    pub process: Option<ProcessInfo>,
    pub dev_tag: Option<&'static str>,
}

impl PortEntry {
    pub fn new(
        port: u16,
        protocol: Protocol,
        state: PortState,
        local_ip: String,
        pid: u32,
        process: Option<ProcessInfo>,
    ) -> Self {
        let dev_tag = infer_dev_tag(port, process.as_ref().map(|p| p.name.as_str()));

        Self {
            port,
            protocol,
            state,
            local_ip,
            pid,
            process,
            dev_tag,
        }
    }
}

fn infer_dev_tag(port: u16, process_name: Option<&str>) -> Option<&'static str> {
    let name_lower = process_name.map(|n| n.to_lowercase()).unwrap_or_default();

    if name_lower.contains("node") || name_lower.contains("bun") || name_lower.contains("deno") {
        return Some("Node/JS");
    }
    if name_lower.contains("cargo") || name_lower.contains("rust") {
        return Some("Rust/Dev");
    }
    if name_lower.contains("python") || name_lower.contains("uvicorn") || name_lower.contains("gunicorn") {
        return Some("Python/API");
    }
    if name_lower.contains("postgres") {
        return Some("PostgreSQL");
    }
    if name_lower.contains("mysqld") || name_lower.contains("mariadb") {
        return Some("MySQL");
    }
    if name_lower.contains("redis") {
        return Some("Redis");
    }
    if name_lower.contains("docker") || name_lower.contains("com.docker") {
        return Some("Docker");
    }
    if name_lower.contains("java") || name_lower.contains("spring") {
        return Some("Java/JVM");
    }
    if name_lower.contains("go") {
        return Some("Go/API");
    }

    match port {
        3000 => Some("Web/React/Next"),
        5173 => Some("Vite/Frontend"),
        8080 => Some("HTTP/Alt"),
        8000 => Some("API/Dev"),
        4000 => Some("GraphQL/API"),
        5432 => Some("PostgreSQL"),
        3306 => Some("MySQL"),
        6379 => Some("Redis"),
        27017 => Some("MongoDB"),
        9200 => Some("Elasticsearch"),
        19000..=19006 => Some("Expo/Mobile"),
        8081 => Some("Metro/Expo"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_formatting() {
        let proc = ProcessInfo {
            pid: 1234,
            name: "test.exe".to_string(),
            exe_path: None,
            memory_bytes: 142 * 1024 * 1024 + 512 * 1024,
            cmd_args: vec![],
        };
        assert_eq!(proc.formatted_memory(), "142.5 MB");

        let small_proc = ProcessInfo {
            pid: 5678,
            name: "tiny.exe".to_string(),
            exe_path: None,
            memory_bytes: 512 * 1024,
            cmd_args: vec![],
        };
        assert_eq!(small_proc.formatted_memory(), "512 KB");
    }

    #[test]
    fn test_infer_dev_tag() {
        assert_eq!(infer_dev_tag(5432, None), Some("PostgreSQL"));
        assert_eq!(infer_dev_tag(3000, Some("node.exe")), Some("Node/JS"));
        assert_eq!(infer_dev_tag(9999, Some("mysqld.exe")), Some("MySQL"));
        assert_eq!(infer_dev_tag(12345, None), None);
    }
}
