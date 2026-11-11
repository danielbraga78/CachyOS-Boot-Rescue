//! Gerenciador de operações assíncronas e exclusão mútua de operações de escrita.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    app::{AppCommand, AppEvent, OpId, OpKind},
    diagnostic::{runner::run_diagnostic_op, types::DiagnosticConfig},
    discovery::{self, types::DiscoveryReport},
    error::Error,
    logging,
    operations::{execute_mount, run_chroot_task},
    runtime::{CommandReceiver, CommandSender, EventSender, command_channel},
    storage::{self, MountPlan},
};

struct OperationHandle {
    kind: OpKind,
    task: tokio::task::JoinHandle<()>,
}

/// Gerencia comandos da UI e mantém o `ChrootManager` em uma única task.
pub struct TaskManager {
    command_rx: CommandReceiver,
    event_tx: EventSender,
    active_operations: HashMap<OpId, OperationHandle>,
    finished_tx: tokio::sync::mpsc::UnboundedSender<OpId>,
    finished_rx: tokio::sync::mpsc::UnboundedReceiver<OpId>,
    discovery_report: Arc<Mutex<Option<DiscoveryReport>>>,
    root_mount: PathBuf,
    chroot_command_tx: Arc<Mutex<Option<CommandSender>>>,
    chroot_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl TaskManager {
    /// Cria um novo gerenciador com um ponto de montagem exclusivo por sessão.
    pub fn new(command_rx: CommandReceiver, event_tx: EventSender) -> Self {
        let session_id = uuid::Uuid::new_v4();
        let (finished_tx, finished_rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            command_rx,
            event_tx,
            active_operations: HashMap::new(),
            finished_tx,
            finished_rx,
            discovery_report: Arc::new(Mutex::new(None)),
            root_mount: PathBuf::from("/mnt/cachyos-boot-rescue").join(session_id.to_string()),
            chroot_command_tx: Arc::new(Mutex::new(None)),
            chroot_task: Arc::new(Mutex::new(None)),
        }
    }

    /// Executa o loop do TaskManager e limpa as tasks ao encerrar.
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                command = self.command_rx.recv() => {
                    match command {
                        Some(AppCommand::Quit) | None => break,
                        Some(command) => self.handle_command(command).await,
                    }
                }
                Some(op_id) = self.finished_rx.recv() => {
                    self.active_operations.remove(&op_id);
                }
            }
        }
        self.shutdown_all().await;
    }

    /// Processa um comando recebido.
    pub async fn handle_command(&mut self, command: AppCommand) {
        match command {
            AppCommand::RunDiscovery => self.spawn_discovery(),
            AppCommand::BuildMountPlan { candidate_idx } => {
                self.spawn_build_mount_plan(candidate_idx)
            }
            AppCommand::ExecuteMount { plan } => self.spawn_mount(plan),
            AppCommand::ExecuteChrootCommand { command } => self.send_chroot_command(command).await,
            AppCommand::RunDiagnostic {
                candidate_idx,
                config,
            } => self.spawn_diagnostic(candidate_idx, config),
            AppCommand::CancelOperation { op_id } => self.cancel_operation(op_id),
            AppCommand::ExportReport { output_path } => self.spawn_export_report(output_path),
            AppCommand::LeaveChroot => self.leave_chroot().await,
            AppCommand::TerminalResize | AppCommand::Tick => {}
            AppCommand::Quit => self.shutdown_all().await,
        }
    }

    fn can_run(&self, kind: OpKind) -> bool {
        let mount_running = self
            .active_operations
            .values()
            .any(|operation| operation.kind == OpKind::Mount);
        let chroot_running = self
            .active_operations
            .values()
            .any(|operation| operation.kind == OpKind::ChrootCommand);
        match kind {
            OpKind::Discovery => true,
            OpKind::Diagnostic => !mount_running && !chroot_running,
            OpKind::Mount => {
                !mount_running
                    && !chroot_running
                    && self
                        .chroot_command_tx
                        .lock()
                        .map(|sender| sender.is_none())
                        .unwrap_or(false)
            }
            OpKind::ChrootCommand => !mount_running,
        }
    }

    fn spawn_tracked<F>(&mut self, op_id: OpId, kind: OpKind, operation: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let finished_tx = self.finished_tx.clone();
        let task = tokio::spawn(async move {
            operation.await;
            let _ = finished_tx.send(op_id);
        });
        self.active_operations
            .insert(op_id, OperationHandle { kind, task });
    }

    fn spawn_discovery(&mut self) {
        let event_tx = self.event_tx.clone();
        let report_slot = Arc::clone(&self.discovery_report);
        let op_id = OpId::new();

        self.spawn_tracked(op_id, OpKind::Discovery, async move {
            let result = tokio::task::spawn_blocking(|| discovery::run_discovery(None)).await;
            let result = match result {
                Ok(result) => result,
                Err(error) => Err(Error::internal(error.to_string())),
            };
            if let Ok(report) = &result {
                // Lock, update, and release before any await
                if let Ok(mut slot) = report_slot.lock() {
                    *slot = Some(report.clone());
                } else {
                    let _ = event_tx
                        .send(AppEvent::DiscoveryCompleted(Err(Error::internal(
                            "Falha ao atualizar o resultado da descoberta",
                        ))))
                        .await;
                    return;
                }
            }
            let _ = event_tx.send(AppEvent::DiscoveryCompleted(result)).await;
        });
    }

    fn spawn_build_mount_plan(&mut self, candidate_idx: usize) {
        let report = match self.discovery_report.lock() {
            Ok(report) => report.clone(),
            Err(_) => {
                let _ = self
                    .event_tx
                    .try_send(AppEvent::MountPlanCompleted(Err(Error::internal(
                        "Falha ao acessar o resultado da descoberta",
                    ))));
                return;
            }
        };
        let Some(report) = report else {
            let _ =
                self.event_tx
                    .try_send(AppEvent::MountPlanCompleted(Err(Error::invalid_state(
                        "Execute a descoberta antes de criar o plano",
                        None,
                    ))));
            return;
        };
        let Some(candidate) = report.linux_candidates.get(candidate_idx).cloned() else {
            let _ = self
                .event_tx
                .try_send(AppEvent::MountPlanCompleted(Err(Error::validation(
                    "Instalação selecionada não existe",
                    None,
                ))));
            return;
        };

        let event_tx = self.event_tx.clone();
        let root_mount = self.root_mount.clone();
        let op_id = OpId::new();
        self.spawn_tracked(op_id, OpKind::Discovery, async move {
            let result = tokio::task::spawn_blocking(move || {
                storage::build_mount_plan(
                    &candidate,
                    &report.devices,
                    candidate.fstab_content.as_deref(),
                    &root_mount,
                )
            })
            .await
            .map_err(|error| Error::internal(error.to_string()));
            let _ = event_tx.send(AppEvent::MountPlanCompleted(result)).await;
        });
    }

    fn spawn_diagnostic(&mut self, candidate_idx: usize, config: DiagnosticConfig) {
        let report =
            match self.discovery_report.lock() {
                Ok(report) => report.clone(),
                Err(_) => {
                    let _ = self.event_tx.try_send(AppEvent::DiagnosticCompleted(Err(
                        Error::internal("Falha ao acessar o resultado da descoberta"),
                    )));
                    return;
                }
            };
        let Some(report) = report else {
            let _ =
                self.event_tx
                    .try_send(AppEvent::DiagnosticCompleted(Err(Error::invalid_state(
                        "Execute a descoberta antes do diagnóstico",
                        None,
                    ))));
            return;
        };
        if report.linux_candidates.get(candidate_idx).is_none() {
            let _ = self
                .event_tx
                .try_send(AppEvent::DiagnosticCompleted(Err(Error::validation(
                    "Instalação selecionada não existe",
                    None,
                ))));
            return;
        }

        // Obtém a sessão de montagem atual se houver
        let mount_session = self.get_mount_session().cloned();

        let event_tx = self.event_tx.clone();
        let op_id = OpId::new();
        self.spawn_tracked(op_id, OpKind::Diagnostic, async move {
            run_diagnostic_op(event_tx, report, candidate_idx, mount_session, config).await;
        });
    }

    /// Obtém a sessão de montagem ativa se houver (simplificado - retorna None por enquanto)
    fn get_mount_session(&self) -> Option<&crate::storage::MountSession> {
        // Em uma implementação completa, isso viria do estado compartilhado
        // Por enquanto retorna None - diagnóstico funciona sem chroot
        None
    }

    fn spawn_mount(&mut self, plan: MountPlan) {
        if !self.can_run(OpKind::Mount) {
            let _ =
                self.event_tx
                    .try_send(AppEvent::MountCompleted(Err(Error::operation_blocked(
                        "outra operação mutável está em andamento",
                    ))));
            return;
        }
        if plan.has_blocking_errors() {
            let _ = self
                .event_tx
                .try_send(AppEvent::MountCompleted(Err(Error::validation(
                    "O plano contém erros bloqueantes",
                    None,
                ))));
            return;
        }

        let event_tx = self.event_tx.clone();
        let root_mount = self.root_mount.clone();
        let chroot_task = Arc::clone(&self.chroot_task);
        let chroot_event_tx = self.event_tx.clone();
        let op_id = OpId::new();
        let (chroot_tx, chroot_rx) = command_channel(crate::runtime::COMMAND_CHANNEL_BUFFER);
        if let Ok(mut sender) = self.chroot_command_tx.lock() {
            *sender = Some(chroot_tx);
        }

        self.spawn_tracked(op_id, OpKind::Mount, async move {
            // Execute mount and get result
            let mount_result = execute_mount(event_tx, plan, root_mount.clone()).await;

            // Only spawn chroot task if mount succeeded
            if let Ok(_session) = mount_result {
                let handle = tokio::spawn(run_chroot_task(chroot_event_tx, chroot_rx, root_mount));
                if let Ok(mut task) = chroot_task.lock() {
                    *task = Some(handle);
                }
            }
            // If mount failed, the error event was already sent by execute_mount,
            // and we don't spawn the chroot task.
        });
    }

    async fn send_chroot_command(&mut self, command: Vec<String>) {
        if !self.can_run(OpKind::ChrootCommand) {
            let _ = self
                .event_tx
                .send(AppEvent::ChrootCommandCompleted(Err(
                    Error::operation_blocked("uma montagem ainda está em andamento"),
                )))
                .await;
            return;
        }

        let sender = self
            .chroot_command_tx
            .lock()
            .ok()
            .and_then(|sender| sender.clone());
        match sender {
            Some(sender) => {
                if let Err(error) = sender
                    .send(AppCommand::ExecuteChrootCommand { command })
                    .await
                {
                    if let Ok(mut current) = self.chroot_command_tx.lock() {
                        *current = None;
                    }
                    let _ = self
                        .event_tx
                        .send(AppEvent::ChrootCommandCompleted(Err(Error::internal(
                            error.to_string(),
                        ))))
                        .await;
                }
            }
            None => {
                let _ = self
                    .event_tx
                    .send(AppEvent::ChrootCommandCompleted(Err(Error::invalid_state(
                        "O chroot não está preparado",
                        None,
                    ))))
                    .await;
            }
        }
    }

    fn cancel_operation(&mut self, op_id: OpId) {
        let Some(operation) = self.active_operations.remove(&op_id) else {
            return;
        };
        match operation.kind {
            OpKind::Discovery => {
                operation.task.abort();
                let _ = self
                    .event_tx
                    .try_send(AppEvent::DiscoveryCompleted(Err(Error::internal(
                        "Descoberta cancelada",
                    ))));
            }
            OpKind::Mount => {
                operation.task.abort();
                let _ = self
                    .event_tx
                    .try_send(AppEvent::MountCompleted(Err(Error::internal(
                        "Montagem cancelada",
                    ))));
            }
            OpKind::ChrootCommand => {
                operation.task.abort();
                let _ =
                    self.event_tx
                        .try_send(AppEvent::ChrootCommandCompleted(Err(Error::internal(
                            "Comando chroot cancelado",
                        ))));
            }
            OpKind::Diagnostic => {
                operation.task.abort();
                let _ =
                    self.event_tx
                        .try_send(AppEvent::DiagnosticCompleted(Err(Error::internal(
                            "Diagnóstico cancelado",
                        ))));
            }
        }
    }

    fn spawn_export_report(&mut self, output_path: PathBuf) {
        let event_tx = self.event_tx.clone();
        tokio::spawn(async move {
            let result =
                tokio::task::spawn_blocking(move || logging::export_session_report(&output_path))
                    .await
                    .map_err(|error| Error::internal(error.to_string()))
                    .and_then(|result| result);
            let _ = event_tx.send(AppEvent::ReportExported(result)).await;
        });
    }

    async fn leave_chroot(&mut self) {
        let sender = self
            .chroot_command_tx
            .lock()
            .ok()
            .and_then(|sender| sender.clone());
        if let Some(sender) = sender {
            let _ = sender.send(AppCommand::Quit).await;
        }
        let task = self
            .chroot_task
            .lock()
            .ok()
            .and_then(|mut task| task.take());
        if let Some(task) = task
            && let Err(error) = task.await
        {
            let _ = self
                .event_tx
                .send(AppEvent::ChrootCleanedUp(Err(Error::internal(
                    error.to_string(),
                ))))
                .await;
        }
        if let Ok(mut sender) = self.chroot_command_tx.lock() {
            *sender = None;
        }
    }

    async fn shutdown_all(&mut self) {
        let active = self.active_operations.drain().collect::<Vec<_>>();
        for (_, operation) in active {
            if operation.kind == OpKind::Discovery {
                operation.task.abort();
            }
            if let Err(error) = operation.task.await
                && !error.is_cancelled()
            {
                tracing::error!(error = %error, "Falha ao finalizar operação");
            }
        }
        self.leave_chroot().await;
    }
}
