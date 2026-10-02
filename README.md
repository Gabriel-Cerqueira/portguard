# portguard

Utilitário de linha de comando e interface de terminal (TUI) para monitoramento e gerenciamento de portas de rede no Windows.

## Visão Geral

O **portguard** permite inspecionar portas TCP e UDP ativas no sistema, exibindo o identificador de processo (PID), nome do executável, consumo de memória de trabalho e tags de serviços de desenvolvimento, permitindo também o encerramento seguro de processos conflitantes.

## Instalação e Execução

Requisitos:
- Rust (edição 2024 / estável recente)
- Windows 10/11

### Modo Interativo (TUI)

```bash
cargo run
```

Atalhos de teclado:
- `Up` / `Down` ou `j` / `k`: Navegar entre os registros da tabela.
- `Shift + K` ou `x`: Solicitar encerramento do processo selecionado.
- `/`: Filtrar por número de porta, PID, processo ou tag.
- `Tab`: Alternar entre exibir apenas portas em escuta (Listening) e todas as conexões.
- `r`: Atualizar dados imediatamente.
- `?`: Abrir modal de ajuda.
- `q` ou `Esc`: Sair da aplicação.

### Modo Linha de Comando (CLI)

```bash
# Listar portas em escuta
cargo run -- list

# Listar todas as conexões ativas
cargo run -- list --all

# Inspecionar uma porta específica
cargo run -- 3000
cargo run -- inspect 5432

# Encerrar o processo associado a uma porta
cargo run -- kill 3000

# Encerrar sem confirmação interativa
cargo run -- kill 3000 --force
```

## Estrutura do Código

- `src/model.rs`: Tipos fundamentais de domínio (`PortEntry`, `ProcessInfo`, `Protocol`, `PortState`).
- `src/scanner.rs`: Mapeamento de portas locais via Windows IP Helper API.
- `src/process.rs`: Coleta de consumo de recursos e rotina de finalização de processos.
- `src/cli.rs`: Parser de argumentos e execução de subcomandos via `clap`.
- `src/ui.rs`: Layout e renderização gráfica de terminal via `ratatui`.
- `src/app.rs`: Máquina de estado da interface e tratamento de eventos de entrada.
- `src/main.rs`: Inicialização do runtime e loop principal de renderização.

## Testes

```bash
cargo test
```

## Licença

MIT
