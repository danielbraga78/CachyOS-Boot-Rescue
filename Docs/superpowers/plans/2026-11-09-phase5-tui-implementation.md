# Phase 5 TUI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement TUI com Ratatui + Crossterm conectando Discovery, Storage/Chroot via arquitetura App-centralizada + Reducer + TaskManager.

**Architecture:** AppModel centralizado (AppState navegação + SharedData dados + OperationState controle), Reducer puro para transições, TaskManager (Tokio + mpsc) para operações assíncronas, ChrootManager com propriedade única. Fluxo: Input → Reducer → Command → TaskManager → Event → Reducer → Render.

**Tech Stack:** Rust 1.99, tokio 1.40, ratatui 0.28+, crossterm 0.27+, tracing 0.1, clap 4.5, uuid 1.0, anyhow/thiserror.

**Spec:** `Docs/superpowers/specs/2026-11-09-phase5-tui-design.md`

## Global Constraints

- `cargo fmt --check` passa
- `cargo clippy --all-targets --all-features -- -D warnings` passa
- `cargo test --all-targets` passa (inclui novos testes reducer)
- `cargo build --locked` passa
- Nenhum comando destrutivo em testes (mocks/fixtures apenas)
- Terminal sempre restaurado ao sair (panic, erro, saída normal)
- Logs via `tracing` (Fase 2), relatórios sanitizados via `logging::export_session_report`

## Review Focus

| Input/Condição | Comportamento Esperado | Tarefa que Cobre |
|----------------|------------------------|------------------|
| Panic durante operação de montagem | Terminal restaurado, mounts limpos via `MountSession::rollback()` + `cleanup()` | Task 12 (App run loop + panic hook) |
| Usuário pressiona ESC durante montagem | Operação cancelada, `MountSession::rollback()` executado, volta para MountReview | Task 8 (TaskManager cancel) + Task 10 (MountProgress screen) |
| Duas operações mutáveis simultâneas (ex: mount + chroot) | Segunda bloqueada com `OperationState::Blocked`, mensagem clara | Task 7 (TaskManager concorrência) |
| ChrootManager usado após cleanup | Erro claro "Chroot não preparado", não panic | Task 9 (operations/chroot) |
| Terminal redimensionado durante operação longa | UI re-renderiza corretamente, progresso preservado | Task 11 (App loop + TerminalResize event) |

---

## File Structure (Novos Arquivos)

```
src/
├── app/
│   ├── mod.rs              // Re-exports públicos
│   ├── app.rs              // App struct, run(), terminal setup/teardown
│   ├── model.rs            // AppModel, SharedData, OperationState, OpId, OpKind
│   ├── state.rs            // AppState enum
│   ├── reducer.rs          // reduce(model, event) -> (model, Vec<Command>)
│   ├── commands.rs         // AppCommand enum
│   └── events.rs           // AppEvent enum
│
├── runtime/
│   ├── mod.rs
│   ├── task_manager.rs     // TaskManager, OperationHandle, concorrência
│   └── channels.rs         // Canais mpsc typed + helpers
│
├── operations/
│   ├── mod.rs
│   ├── discovery.rs        // run_discovery() wrapper
│   ├── mount.rs            // build_mount_plan + MountSession::mount_all
│   ├── chroot.rs           // ChrootManager prepare/execute/cleanup (task proprietária)
│   └── boot_repair.rs      // Stub (não implementado)
│
└── ui/
    ├── mod.rs
    ├── layout.rs           // Layout principal (header, body, footer, help)
    ├── components/
    │   ├── mod.rs
    │   ├── list_view.rs
    │   ├── table_view.rs
    │   ├── progress_bar.rs
    │   ├── confirm_dialog.rs
    │   ├── error_dialog.rs
    │   ├── log_view.rs
    │   └── help_footer.rs
    └── screens/
        ├── mod.rs
        ├── welcome.rs
        ├── discovery.rs
        ├── install_select.rs
        ├── mount_review.rs
        ├── mount_progress.rs
        ├── chroot.rs
        └── logs.rs
```

---

## Tasks

### Task 0: Add TUI Dependencies

**Files:**
- Modify: `cachyos-boot-rescue/Cargo.toml`

**Interfaces:**
- Produces: `ratatui`, `crossterm` dependencies available

- [ ] **Step 1: Add ratatui and crossterm to Cargo.toml**

```toml
# TUI
ratatui = { version = "0.28", features = ["crossterm"] }
crossterm = "0.27"
```

- [ ] **Step 2: Verify build**

Run: `cargo build --locked`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add cachyos-boot-rescue/Cargo.toml
git commit -m "feat(deps): add ratatui and crossterm for TUI"
```

---

### Task 1: AppModel, State, Commands, Events (Core Types)

**Files:**
- Create: `src/app/model.rs`
- Create: `src/app/state.rs`
- Create: `src/app/commands.rs`
- Create: `src/app/events.rs`
- Create: `src/app/mod.rs` (re-exports)
- Test: `tests/app_model_tests.rs`

**Interfaces:**
- Produces: `AppModel`, `SharedData`, `OperationState`, `AppState`, `AppCommand`, `AppEvent`, `OpId`, `OpKind`, `MountStage`

- [ ] **Step 1: Write failing tests for core types**

```rust
// tests/app_model_tests.rs
use cachyos_boot_rescue::app::{AppModel, AppState, OperationState, OpId, OpKind, MountStage};

