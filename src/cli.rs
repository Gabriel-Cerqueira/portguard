//! Módulo de Interface de Linha de Comando (`src/cli.rs`)
//!
//! Fornece comandos diretos para automação em terminal, scripts e PowerShell
//! sem a necessidade de abrir a interface interativa (TUI).

use clap::{Parser, Subcommand};
use crate::process::ProcessManager;
use crate::scanner::{scan_all_ports, scan_listening_ports};

/// Guardião nativo de portas e processos de desenvolvimento no Windows
#[derive(Parser, Debug)]
#[command(name = "portguard", version, about, long_about = None)]
pub struct Cli {
    /// Comando opcional. Se omitido, o PortGuard abre o Dashboard Interativo (TUI).
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Número de porta direto para inspeção rápida (ex.: `portguard 3000`)
    pub direct_port: Option<u16>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Lista portas ativas em formato de texto no terminal
    List {
        /// Exibe todas as portas (inclusive ESTABLISHED/TIME_WAIT), e não apenas LISTENING
        #[arg(short, long)]
        all: bool,
    },
    /// Inspeciona os detalhes de uma porta específica (ex.: `portguard inspect 3000`)
    Inspect {
        /// Número da porta a ser inspecionada
        port: u16,
    },
    /// Encerra o processo que está ocupando uma porta (ex.: `portguard kill 3000`)
    Kill {
        /// Número da porta a ser liberada
        port: u16,
        /// Força o encerramento sem pedir confirmação
        #[arg(short, long)]
        force: bool,
    },
}

/// Executa os comandos do modo CLI direto
pub fn run_cli_command(cli: Cli) -> Result<(), String> {
    let mut proc_manager = ProcessManager::new();

    // Caso 1: Usuário passou apenas o número da porta diretamente: `portguard 3000`
    if let Some(port) = cli.direct_port {
        return inspect_port(port, &mut proc_manager);
    }

    // Caso 2: Subcomando explícito
    match cli.command {
        Some(Commands::List { all }) => list_ports(all, &mut proc_manager),
        Some(Commands::Inspect { port }) => inspect_port(port, &mut proc_manager),
        Some(Commands::Kill { port, force }) => kill_port_process(port, force, &mut proc_manager),
        None => Err("Nenhum comando CLI fornecido.".to_string()),
    }
}

/// Lista portas no terminal
fn list_ports(show_all: bool, proc_manager: &mut ProcessManager) -> Result<(), String> {
    let mut ports = if show_all {
        scan_all_ports()?
    } else {
        scan_listening_ports()?
    };

    proc_manager.enrich_ports(&mut ports);

    if ports.is_empty() {
        println!("Nenhuma porta encontrada.");
        return Ok(());
    }

    println!(
        "{:<6} {:<6} {:<8} {:<20} {:<10} {:<15} {}",
        "PORT", "PROTO", "PID", "PROCESSO", "RAM", "TAG", "ESTADO"
    );
    println!("{}", "-".repeat(80));

    for p in ports {
        let (name, ram) = match &p.process {
            Some(proc) => (proc.name.clone(), proc.formatted_memory()),
            None => ("-".to_string(), "-".to_string()),
        };
        let tag = p.dev_tag.unwrap_or("-");

        println!(
            "{:<6} {:<6} {:<8} {:<20} {:<10} {:<15} {}",
            p.port, p.protocol, p.pid, name, ram, tag, p.state
        );
    }

    Ok(())
}

/// Inspeciona uma porta específica
fn inspect_port(port: u16, proc_manager: &mut ProcessManager) -> Result<(), String> {
    let mut ports = scan_all_ports()?;
    proc_manager.enrich_ports(&mut ports);

    let matching: Vec<_> = ports.into_iter().filter(|p| p.port == port).collect();

    if matching.is_empty() {
        println!("Nenhum processo encontrado escutando ou conectado na porta {port}.");
        return Ok(());
    }

    println!("\n=== Detalhes da Porta {port} ===");
    for p in matching {
        println!("• Protocolo: {}", p.protocol);
        println!("• Estado:    {}", p.state);
        println!("• IP Local:  {}", p.local_ip);
        println!("• PID:       {}", p.pid);

        if let Some(tag) = p.dev_tag {
            println!("• Tag Dev:   [{tag}]");
        }

        if let Some(proc) = &p.process {
            println!("• Processo:  {}", proc.name);
            println!("• Memória:   {}", proc.formatted_memory());
            if let Some(exe) = &proc.exe_path {
                println!("• Caminho:   {}", exe);
            }
            if !proc.cmd_args.is_empty() {
                println!("• Comando:   {}", proc.cmd_args.join(" "));
            }
        }
        println!();
    }

    Ok(())
}

/// Encerra o processo que ocupa a porta indicada
fn kill_port_process(port: u16, force: bool, proc_manager: &mut ProcessManager) -> Result<(), String> {
    let mut ports = scan_all_ports()?;
    proc_manager.enrich_ports(&mut ports);

    let target = ports.into_iter().find(|p| p.port == port);

    let entry = match target {
        Some(e) => e,
        None => {
            return Err(format!("Nenhum processo encontrado utilizando a porta {port}."));
        }
    };

    let proc_name = entry
        .process
        .as_ref()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "desconhecido".to_string());

    if !force {
        println!(
            "Tem certeza que deseja encerrar o processo '{}' (PID {}) na porta {}? [s/N]",
            proc_name, entry.pid, port
        );
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| e.to_string())?;

        let input = input.trim().to_lowercase();
        if input != "s" && input != "sim" && input != "y" && input != "yes" {
            println!("Operação cancelada pelo usuário.");
            return Ok(());
        }
    }

    ProcessManager::kill_process(entry.pid)?;
    println!(
        "✔ Processo '{}' (PID {}) na porta {} foi encerrado com sucesso.",
        proc_name, entry.pid, port
    );

    Ok(())
}
