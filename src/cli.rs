//! Command-line interface parser and execution handlers.

use clap::{Parser, Subcommand};
use crate::process::ProcessManager;
use crate::scanner::{scan_all_ports, scan_listening_ports};

/// Windows native developer port and process guardian.
#[derive(Parser, Debug)]
#[command(name = "portguard", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Direct port number to inspect (e.g. `portguard 3000`)
    pub direct_port: Option<u16>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List active ports
    List {
        /// Display all ports instead of only listening ports
        #[arg(short, long)]
        all: bool,
    },
    /// Inspect details of a specific port
    Inspect {
        /// Port number to inspect
        port: u16,
    },
    /// Terminate the process occupying a port
    Kill {
        /// Port number to free
        port: u16,
        /// Force termination without prompt
        #[arg(short, long)]
        force: bool,
    },
}

/// Executes non-interactive CLI commands.
pub fn run_cli_command(cli: Cli) -> Result<(), String> {
    let mut proc_manager = ProcessManager::new();

    if let Some(port) = cli.direct_port {
        return inspect_port(port, &mut proc_manager);
    }

    match cli.command {
        Some(Commands::List { all }) => list_ports(all, &mut proc_manager),
        Some(Commands::Inspect { port }) => inspect_port(port, &mut proc_manager),
        Some(Commands::Kill { port, force }) => kill_port_process(port, force, &mut proc_manager),
        None => Err("No CLI command provided.".to_string()),
    }
}

fn list_ports(show_all: bool, proc_manager: &mut ProcessManager) -> Result<(), String> {
    let mut ports = if show_all {
        scan_all_ports()?
    } else {
        scan_listening_ports()?
    };

    proc_manager.enrich_ports(&mut ports);

    if ports.is_empty() {
        println!("No active ports found.");
        return Ok(());
    }

    println!(
        "{:<6} {:<6} {:<8} {:<20} {:<10} {:<15} {}",
        "PORT", "PROTO", "PID", "PROCESS", "RAM", "TAG", "STATE"
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

fn inspect_port(port: u16, proc_manager: &mut ProcessManager) -> Result<(), String> {
    let mut ports = scan_all_ports()?;
    proc_manager.enrich_ports(&mut ports);

    let matching: Vec<_> = ports.into_iter().filter(|p| p.port == port).collect();

    if matching.is_empty() {
        println!("No process found on port {port}.");
        return Ok(());
    }

    println!("\n=== Port {port} ===");
    for p in matching {
        println!("• Protocol:    {}", p.protocol);
        println!("• State:       {}", p.state);
        println!("• Local IP:    {}", p.local_ip);
        println!("• PID:         {}", p.pid);

        if let Some(tag) = p.dev_tag {
            println!("• Dev Tag:     [{tag}]");
        }

        if let Some(proc) = &p.process {
            println!("• Process:     {}", proc.name);
            println!("• Memory:      {}", proc.formatted_memory());
            if let Some(exe) = &proc.exe_path {
                println!("• Path:        {}", exe);
            }
            if !proc.cmd_args.is_empty() {
                println!("• Command:     {}", proc.cmd_args.join(" "));
            }
        }
        println!();
    }

    Ok(())
}

fn kill_port_process(port: u16, force: bool, proc_manager: &mut ProcessManager) -> Result<(), String> {
    let mut ports = scan_all_ports()?;
    proc_manager.enrich_ports(&mut ports);

    let target = ports.into_iter().find(|p| p.port == port);

    let entry = match target {
        Some(e) => e,
        None => {
            return Err(format!("No process found using port {port}."));
        }
    };

    let proc_name = entry
        .process
        .as_ref()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    if !force {
        println!(
            "Terminate process '{}' (PID {}) on port {}? [y/N]",
            proc_name, entry.pid, port
        );
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| e.to_string())?;

        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" && input != "s" && input != "sim" {
            println!("Operation cancelled.");
            return Ok(());
        }
    }

    ProcessManager::kill_process(entry.pid)?;
    println!(
        "Process '{}' (PID {}) on port {} terminated.",
        proc_name, entry.pid, port
    );

    Ok(())
}