#[test]
fn test_app_model_default_state_is_welcome() {
    let model = AppModel::default();
    assert!(matches!(model.state, AppState::Welcome));
    assert!(model.shared.discovery_report.is_none());
    assert!(matches!(model.operation, OperationState::Idle));
}

#[test]
fn test_op_id_unique() {
    let id1 = OpId::new();
    let id2 = OpId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_op_kind_conflicts() {
    assert!(OpKind::Mount.conflicts_with(OpKind::Mount));
    assert!(OpKind::Mount.conflicts_with(OpKind::ChrootCommand));
    assert!(!OpKind::Discovery.conflicts_with(OpKind::Discovery));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_app_model_default_state_is_welcome -- --nocapture`
Expected: FAIL (types not defined)

- [ ] **Step 3: Implement core types in `src/app/model.rs`, `state.rs`, `commands.rs`, `events.rs`**

Signatures exatas do spec:
- `AppModel { state: AppState, shared: SharedData, selected_install: Option<usize>, operation: OperationState }`
- `SharedData { discovery_report, mount_plan, mount_session, chroot_manager, session_id, last_error }`
- `AppState` enum com 8 variantes (Welcome, Discovery, InstallSelect, MountReview, MountProgress, ChrootActive, LogsReport, Error)
- `OperationState { Idle, Running { op_id, kind, cancel_requested }, Blocked { reason } }`
- `AppCommand` 7 variantes, `AppEvent` 10 variantes
- `OpId(uuid::Uuid)` com `new()`, `OpKind { Discovery, Mount, ChrootCommand }` com `conflicts_with(&self, other: &OpKind) -> bool`
- `MountStage { Preparing, MountingRoot, MountingBoot, MountingEfi, MountingOther, Finalizing, Completed }`

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test app_model -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/app/model.rs src/app/state.rs src/app/commands.rs src/app/events.rs src/app/mod.rs tests/app_model_tests.rs
git commit -m "feat(app): core types AppModel, AppState, AppCommand, AppEvent, OpId, OpKind"
```

---

### Task 2: Reducer (Transições Puras)

**Files:**
- Create: `src/app/reducer.rs`
- Test: `tests/app_reducer_tests.rs`

**Interfaces:**
- Consumes: `AppModel`, `AppEvent`, `AppCommand` (Task 1)
- Produces: `fn reduce(model: AppModel, event: AppEvent) -> (AppModel, Vec<AppCommand>)`

- [ ] **Step 1: Write failing tests for reducer transitions**

```rust
// tests/app_reducer_tests.rs
use cachyos_boot_rescue::app::{AppModel, AppState, OperationState, AppEvent, AppError, MountStage, MountPlan, MountSession, DiscoveryReport};

fn dummy_discovery_report() -> DiscoveryReport {
    DiscoveryReport {
        devices: vec![],
        linux_candidates: vec![],
        esp_candidates: vec![],
        uefi_state: cachyos_boot_rescue::discovery::UefiState::Uefi,
        timestamp: chrono::Utc::now(),
    }
}

fn dummy_mount_plan() -> MountPlan {
    MountPlan {
        root_mount: std::path::PathBuf::from("/mnt/root"),
        entries: vec![],
        required_mounts: vec![],
        warnings: vec![],
    }
}

#[test]
fn test_welcome_to_discovery_on_start() {
    let model = AppModel::default();
    let (model, cmds) = cachyos_boot_rescue::app::reduce(model, AppEvent::Tick);
    assert!(matches!(model.state, AppState::Welcome));
    assert!(cmds.is_empty());
}

#[test]
fn test_discovery_completed_ok_transitions_to_install_select() {
    let mut model = AppModel::default();
    model.state = AppState::Discovery;
    let report = dummy_discovery_report();
    let (model, cmds) = cachyos_boot_rescue::app::reduce(model, AppEvent::DiscoveryCompleted(Ok(report)));
    assert!(matches!(model.state, AppState::InstallSelect));
    assert!(model.shared.discovery_report.is_some());
    assert!(cmds.is_empty());
}

#[test]
fn test_discovery_completed_err_transitions_to_error() {
    let mut model = AppModel::default();
    model.state = AppState::Discovery;
    let (model, cmds) = cachyos_boot_rescue::app::reduce(model, AppEvent::DiscoveryCompleted(Err(AppError::CommandFailed("lsblk failed".into()))));
    assert!(matches!(model.state, AppState::Error { .. }));
    assert!(cmds.is_empty());
}

#[test]
fn test_install_select_to_mount_review_on_build_plan() {
    let mut model = AppModel::default();
    model.state = AppState::InstallSelect;
    model.selected_install = Some(0);
    model.shared.discovery_report = Some(dummy_discovery_report());
    let (model, cmds) = cachyos_boot_rescue::app::reduce(model, AppEvent::MountPlanCompleted(Ok(dummy_mount_plan())));
    assert!(matches!(model.state, AppState::MountReview));
    assert!(model.shared.mount_plan.is_some());
    assert!(cmds.is_empty());
}

#[test]
fn test_mount_review_to_mount_progress_on_execute_mount() {
    let mut model = AppModel::default();
    model.state = AppState::MountReview;
    model.shared.mount_plan = Some(dummy_mount_plan());
    let (model, cmds) = cachyos_boot_rescue::app::reduce(model, AppEvent::MountProgress { stage: MountStage::Preparing, percent: 0, message: "Starting".into() });
    assert!(matches!(model.state, AppState::MountProgress { .. }));
}

#[test]
fn test_mount_completed_ok_transitions_to_chroot_active() {
    let mut model = AppModel::default();
    model.state = AppState::MountProgress { stage: MountStage::Finalizing };
    let session = MountSession::new();
    let (model, cmds) = cachyos_boot_rescue::app::reduce(model, AppEvent::MountCompleted(Ok(session)));
    assert!(matches!(model.state, AppState::ChrootActive));
    assert!(model.shared.mount_session.is_some());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_discovery_completed_ok_transitions_to_install_select -- --nocapture`
Expected: FAIL (reducer not implemented)

- [ ] **Step 3: Implement `reduce` em `src/app/reducer.rs`**

Match exaustivo em `AppEvent` retornando `(AppModel, Vec<AppCommand>)`. Transições:
- `DiscoveryCompleted(Ok)` → `InstallSelect`, salva report
- `DiscoveryCompleted(Err)` → `Error`, salva erro
- `MountPlanCompleted(Ok)` → `MountReview`, salva plan
- `MountPlanCompleted(Err)` → `Error`
- `MountProgress` → atualiza `MountProgress { stage }`
- `MountCompleted(Ok)` → `ChrootActive`, salva session
- `MountCompleted(Err)` → `Error`
- `ChrootCommandCompleted` → mantém `ChrootActive`, loga resultado
- `ChrootPrepared(Ok)` → mantém `ChrootActive`
- `ChrootPrepared(Err)` → `Error`
- `ReportExported` → loga sucesso
- `TerminalResize` → no-op no model (UI trata)
- `Tick` → no-op

**Importante:** Reducer **não** dispara comandos assíncronos diretamente. Comando `RunDiscovery` é disparado pelo `App::run()` ao entrar em `AppState::Discovery`. Comando `ExecuteMount` é disparado pelo `App` ao usuário confirmar em `MountReview`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test app_reducer -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/app/reducer.rs tests/app_reducer_tests.rs
git commit -m "feat(app): reducer puro com transições de estado cobertas por testes"
```

---

### Task 3: Runtime Channels (mpsc Typed)

**Files:**
- Create: `src/runtime/channels.rs`
- Create: `src/runtime/mod.rs` (re-exports)
- Test: `tests/runtime_channels_tests.rs`

**Interfaces:**
- Produces: `CommandChannel (Sender<AppCommand>, Receiver<AppCommand>)`, `EventChannel (Sender<AppEvent>, Receiver<AppEvent>)`, `channel_pair()` helpers

- [ ] **Step 1: Write failing test**

```rust
// tests/runtime_channels_tests.rs
use cachyos_boot_rescue::runtime::{command_channel, event_channel, COMMAND_CHANNEL_BUFFER, EVENT_CHANNEL_BUFFER};
use cachyos_boot_rescue::app::{AppCommand, AppEvent};

#[tokio::test]
async fn test_command_channel_send_recv() {
    let (tx, mut rx) = command_channel(COMMAND_CHANNEL_BUFFER);
    tx.send(AppCommand::RunDiscovery).await.unwrap();
    let cmd = rx.recv().await.unwrap();
    assert!(matches!(cmd, AppCommand::RunDiscovery));
}

#[tokio::test]
async fn test_event_channel_send_recv() {
    let (tx, mut rx) = event_channel(EVENT_CHANNEL_BUFFER);
    tx.send(AppEvent::Tick).await.unwrap();
    let evt = rx.recv().await.unwrap();
    assert!(matches!(evt, AppEvent::Tick));
}
```

- [ ] **Step 2: Run test to verify fail**

Run: `cargo test test_command_channel_send_recv -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement `src/runtime/channels.rs`**

```rust
use tokio::sync::mpsc;
use crate::app::{AppCommand, AppEvent};

pub type CommandSender = mpsc::Sender<AppCommand>;
pub type CommandReceiver = mpsc::Receiver<AppCommand>;
pub type EventSender = mpsc::Sender<AppEvent>;
pub type EventReceiver = mpsc::Receiver<AppEvent>;

pub fn command_channel(buffer: usize) -> (CommandSender, CommandReceiver) {
    mpsc::channel(buffer)
}

pub fn event_channel(buffer: usize) -> (EventSender, EventReceiver) {
    mpsc::channel(buffer)
}

// Buffer sizes per spec: commands 32, events 64
pub const COMMAND_CHANNEL_BUFFER: usize = 32;
pub const EVENT_CHANNEL_BUFFER: usize = 64;
```

- [ ] **Step 4: Run test to verify pass**

Run: `cargo test runtime_channels -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/runtime/channels.rs src/runtime/mod.rs tests/runtime_channels_tests.rs
git commit -m "feat(runtime): canais mpsc tipados para Command/Event"
```

---

### Task 4: TaskManager (Concorrência, Cancelamento, Operações)

**Files:**
- Create: `src/runtime/task_manager.rs`
- Test: `tests/task_manager_tests.rs`

**Interfaces:**
- Consumes: `AppCommand`, `AppEvent`, `OpId`, `OpKind`, `OperationState`, channels (Task 3), operations (Task 5-7)
- Produces: `TaskManager::run()`, `spawn_*()`, `cancel_operation()`, `shutdown_all()`

- [ ] **Step 1: Write failing tests**

```rust
// tests/task_manager_tests.rs
use cachyos_boot_rescue::runtime::{TaskManager, command_channel, event_channel, COMMAND_CHANNEL_BUFFER, EVENT_CHANNEL_BUFFER};
use cachyos_boot_rescue::app::{AppCommand, AppEvent, AppError, OpKind, MountPlan};

fn dummy_mount_plan() -> MountPlan {
    MountPlan {
        root_mount: std::path::PathBuf::from("/mnt/root"),
        entries: vec![],
        required_mounts: vec![],
        warnings: vec![],
    }
}

#[tokio::test]
async fn test_task_manager_blocks_concurrent_mutable_ops() {
    let (cmd_tx, cmd_rx) = command_channel(COMMAND_CHANNEL_BUFFER);
    let (evt_tx, mut evt_rx) = event_channel(EVENT_CHANNEL_BUFFER);
    let mut tm = TaskManager::new(cmd_rx, evt_tx.clone());
    
    // Spawn primeira montagem
    tm.handle_command(AppCommand::ExecuteMount { plan: dummy_mount_plan() });
    
    // Segunda montagem deve ser bloqueada
    tm.handle_command(AppCommand::ExecuteMount { plan: dummy_mount_plan() });
    
    // Deve emitir MountCompleted(Err(OperationBlocked))
    let evt = evt_rx.recv().await.unwrap();
    assert!(matches!(evt, AppEvent::MountCompleted(Err(AppError::OperationBlocked))));
}
```

- [ ] **Step 2: Run tests to verify fail**

Run: `cargo test task_manager -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement `src/runtime/task_manager.rs`**

Estrutura:
```rust
pub struct TaskManager {
    command_rx: CommandReceiver,
    event_tx: EventSender,
    active_operations: HashMap<OpId, OperationHandle>,
    discovery_op_id: Option<OpId>,
    mount_op_id: Option<OpId>,
    chroot_op_id: Option<OpId>,
}

struct OperationHandle {
    id: OpId,
    kind: OpKind,
    abort_handle: tokio::task::AbortHandle,
}

impl TaskManager {
    pub fn new(command_rx: CommandReceiver, event_tx: EventSender) -> Self { ... }
    
    pub async fn run(mut self) { ... }
    
    fn handle_command(&mut self, cmd: AppCommand) {
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
    
    fn can_run(&self, kind: OpKind) -> bool {
        match kind {
            OpKind::Discovery => true,
            OpKind::Mount | OpKind::ChrootCommand => {
                !self.active_operations.values().any(|op| 
                    matches!(op.kind, OpKind::Mount | OpKind::ChrootCommand)
                )
            }
        }
    }
    
    fn spawn_mount(&mut self, plan: MountPlan) {
        if !self.can_run(OpKind::Mount) {
            self.event_tx.send(AppEvent::MountCompleted(Err(AppError::OperationBlocked))).ok();
            return;
        }
        let op_id = OpId::new();
        let (abort_handle, abort_registration) = tokio::task::AbortHandle::new_pair();
        let evt_tx = self.event_tx.clone();
        let handle = tokio::spawn(async move {
            // Chama operations::mount::execute_mount(plan, progress_callback)
        });
        self.active_operations.insert(op_id, OperationHandle { id: op_id, kind: OpKind::Mount, abort_handle });
        self.mount_op_id = Some(op_id);
    }
    
    fn cancel_operation(&mut self, op_id: OpId) {
        if let Some(handle) = self.active_operations.remove(&op_id) {
            handle.abort_handle.abort();
        }
    }
    
    async fn shutdown_all(&mut self) {
        for (_, handle) in self.active_operations.drain() {
            handle.abort_handle.abort();
        }
    }
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test task_manager -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/runtime/task_manager.rs tests/task_manager_tests.rs
git commit -m "feat(runtime): TaskManager com controle de concorrência e cancelamento"
```

---

### Task 5: Operations — Discovery Wrapper

**Files:**
- Create: `src/operations/discovery.rs`
- Create: `src/operations/mod.rs` (re-exports)
- Test: `tests/operations_discovery_tests.rs`

**Interfaces:**
- Consumes: `discovery::run_discovery`, `CommandConfig`, `EventSender`
- Produces: `async fn run_discovery_op(event_tx: EventSender, config: CommandConfig)`

- [ ] **Step 1: Write failing test**

```rust
// tests/operations_discovery_tests.rs
use cachyos_boot_rescue::operations::run_discovery_op;
use cachyos_boot_rescue::runtime::event_channel;
use cachyos_boot_rescue::app::{AppEvent, AppError};

#[tokio::test]
async fn test_run_discovery_op_emits_completed_event() {
    let (tx, mut rx) = event_channel(cachyos_boot_rescue::runtime::EVENT_CHANNEL_BUFFER);
    let config = cachyos_boot_rescue::discovery::default_command_config();
    run_discovery_op(tx, Some(config)).await;
    let evt = rx.recv().await.unwrap();
    assert!(matches!(evt, AppEvent::DiscoveryCompleted(_)));
}
```

- [ ] **Step 2: Run test fail**

- [ ] **Step 3: Implement `src/operations/discovery.rs`**

```rust
use crate::discovery::{run_discovery, default_command_config, CommandConfig};
use crate::app::{AppEvent, AppError};
use crate::runtime::EventSender;

pub async fn run_discovery_op(event_tx: EventSender, config: Option<CommandConfig>) {
    let config = config.unwrap_or_else(default_command_config);
    let result = tokio::task::spawn_blocking(move || run_discovery(Some(config))).await;
    let event = match result {
        Ok(Ok(report)) => AppEvent::DiscoveryCompleted(Ok(report)),
        Ok(Err(e)) => AppEvent::DiscoveryCompleted(Err(AppError::from(e))),
        Err(e) => AppEvent::DiscoveryCompleted(Err(AppError::Internal(e.to_string()))),
    };
    let _ = event_tx.send(event).await;
}
```

- [ ] **Step 4: Run test pass**

- [ ] **Step 5: Commit**

```bash
git add src/operations/discovery.rs src/operations/mod.rs tests/operations_discovery_tests.rs
git commit -m "feat(operations): discovery wrapper assíncrono"
```

---

### Task 6: Operations — Mount (build_mount_plan + MountSession)

**Files:**
- Create: `src/operations/mount.rs`
- Test: `tests/operations_mount_tests.rs`

**Interfaces:**
- Consumes: `storage::build_mount_plan`, `MountSession::mount_all`, `MountPlan`, `MountSession`, `EventSender` (para progress)
- Produces: `async fn execute_mount(event_tx: EventSender, plan: MountPlan, root_mount: PathBuf)`

- [ ] **Step 1: Write failing test**

```rust
// tests/operations_mount_tests.rs
use cachyos_boot_rescue::operations::execute_mount;
use cachyos_boot_rescue::runtime::event_channel;
use cachyos_boot_rescue::app::{AppEvent, MountStage};
use tempfile::tempdir;

#[tokio::test]
async fn test_execute_mount_emits_progress_and_completed() {
    let (tx, mut rx) = event_channel(cachyos_boot_rescue::runtime::EVENT_CHANNEL_BUFFER);
    let plan = cachyos_boot_rescue::storage::MountPlan {
        root_mount: std::path::PathBuf::from("/mnt/root"),
        entries: vec![],
        required_mounts: vec![],
        warnings: vec![],
    };
    let root = tempdir().unwrap().path().to_path_buf();
    execute_mount(tx, plan, root).await;
    // Verifica MountProgress events + MountCompleted
    let mut progress_count = 0;
    let mut completed = false;
    while let Ok(evt) = rx.try_recv() {
        match evt {
            AppEvent::MountProgress { .. } => progress_count += 1,
            AppEvent::MountCompleted(_) => completed = true,
            _ => {}
        }
    }
    assert!(progress_count > 0);
    assert!(completed);
}
```

- [ ] **Step 2: Run test fail**

- [ ] **Step 3: Implement `src/operations/mount.rs`**

```rust
use crate::storage::{build_mount_plan, MountSession, MountPlan};
use crate::app::{AppEvent, AppError, MountStage};
use crate::runtime::EventSender;
use std::path::PathBuf;

pub async fn execute_mount(event_tx: EventSender, plan: MountPlan, root_mount: PathBuf) {
    let mut session = MountSession::new();
    
    // Stage: Preparing
    let _ = event_tx.send(AppEvent::MountProgress { stage: MountStage::Preparing, percent: 0, message: "Preparando montagem".into() }).await;
    
    // Monta root primeiro
    let _ = event_tx.send(AppEvent::MountProgress { stage: MountStage::MountingRoot, percent: 10, message: "Montando raiz".into() }).await;
    
    // Usa MountSession::mount_all
    let result = tokio::task::spawn_blocking(move || {
        session.mount_all(&plan, &root_mount)
    }).await;
    
    let event = match result {
        Ok(Ok(_)) => {
            let _ = event_tx.send(AppEvent::MountProgress { stage: MountStage::Completed, percent: 100, message: "Montagem concluída".into() }).await;
            AppEvent::MountCompleted(Ok(session))
        }
        Ok(Err(e)) => AppEvent::MountCompleted(Err(AppError::from(e))),
        Err(e) => AppEvent::MountCompleted(Err(AppError::Internal(e.to_string()))),
    };
    let _ = event_tx.send(event).await;
}
```

- [ ] **Step 4: Run test pass**

- [ ] **Step 5: Commit**

```bash
git add src/operations/mount.rs tests/operations_mount_tests.rs
git commit -m "feat(operations): mount wrapper com progresso"
```

---

### Task 7: Operations — Chroot (Task Proprietária)

**Files:**
- Create: `src/operations/chroot.rs`
- Test: `tests/operations_chroot_tests.rs`

**Interfaces:**
- Consumes: `ChrootManager`, `ChrootCommandConfig`, `EventSender`, `CommandReceiver` (para comandos serializados)
- Produces: `async fn run_chroot_task(event_tx: EventSender, cmd_rx: CommandReceiver, root_mount: PathBuf)`

- [ ] **Step 1: Write failing test**

```rust
// tests/operations_chroot_tests.rs
use cachyos_boot_rescue::operations::run_chroot_task;
use cachyos_boot_rescue::runtime::{event_channel, command_channel};
use cachyos_boot_rescue::app::{AppEvent, AppCommand};
use tempfile::tempdir;

#[tokio::test]
async fn test_chroot_task_prepare_execute_cleanup() {
    let (evt_tx, mut evt_rx) = event_channel(cachyos_boot_rescue::runtime::EVENT_CHANNEL_BUFFER);
    let (cmd_tx, cmd_rx) = command_channel(cachyos_boot_rescue::runtime::COMMAND_CHANNEL_BUFFER);
    let root = tempdir().unwrap().path().to_path_buf();
    
    let handle = tokio::spawn(run_chroot_task(evt_tx, cmd_rx, root));
    
    cmd_tx.send(AppCommand::ExecuteChrootCommand { command: vec!["prepare".into()] }).await.unwrap();
    // Aguarda ChrootPrepared
    let evt = evt_rx.recv().await.unwrap();
    assert!(matches!(evt, AppEvent::ChrootPrepared(Ok(_))));
    
    handle.await.unwrap();
}
```

- [ ] **Step 2: Run test fail**

- [ ] **Step 3: Implement `src/operations/chroot.rs`**

```rust
use crate::chroot::{ChrootManager, ChrootCommandConfig, ChrootCommandResult};
use crate::app::{AppEvent, AppError, AppCommand};
use crate::runtime::{EventSender, CommandReceiver};
use std::path::PathBuf;

pub async fn run_chroot_task(event_tx: EventSender, mut cmd_rx: CommandReceiver, root_mount: PathBuf) {
    let mut manager = ChrootManager::new(root_mount);
    
    match manager.prepare() {
        Ok(_) => { let _ = event_tx.send(AppEvent::ChrootPrepared(Ok(()))).await; }
        Err(e) => { let _ = event_tx.send(AppEvent::ChrootPrepared(Err(AppError::from(e)))).await; return; }
    }
    
    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            AppCommand::ExecuteChrootCommand { command } => {
                let config = ChrootCommandConfig::default();
                let result = manager.execute(&command, Some(config));
                let event = match result {
                    Ok(res) => AppEvent::ChrootCommandCompleted(Ok(format!("exit={}, stdout={}", res.exit_code, res.stdout))),
                    Err(e) => AppEvent::ChrootCommandCompleted(Err(AppError::from(e))),
                };
                let _ = event_tx.send(event).await;
            }
            AppCommand::Quit => break,
            _ => {}
        }
    }
    
    let cleanup_result = manager.cleanup();
    let _ = event_tx.send(AppEvent::ChrootCleanedUp(cleanup_result.map_err(AppError::from))).await;
}
```

**Importante:** Esta task é **proprietária** do `ChrootManager`. O `TaskManager` mantém o `CommandSender` para esta task e serializa todos os `ExecuteChrootCommand` através dele.

- [ ] **Step 4: Run test pass**

- [ ] **Step 5: Commit**

```bash
git add src/operations/chroot.rs tests/operations_chroot_tests.rs
git commit -m "feat(operations): chroot task proprietária com ciclo prepare/execute/cleanup"
```

---

### Task 8: UI Components (Widgets Reutilizáveis)

**Files:**
- Create: `src/ui/components/mod.rs`
- Create: `src/ui/components/list_view.rs`
- Create: `src/ui/components/table_view.rs`
- Create: `src/ui/components/progress_bar.rs`
- Create: `src/ui/components/confirm_dialog.rs`
- Create: `src/ui/components/error_dialog.rs`
- Create: `src/ui/components/log_view.rs`
- Create: `src/ui/components/help_footer.rs`

**Interfaces:**
- Produces: widgets Ratatui `Widget` + `StatefulWidget` com APIs limpas

- [ ] **Step 1: Implement components sem testes unitários (testados via integração UI)**

Cada componente: `fn render(area: Rect, buf: &mut Buffer, state: &mut State)` ou `render_ref(&self, area, buf)`.

**ListView:** Navegação ↑↓, Enter, Esc, seleção única, scroll.
**TableView:** Colunas fixas, header, scroll horizontal/vertical.
**ProgressBar:** Stage label + percent + bar + message.
**ConfirmDialog:** "Tem certeza?" + [Não] [Sim] (foco em Não).
**ErrorDialog:** Mensagem + session_id + [Exportar] [Voltar].
**LogView:** Lista eventos, filtro, scroll, tecla 'e' exporta.
**HelpFooter:** Teclas contextuais por tela.

- [ ] **Step 2: Commit**

```bash
git add src/ui/components/
git commit -m "feat(ui): componentes reutilizáveis (list, table, progress, dialogs, log, help)"
```

---

### Task 9: UI Layout Principal

**Files:**
- Create: `src/ui/layout.rs`
- Create: `src/ui/mod.rs` (re-exports)

**Interfaces:**
- Produces: `fn draw_main_layout(f: &mut Frame, app: &AppModel, focused_screen: &mut dyn Screen)`

- [ ] **Step 1: Implement layout**

```rust
pub fn draw_main_layout(f: &mut Frame, app: &AppModel, screen: &mut dyn Screen) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Body (screen)
            Constraint::Length(1),  // Separator
            Constraint::Length(3),  // Footer/Help
        ])
        .split(f.area());
    
    // Header: título + session_id + estado firmware
    // Body: delega para screen.render(chunks[1], f)
    // Footer: HelpFooter com teclas da tela atual
}

