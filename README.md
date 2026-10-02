# portguard

Utilitário de linha de comando e interface de terminal (TUI) para monitoramento e gerenciamento de portas de rede no Windows.

## Visão Geral

O **portguard** permite inspecionar portas TCP e UDP ativas no sistema operacional, exibindo o identificador de processo (PID), nome do executável, consumo de memória de trabalho (RAM) e tags automáticas de serviços de desenvolvimento (PostgreSQL, MySQL, Node.js, Vite, Redis, Docker, etc.), permitindo também o encerramento seguro e rápido de processos conflitantes.

---

## Como Usar

Você pode utilizar o PortGuard baixando o executável standalone pré-compilado (sem necessidade de ter o Rust instalado) ou compilando diretamente a partir do código-fonte.

### Opção 1: Usando o Executável Pré-compilado (.exe)

Não requer instalação de Rust, Node.js ou qualquer outro runtime.

1. Baixe o arquivo `portguard.exe` na aba [Releases](https://github.com/Gabriel-Cerqueira/portguard/releases).
2. Abra um terminal (PowerShell, Prompt de Comando ou Windows Terminal) na pasta onde o arquivo foi baixado.
3. Execute diretamente:

```powershell
# Abrir o Dashboard Interativo (TUI)
.\portguard.exe

# Ou utilizar os comandos diretos de linha de comando (CLI):
.\portguard.exe list
.\portguard.exe 3000
.\portguard.exe kill 3000
```

#### Adicionando ao PATH do Windows (Opcional - Uso Global)
Para executar digitando apenas `portguard` a partir de qualquer pasta ou terminal do seu sistema:
1. Mova o `portguard.exe` para uma pasta de sua preferência (ex.: `C:\Ferramentas\`).
2. Adicione o caminho dessa pasta à variável de ambiente `Path` do Windows.
3. Abra um novo terminal e use diretamente o comando `portguard`.

---

### Opção 2: Compilando a partir do Código-Fonte (com Rust)

Requisitos:
- Rust (versão 1.85+ / edição 2024)
- Windows 10 ou 11

```bash
# Executar a interface interativa em modo de desenvolvimento
cargo run

# Compilar o binário otimizado de produção
cargo build --release
# O executável será gerado em: target\release\portguard.exe
```

---

## Comandos de Linha de Comando (CLI)

```bash
# Listar portas ativas em modo de escuta (Listening)
portguard list

# Listar todas as conexões de rede (incluindo ESTABLISHED e TIME_WAIT)
portguard list --all

# Inspecionar detalhes de uma porta específica
portguard 3000
portguard inspect 5432

# Encerrar o processo associado a uma porta (com confirmação interativa)
portguard kill 3000

# Encerrar sem solicitar confirmação
portguard kill 3000 --force
```

---

## Atalhos do Dashboard Interativo (TUI)

| Tecla | Ação |
| :--- | :--- |
| `Up` / `Down` ou `j` / `k` | Navegar pelas portas da tabela (com rolagem automática de tela) |
| `Shift + K` ou `x` | Abrir modal de confirmação para encerrar o processo selecionado |
| `/` | Ativar o filtro de busca por porta, PID, processo ou tag |
| `Tab` | Alternar entre exibir apenas portas em escuta (`LISTENING`) e todas as conexões |
| `r` | Atualizar os dados do sistema imediatamente |
| `?` ou `h` | Abrir o modal de ajuda com os atalhos |
| `q` ou `Esc` | Sair da aplicação |

---

## Estrutura do Projeto

- `src/model.rs`: Tipos fundamentais de domínio (`PortEntry`, `ProcessInfo`, `Protocol`, `PortState`).
- `src/scanner.rs`: Mapeamento nativo de portas locais via Windows IP Helper API (`GetExtendedTcpTable` e `GetExtendedUdpTable`).
- `src/process.rs`: Inspeção de processos com `sysinfo` e encerramento via `TerminateProcess`.
- `src/cli.rs`: Parser de argumentos e execução de comandos de terminal com `clap`.
- `src/ui.rs`: Layout, tabela navegável (`TableState`) e modais com `ratatui`.
- `src/app.rs`: Gerenciamento de estado, filtros e eventos de teclado.
- `src/main.rs`: Inicialização do runtime e loop principal de renderização com `crossterm`.

---

## Testes Automatizados

```bash
cargo test
```

---

## Licença

MIT
