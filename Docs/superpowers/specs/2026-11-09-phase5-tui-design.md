# Phase 5 — TUI e Fluxo do Usuário: Design Document

**Data:** 2026-11-09  
**Status:** Aprovado para implementação  
**Base:** Opção A (App-centralizada) + Reducer pattern + Tokio/mpsc

---

## 1. Intent e Porquê

Criar uma interface TUI navegável que conecte as operações já implementadas (Discovery, Storage/Chroot) sem adicionar diagnóstico ou reparação de bootloader. O foco é **segurança operacional**, **estado previsível** e **experiência de terminal robusta**.

**Princípios:**
- Estado único centralizado (`AppModel`) — sem duplicação
- Fluxo unidirecional: `Input → Reducer → Command → TaskManager → Event → Reducer → Render`
- Operações longas não bloqueiam a UI (Tokio tasks + mpsc)
- Cada operação mutável tem ID único, bloqueio de concorrência, cancelamento controlado e limpeza garantida
- Terminal sempre restaurado ao sair (panic, erro, saída normal)

---

## 2. Goals, Non-Goals, Anti-Goals

### Goals
- [ ] Navegação entre telas: Welcome → Discovery → InstallSelect → MountReview → MountProgress → ChrootActive → (volta) MountReview
- [ ] Discovery assíncrono com progresso/resultado via eventos
- [ ] MountPlan revisável antes de execução; mounts existentes vs. planejados; ambiguidades visíveis
- [ ] Confirmação explícita antes de montar
- [ ] Progresso de montagem (stage/percent/message)
- [ ] Chroot com ciclo de vida controlado (prepare → execute → cleanup)
- [ ] Tela de erro com session_id + exportação relatório sanitizado
- [ ] Tela de logs/relatórios (entregue pelo Agente 2)
- [ ] Reducer testável unitariamente (estado puro)
- [ ] Restauração terminal em todos os caminhos de saída

### Non-Goals
- Diagnóstico de bootloader (GRUB, Limine, systemd-boot) — Fase 6
- Reparação de boot — Fase 8
- Backup/transação/rollback — Fase 7
- Migração de bootloader ou conversão de layout EFI

### Anti-Goals
- ❌ Estado duplicado entre `AppState` e `SharedData`
- ❌ Comando e evento no mesmo enum
- ❌ Lógica assíncrona espalhada no `App`
- ❌ `ChrootManager` compartilhado via `Arc<Mutex>` entre tasks
- ❌ Operações mutáveis simultâneas sem bloqueio
- ❌ Terminal corrompido ao sair

---

## 3. Constraints

- **Rust 1.99+**, `tokio 1.40`, `ratatui 0.28+`, `crossterm 0.27+`
- Deve compilar com `cargo build --locked`
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` devem passar
- Testes unitários do reducer: `cargo test --all-targets` passa
- Não executa comandos destrutivos em testes (mocks/fixtures apenas)
- Logs via `tracing` (Fase 2), relatórios sanitizados (Fase 2)

---

## 4. Approaches Considerados

| Approach | Trade-offs | Decisão |
|----------|------------|---------|
| **A: App-centralizada + Reducer** (escolhida) | Simples, estado puro, testável, previsível | ✅ Aprovada |
| B: Stack de telas (trait objects) | Flexível mas complexo, difícil testar estado | ❌ Rejeitada |
| C: MVU estilo Elm (puro) | Conceito excelente, mas boilerplate excessivo para fase 5 | ⚠️ Parcial (Reducer pattern adotado) |

**Por que A + Reducer:** Melhor equilíbrio para projeto que crescerá para Fases 6-8. Reducer dá transições puras/testáveis; TaskManager isola async; AppModel centralizado evita bugs de sincronização.

---

## 5. Arquitetura Final

### 5.1 Estrutura de Módulos

```
src/
├── app/
│   ├── mod.rs           // Re-exports públicos
│   ├── app.rs           // App principal: run(), loop principal, terminal setup/teardown
│   ├── model.rs         // AppModel, SharedData, OperationState, AppState
│   ├── state.rs         // AppState enum (navegação apenas)
│   ├── reducer.rs       // Reducer puro: reduce(model, event) -> (model, Vec<Command>)
│   ├── commands.rs      // AppCommand enum (solicitações de execução)
│   └── events.rs        // AppEvent enum (resultados/notificações)
│
├── runtime/
│   ├── mod.rs
│   ├── task_manager.rs  // Despacha commands, gerencia operações concorrentes
│   └── channels.rs      // Tipos de canais mpsc + helpers
│
├── ui/
│   ├── mod.rs
│   ├── layout.rs        // Layout principal (header, body, footer, help)
│   ├── components/      // Widgets reutilizáveis (list, table, progress, dialog)
│   └── screens/
│       ├── welcome.rs
│       ├── discovery.rs
│       ├── install_select.rs
│       ├── mount_review.rs
│       ├── mount_progress.rs
│       ├── chroot.rs
│       └── logs.rs
│
└── operations/
    ├── mod.rs
    ├── discovery.rs     // run_discovery() wrapper para TaskManager
    ├── mount.rs         // build_mount_plan + MountSession::mount_all
    ├── chroot.rs        // ChrootManager prepare/execute/cleanup
    └── boot_repair.rs   // Stub para Fase 8 (não implementado agora)
