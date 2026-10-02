pub mod cli;
pub mod model;
pub mod process;
pub mod scanner;

use clap::Parser;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Se foram passados argumentos além do nome do executável, roda em modo CLI
    if args.len() > 1 {
        let cli = cli::Cli::parse();
        if let Err(err) = cli::run_cli_command(cli) {
            eprintln!("Erro: {err}");
            std::process::exit(1);
        }
    } else {
        println!("PortGuard TUI Dashboard (em desenvolvimento)");
    }
}
