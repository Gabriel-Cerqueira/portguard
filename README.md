# 🛡️ PortGuard

> **Guardião nativo de portas e processos de desenvolvimento no Windows construído em Rust.**

O **PortGuard** é uma ferramenta de alta performance para inspecionar, filtrar e liberar portas ocupadas no Windows (como `3000`, `5432`, `8080`, `3306`), identificando instantaneamente o processo, consumo de memória RAM, PID e caminho do executável com suporte a interface interativa (TUI) e comandos rápidos via CLI.

---

## 🚀 Como Executar

### 1. Modo Dashboard Interativo (TUI)
Para abrir a interface visual completa no seu terminal:

```bash
cargo run
```

### 2. Modo Linha de Comando (CLI)
Para listar ou inspecionar portas diretamente pelo PowerShell / terminal:

```bash
# Listar portas em modo LISTENING com processos e consumo de RAM
cargo run -- list

# Listar todas as conexões ativas (inclusive ESTABLISHED/TIME_WAIT)
cargo run -- list --all

# Inspecionar uma porta específica
cargo run -- 3000
# ou
cargo run -- inspect 5432

# Encerrar o processo que está ocupando uma porta (com confirmação)
cargo run -- kill 3000

# Encerrar forçadamente sem pedir confirmação
cargo run -- kill 3000 --force
```

---

## ⌨️ Atalhos de Teclado no Dashboard (TUI)

| Tecla | Ação |
| :--- | :--- |
| **`↑` / `k`** | Move para a porta anterior na tabela |
| **`↓` / `j`** | Move para a próxima porta na tabela |
| **`K` (Shift+k) / `x`** | Abre o modal para **encerrar o processo** selecionado |
| **`/`** | Ativa a **barra de busca/filtro** por número de porta, PID, processo ou tag |
| **`Tab`** | Alterna entre exibir apenas `LISTENING` (dev) ou `TODAS` as conexões |
| **`r`** | Atualiza imediatamente a lista de portas e processos |
| **`?` / `h`** | Exibe o modal de ajuda com os atalhos |
| **`q` / `Esc`** | Sai da aplicação |

---

## 🏗️ Estrutura do Código & Arquitetura

O projeto foi estruturado com foco em clareza, modularidade e boas práticas idiomáticas de Rust para facilitar o estudo:

```
src/
├── model.rs      # Estruturas de dados (PortEntry, ProcessInfo, Protocol, PortState) e tags dev
├── scanner.rs    # Varredura nativa do Windows (IP Helper API: GetExtendedTcpTable / GetExtendedUdpTable)
├── process.rs    # Enriquecimento com sysinfo (RAM, caminho, argumentos) e Win32 TerminateProcess
├── cli.rs        # Comandos diretos de terminal usando clap
├── ui.rs         # Renderização de layout, tabelas e modais com ratatui
├── app.rs        # Gerenciamento de estado e eventos de teclado
└── main.rs       # Ponto de entrada e loop de eventos com crossterm
```

---

## 🧪 Rodando os Testes Automatizados

```bash
cargo test
```