```

### 5.2 Modelos de Dados

```rust
// app/state.rs
pub enum AppState {
    Welcome,
    Discovery,                    // Auto-inicia discovery ao entrar
    InstallSelect,                // Lista linux_candidates do DiscoveryReport
    MountReview,                  // Mostra MountPlan + mounts existentes + ambiguidades
    MountProgress { stage: MountStage }, // Progresso detalhado
    ChrootActive,                 // Shell interativo ou comando único
    LogsReport,                   // Tela de logs/relatórios (Agente 2)
    Error { message: String },    // Erro com session_id + opção exportar
}

// app/model.rs
pub struct AppModel {
    pub state: AppState,
    pub shared: SharedData,
    pub selected_install: Option<usize>,  // Índice em linux_candidates
    pub operation: OperationState,        // Estado da operação em andamento
}

pub struct SharedData {
    pub discovery_report: Option<DiscoveryReport>,
    pub mount_plan: Option<MountPlan>,
    pub mount_session: Option<MountSession>,
    pub chroot_manager: Option<ChrootManager>, // Apenas UMA task proprietária
    pub session_id: String,
    pub last_error: Option<AppError>,
}

pub enum OperationState {
    Idle,
    Running { op_id: OpId, kind: OpKind, cancel_requested: bool },
    Blocked { reason: String }, // Outra operação incompatível rodando
}

pub enum OpKind { Discovery, Mount, ChrootCommand }
pub struct OpId(uuid::Uuid); // Único por operação

// app/commands.rs
pub enum AppCommand {
    RunDiscovery,
    BuildMountPlan { candidate_idx: usize },
    ExecuteMount { plan: MountPlan },
    ExecuteChrootCommand { command: Vec<String> },
    CancelOperation { op_id: OpId },
    ExportReport { output_path: PathBuf },
    Quit,
}

// app/events.rs
pub enum AppEvent {
    DiscoveryCompleted(Result<DiscoveryReport, AppError>),
    MountPlanCompleted(Result<MountPlan, AppError>),
    MountProgress { stage: MountStage, percent: u8, message: String },
    MountCompleted(Result<MountSession, AppError>),
    ChrootCommandCompleted(Result<String, AppError>),
    ChrootPrepared(Result<(), AppError>),
    ChrootCleanedUp(Result<(), AppError>),
    ReportExported(Result<PathBuf, AppError>),
    TerminalResize,
    Tick, // Para timeouts/animations
}
```

### 5.3 Reducer (Puro, Testável)

```rust
// app/reducer.rs
pub fn reduce(model: AppModel, event: AppEvent) -> (AppModel, Vec<AppCommand>) {
    // Transições puras, sem side effects
    // Retorna novo model + comandos a despachar
}