pub trait Screen {
    fn render(&mut self, area: Rect, f: &mut Frame, app: &AppModel);
    fn handle_input(&mut self, key: crossterm::event::KeyEvent, app: &AppModel) -> Option<AppCommand>;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/ui/layout.rs src/ui/mod.rs
git commit -m "feat(ui): layout principal com header/body/footer"
```

---

### Task 10: Screens (Telas)

**Files:**
- Create: `src/ui/screens/welcome.rs`
- Create: `src/ui/screens/discovery.rs`
- Create: `src/ui/screens/install_select.rs`
- Create: `src/ui/screens/mount_review.rs`
- Create: `src/ui/screens/mount_progress.rs`
- Create: `src/ui/screens/chroot.rs`
- Create: `src/ui/screens/logs.rs`
- Create: `src/ui/screens/mod.rs` (re-exports + Screen trait)

**Interfaces:**
- Consumes: `AppModel`, UI components, `Screen` trait
- Produces: `render()`, `handle_input()` para cada tela

- [ ] **Step 1: Implement cada tela**

| Tela | Inputs Principais | Ações (geram AppCommand) |
|------|-------------------|--------------------------|
| Welcome | Enter/Esc | `RunDiscovery` (Enter), `Quit` (Esc/q) |
| Discovery | (auto) | Nenhum (auto-inicia discovery ao entrar) |
| InstallSelect | ↑↓, Enter, Esc | `BuildMountPlan { candidate_idx }` (Enter), `RunDiscovery` (r), `Quit` (Esc) |
| MountReview | ↑↓ (entries), y/n, Esc | `ExecuteMount { plan }` (y), `RunDiscovery` (r), `Quit` (Esc) |
| MountProgress | Esc | `CancelOperation { op_id }` (Esc) |
| ChrootActive | Teclado livre, Ctrl+D/Esc | `ExecuteChrootCommand` (cada linha), `Quit` (Ctrl+D) |
| LogsReport | ↑↓, /, e, Esc | `ExportReport` (e), navegação |

**MountReview — Crítica:** Mostra:
- Mounts já existentes (do `DiscoveryReport` + `findmnt`)
- Mounts planejados (do `MountPlan.entries`)
- Flags: `noauto` (skip), `nofail` (warning se device ausente), `ambiguous` (blocked)
- Subvolumes Btrfs detectados
- Confirmação explícita "Digite 'sim' para confirmar" ou dialog Y/N com foco em Não

**MountProgress:** Recebe `MountProgress` events via `AppModel` (atualizado pelo reducer), mostra stage/percent/message.

- [ ] **Step 2: Commit**

```bash
git add src/ui/screens/
git commit -m "feat(ui): todas as telas (welcome, discovery, install_select, mount_review, mount_progress, chroot, logs)"
```

---

### Task 11: App Principal (Loop, Terminal, Integração)

**Files:**
- Modify: `src/app/app.rs` (já existe vazio na Fase 1)
- Modify: `src/main.rs` (já existe)

**Interfaces:**
- Consumes: `AppModel`, `reducer`, `TaskManager`, `channels`, `ui::layout`, `ui::screens`, `crossterm`, `ratatui`
- Produces: `App::run()` loop principal

- [ ] **Step 1: Implement `src/app/app.rs`**

```rust
pub struct App {
    model: AppModel,
    task_manager: TaskManager,
    command_tx: CommandSender,
    event_rx: EventReceiver,
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    current_screen: Box<dyn Screen>,
}

impl App {
    pub async fn run() -> Result<()> {
        // 1. Setup terminal (crossterm: raw mode, alternate screen, mouse capture)
        // 2. Cria canais
        // 3. Spawn TaskManager::run() em task separada
        // 4. Loop principal:
        //    loop {
        //        terminal.draw(|f| ui::layout::draw_main_layout(f, &model, current_screen))?;
        //        
        //        // Handle events (crossterm::event::poll + read)
        //        if event::poll(Duration::from_millis(50))? {
        //            match event::read()? {
        //                Event::Key(key) => {
        //                    let cmd = current_screen.handle_input(key, &model);
        //                    if let Some(cmd) = cmd { command_tx.send(cmd).await?; }
        //                }
        //                Event::Resize(_, _) => { command_tx.send(AppCommand::TerminalResize).await?; }
        //                _ => {}
        //            }
        //        }
        //        
        //        // Processa eventos do TaskManager
        //        while let Ok(event) = event_rx.try_recv() {
        //            let (new_model, cmds) = reduce(model, event);
        //            model = new_model;
        //            for cmd in cmds { command_tx.send(cmd).await?; }
        //            
        //            // Atualiza current_screen baseado em model.state
        //            current_screen = Screen::from_state(&model.state);
        //        }
        //        
        //        // Tick periódico para timeouts
        //        if tick_elapsed { command_tx.send(AppCommand::Tick).await?; }
        //        
        //        // Verifica quit
        //        if matches!(model.state, AppState::Quit) { break; }
        //    }
        // 5. Cleanup: task_manager.shutdown_all().await; terminal.restore()
    }
}
```

**Panic Hook:** No início de `run()`, instala `std::panic::set_hook` que restaura terminal e loga.

- [ ] **Step 2: Atualiza `src/main.rs` para chamar `app::run().await`**

- [ ] **Step 3: Commit**

```bash
git add src/app/app.rs src/main.rs
git commit -m "feat(app): loop principal com terminal, TaskManager, reducer, UI"
```

---

### Task 12: Integração Completa + Testes E2E Manuais

**Files:**
- Test: `tests/integration_tui_tests.rs` (smoke test: app inicia, navega Welcome→Discovery→InstallSelect sem travar)

**Interfaces:**
- Consumes: tudo acima

- [ ] **Step 1: Smoke test**

```rust
// tests/integration_tui_tests.rs
#[tokio::test]
async fn test_app_starts_and_navigates_to_install_select() {
    // Inicia App em background, simula teclas Enter, Enter
    // Verifica que state chega em InstallSelect
    // Não testa terminal real — testa model + reducer + task_manager
}
```

- [ ] **Step 2: Testes manuais (checklist)**

- [ ] `cargo run` → Welcome screen renderiza
- [ ] Enter → Discovery roda (spinner/logs) → InstallSelect com candidatos
- [ ] ↑↓ seleciona candidato → Enter → MountReview mostra plano + existentes + flags
- [ ] 'y' → MountProgress mostra stages → ChrootActive
- [ ] Ctrl+D → Volta para MountReview
- [ ] 'r' → Re-faz discovery
- [ ] Esc em qualquer tela → Volta/Welcome/Quit
- [ ] Resize terminal → UI adapta
- [ ] Ctrl+C → Terminal restaurado, mounts limpos
- [ ] Tela Error (simula falha) → mostra session_id + [Exportar] funciona

- [ ] **Step 3: Quality gates**

Run: `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-targets && cargo build --locked`
Expected: ALL PASS

- [ ] **Step 4: Commit**

```bash
git add tests/integration_tui_tests.rs
git commit -m "test: smoke test integração TUI + quality gates pass"
```

---

## Execution Order & Dependencies

```
Task 0 (Deps) 
    → Task 1 (Core Types) 
    → Task 2 (Reducer) 
    → Task 3 (Channels)
    → Task 4 (TaskManager) 
    → Task 5 (Ops Discovery) 
    → Task 6 (Ops Mount) 
    → Task 7 (Ops Chroot)
    → Task 8 (UI Components)
    → Task 9 (UI Layout)
    → Task 10 (UI Screens)
    → Task 11 (App Loop)
    → Task 12 (Integration + Quality Gates)
```

Tasks 8-10 (UI) podem ser paralelas após Task 1-3 (tipos/canais), mas Task 11 precisa de todas.

---

## Estimativa de Esforço

| Task | Complexidade | Arquivos |
|------|--------------|----------|
| 0 Deps | Baixa | 1 |
| 1 Core Types | Baixa | 5 |
| 2 Reducer | Média | 2 |
| 3 Channels | Baixa | 2 |
| 4 TaskManager | Alta | 2 |
| 5 Ops Discovery | Baixa | 2 |
| 6 Ops Mount | Média | 2 |
| 7 Ops Chroot | Média | 2 |
| 8 UI Components | Alta | 8 |
| 9 UI Layout | Baixa | 2 |
| 10 UI Screens | Alta | 8 |
| 11 App Loop | Média | 2 |
| 12 Integration | Baixa | 1 |
| **Total** | | **39 arquivos novos/modificados** |

---

## Definition of Done (Fase 5)

- [ ] Todos os 12 tasks completos com testes passando
- [ ] `cargo fmt --check` ✅
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` ✅
- [ ] `cargo test --all-targets` ✅ (inclui reducer tests, task_manager tests, integration smoke)
- [ ] `cargo build --locked` ✅
- [ ] Checklist manual Task 12 ✅
- [ ] Documentação atualizada: `Docs/modules/tui.md`, `Docs/architecture/OVERVIEW.md`, `Docs/ai/CURRENT_STATUS.md`
- [ ] Terminal restaurado em todos os caminhos de saída (verificado manualmente)
- [ ] Nenhuma operação mutável simultânea possível (verificado via teste Task 4)
- [ ] ChrootManager com propriedade única (verificado via código Task 7)

---

**Plano salvo em:** `Docs/superpowers/plans/2026-11-09-phase5-tui-implementation.md`

**Próximo:** Aguardar sua revisão e escolha de execução (Subagent-driven vs Native).