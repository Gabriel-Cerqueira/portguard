pub mod app;
pub mod cli;
pub mod model;
pub mod process;
pub mod scanner;
pub mod ui;

use app::App;
use clap::Parser;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io::{self, stdout},
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Se foram passados argumentos via terminal (ex.: `portguard list`, `portguard 3000`), roda CLI
    if args.len() > 1 {
        let cli = cli::Cli::parse();
        if let Err(err) = cli::run_cli_command(cli) {
            eprintln!("Erro: {err}");
            std::process::exit(1);
        }
        return Ok(());
    }

    // Caso contrário, inicializa e executa o Dashboard Interativo (TUI)
    run_tui()
}

/// Inicializa o terminal no modo bruto (Raw Mode) e executa o loop de renderização do TUI
fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Configuração do Terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 2. Inicialização do Estado da Aplicação
    let mut app = App::new();
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();
    let mut last_auto_refresh = Instant::now();

    // 3. Loop Principal de Eventos
    let res: Result<(), Box<dyn std::error::Error>> = loop {
        // Renderiza o frame atual
        terminal.draw(|f| ui::render(f, &mut app))?;

        // Verifica se há eventos de teclado pendentes
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Processa apenas eventos de tecla pressionada (ignora Release/Repeat do Windows)
                if key.kind == event::KeyEventKind::Press {
                    app.handle_key_event(key);
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        // Auto-refresh a cada 3 segundos caso estejamos no modo normal (sem modal aberto)
        if app.mode == app::AppMode::Normal && last_auto_refresh.elapsed() >= Duration::from_secs(3) {
            app.refresh_data();
            last_auto_refresh = Instant::now();
        }

        if app.should_quit {
            break Ok(());
        }
    };

    // 4. Restauração do Terminal (sempre executado ao sair)
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Erro na execução do TUI: {err}");
    }

    Ok(())
}