// Exemplo:
match event {
    AppEvent::DiscoveryCompleted(Ok(report)) => {
        model.shared.discovery_report = Some(report);
        model.state = AppState::InstallSelect;
        (model, vec![])
    }
    AppEvent::DiscoveryCompleted(Err(e)) => {
        model.state = AppState::Error { message: e.to_string() };
        (model, vec![])
    }
    // ...
}
```

### 5.4 TaskManager (Controle de Operações)

```rust
// runtime/task_manager.rs
pub struct TaskManager {
    command_rx: mpsc::Receiver<AppCommand>,
    event_tx: mpsc::Sender<AppEvent>,
    active_operations: HashMap<OpId, OperationHandle>,
    blocked_queue: VecDeque<AppCommand>,
}

impl TaskManager {
    pub async fn run(mut self) {
        while let Some(cmd) = self.command_rx.recv().await {
            match cmd {
                AppCommand::RunDiscovery => self.spawn_discovery(),
                AppCommand::BuildMountPlan { candidate_idx } => self.spawn_build_mount_plan(candidate_idx),
                AppCommand::ExecuteMount { plan } => self.spawn_mount(plan),
                AppCommand::ExecuteChrootCommand { command } => self.spawn_chroot_command(command),
                AppCommand::CancelOperation { op_id } => self.cancel_operation(op_id),
                AppCommand::ExportReport { output_path } => self.spawn_export_report(output_path),
                AppCommand::Quit => self.shutdown_all().await,
            }
        }
    }

    fn can_run(&self, kind: OpKind) -> bool {
        // Bloqueia operações mutáveis simultâneas
        !self.active_operations.values().any(|op| op.kind.conflicts_with(kind))
    }

    fn spawn_mount(&mut self, plan: MountPlan) {
        if !self.can_run(OpKind::Mount) {
            self.event_tx.send(AppEvent::MountCompleted(Err(AppError::OperationBlocked))).ok();
            return;
        }
        let op_id = OpId::new();
        let handle = tokio::spawn(async move {
            // Executa MountSession::mount_all com progress callbacks
            // Envia MountProgress events via channel
        });
        self.active_operations.insert(op_id, OperationHandle { id: op_id, kind: OpKind::Mount, handle });
    }
}
```

### 5.5 ChrootManager — Propriedade Única

- `ChrootManager` vive em `SharedData.chroot_manager: Option<ChrootManager>`
- **Apenas a task de chroot** (spawned pelo TaskManager) tem acesso mutável
- Outras tasks enviam `AppCommand::ExecuteChrootCommand` → TaskManager serializa → task proprietária executa → devolve `AppEvent::ChrootCommandCompleted`
- Evita `Arc<Mutex<ChrootManager>>` e race conditions

### 5.6 Fluxo de Execução Completo

```
┌─────────────┐     Input (Crossterm)      ┌─────────────┐
│   Terminal  │ ─────────────────────────> │    App      │
│  (Ratatui)  │ <───────────────────────── │  (Model)    │
└─────────────┘       Render               └──────┬──────┘
                                                  │
                    ┌─────────────────────────────┘
                    ▼
            ┌───────────────┐
            │   Reducer     │  (puro: Model + Event -> Model + Commands)
            └───────┬───────┘
                    │ Commands
                    ▼
            ┌───────────────┐
            │ TaskManager   │  (Tokio tasks, mpsc channels)
            │  + Operations │
            └───────┬───────┘
                    │ Events
                    ▼
            ┌───────────────┐
            │   Reducer     │  (atualiza Model)
            └───────┬───────┘
                    │
                    ▼ (loop)
```

---

## 6. Telas (UI) — Responsabilidades

| Tela | Responsabilidade | Dados exibidos |
|------|------------------|----------------|
| `Welcome` | Boas-vindas, versão, config, tecla para iniciar | `Config`, `SessionId` |
| `Discovery` | Auto-inicia discovery, mostra spinner + logs | `DiscoveryReport` (parcial) |
| `InstallSelect` | Lista instalações Linux candidatas | `linux_candidates` (device, distro, confidence, reasons) |
| `MountReview` | **Crítica**: mostra plano vs. atual, ambiguidades, confirmação | `MountPlan.entries`, mounts existentes, flags `noauto`/`nofail`/ambiguous |
| `MountProgress` | Progresso detalhado (stage, percent, mensagem) | `MountStage`, `percent`, `message` |
| `ChrootActive` | Shell interativo ou execução comando único | Output sanitizado, prompt |
| `LogsReport` | Consulta logs/relatórios (Agente 2) | Sessões, eventos, exportação |
| `Error` | Mensagem curta + session_id + [Exportar] + [Voltar] | `AppError`, `session_id` |

**Componentes reutilizáveis (`ui/components/`):**
- `ListView<T>` — navegação setas, Enter, Esc
- `TableView` — colunas alinhadas, scroll
- `ProgressBar` — stage + percent + label
- `ConfirmDialog` — "Tem certeza?" com foco em "Não" por padrão
- `ErrorDialog` — mensagem + ações
- `LogView` — scroll, filter, export
- `HelpFooter` — teclas contextuais

---

## 7. Critérios de Aceitação (Obrigatórios)

1. ✅ Todas as telas navegáveis via teclado (setas, Enter, Esc, atalhos)
2. ✅ Estado centralizado em `AppModel` — sem duplicação `AppState`/`SharedData`
3. ✅ Nenhuma operação longa bloqueia a TUI (Tokio + mpsc)
4. ✅ Discovery retorna `AppEvent::DiscoveryCompleted` (assíncrono)
5. ✅ Montagem mostra `AppEvent::MountProgress` (stage/percent/message)
6. ✅ Falhas preservam `OpId`, `stage`, `message` para diagnóstico
7. ✅ Operações mutáveis incompatíveis bloqueadas (`OperationState::Blocked`)
8. ✅ Saída trata recursos: `cleanup()` montagens, `ChrootManager::cleanup()`, terminal restaurado
9. ✅ Chroot com ciclo controlado: `prepare` → `execute*` → `cleanup` (task proprietária)
10. ✅ Reducer tem testes unitários cobrindo transições principais (Welcome→Discovery→InstallSelect→MountReview→MountProgress→ChrootActive→Error→Welcome)

---

## 8. Riscos e Mitigações

| Risco | Mitigação |
|-------|-----------|
| Terminal corrompido em panic | `panic_hook` + `Drop` em `App` restaura terminal; `crossterm::terminal::disable_raw_mode` em `finally` |
| Deadlock em canais mpsc | Canais bounded + timeout no `recv`; `TaskManager` drena canais no shutdown |
| `ChrootManager` vazado entre tasks | Propriedade única: apenas task de chroot tem `&mut ChrootManager` |
| Montagem parcial deixa sistema inconsistente | `MountSession` transacional (Fase 4) + `rollback` automático em erro/cancelamento |
| Operação cancelada deixa mounts órfãos | `CancelOperation` → TaskManager mata task → `MountSession::rollback()` + `cleanup()` |

---

## 9. Integração com Fases Anteriores

- **Fase 2 (Logging):** `tracing` para eventos internos; `export_session_report()` na tela Error/Logs
- **Fase 3 (Discovery):** `discovery::run_discovery()` chamado por `operations::discovery`
- **Fase 4 (Storage/Chroot):** `storage::build_mount_plan()`, `MountSession`, `ChrootManager` chamados por `operations::mount`/`chroot`

---

## 10. Próximos Passos (Após Aprovação)

1. Invoke `writing-plans` skill para criar plano de implementação detalhado
2. Implementar `app/` (model, state, reducer, commands, events) + testes do reducer
3. Implementar `runtime/` (task_manager, channels)
4. Implementar `operations/` (discovery, mount, chroot)
5. Implementar `ui/` (layout, components, screens)
6. Integrar no `main.rs` e `app::run()`
7. Testes de integração manuais + `cargo test --all-targets`

---

**Aprovação:** Este design doc reflete as decisões acordadas. Implementação prossegue após confirmação.