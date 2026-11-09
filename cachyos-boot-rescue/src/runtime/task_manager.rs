//! Gerenciador de tarefas assíncronas com controle de concorrência.

use std::collections::HashMap;

use crate::app::{AppCommand, AppEvent, OpId, OpKind};
use crate::runtime::{CommandReceiver, EventSender};
use crate::storage::MountPlan;

/// Handle para uma operação ativa.
struct OperationHandle {
    id: OpId,
    kind: OpKind,
    abort_handle: tokio::task::AbortHandle,
}

/// Gerenciador de tarefas que controla concorrência e cancelamento.
pub struct TaskManager {
    command_rx: CommandReceiver,
    event_tx: EventSender,
    active_operations: HashMap<OpId, OperationHandle>,
    discovery_op_id: Option<OpId>,
    mount_op_id: Option<OpId>,
    chroot_op_id: Option<OpId>,
}

impl TaskManager {
    /// Cria um novo TaskManager.
    pub fn new(command_rx: CommandReceiver, event_tx: EventSender) -> Self {
        Self {
            command_rx,
            event_tx,
            active_operations: HashMap::new(),
            discovery_op_id: None,
            mount_op_id: None,
            chroot_op_id: None,
        }
    }

    /// Executa o loop principal do TaskManager.
    pub async fn run(mut self) {
        while let Some(cmd) = self.command_rx.recv().await {
            self.handle_command(cmd);
        }
    }

    /// Processa um comando recebido.
    pub async fn handle_command(&mut self, cmd: AppCommand) {
        match cmd {
            AppCommand::RunDiscovery => self.spawn_discovery(),
            AppCommand::BuildMountPlan { candidate_idx } => self.spawn_build_mount_plan(candidate_idx),
            AppCommand::ExecuteMount { plan } => self.spawn_mount(plan),
            AppCommand::ExecuteChrootCommand { command } => self.spawn_chroot_command(command),
            AppCommand::CancelOperation { op_id } => self.cancel_operation(op_id),
            AppCommand::ExportReport { output_path } => self.spawn_export_report(output_path),
            AppCommand::Quit => self.shutdown_all().await,
            AppCommand::TerminalResize => {
                // Ignorado no TaskManager - tratado pela UI
            }
            AppCommand::Tick => {
                // Ignorado no TaskManager - tratado pela UI
            }
        }
    }

    /// Verifica se uma operação do tipo especificado pode rodar.
    fn can_run(&self, kind: OpKind) -> bool {
        match kind {
            OpKind::Discovery => true, // Múltiplos discovery OK
            OpKind::Mount | OpKind::ChrootCommand => {
                // Apenas UMA operação mutável por vez
                !self.active_operations.values().any(|op| 
                    matches!(op.kind, OpKind::Mount | OpKind::ChrootCommand)
                )
            }
        }
    }

    /// Inicia operação de discovery.
    fn spawn_discovery(&mut self) {
        let op_id = OpId::new();
        let evt_tx = self.event_tx.clone();
        
        let handle = tokio::spawn(async move {
            // Discovery será implementado em Task 5
            let _ = evt_tx.send(AppEvent::DiscoveryCompleted(Err(crate::error::Error::internal("não implementado")))).await;
        });
        
        let abort_handle = handle.abort_handle();
        self.active_operations.insert(op_id, OperationHandle { id: op_id, kind: OpKind::Discovery, abort_handle });
        self.discovery_op_id = Some(op_id);
    }

    /// Inicia construção do plano de montagem.
    fn spawn_build_mount_plan(&mut self, _candidate_idx: usize) {
        // BuildMountPlan será implementado em Task 5
        let _ = self.event_tx.send(AppEvent::MountPlanCompleted(Err(crate::error::Error::internal("não implementado"))));
    }

    /// Inicia operação de montagem.
    fn spawn_mount(&mut self, _plan: MountPlan) {
        if !self.can_run(OpKind::Mount) {
            let _ = self.event_tx.try_send(AppEvent::MountCompleted(Err(crate::error::Error::operation_blocked("outra operação mutável em andamento"))));
            return;
        }
        let op_id = OpId::new();
        let evt_tx = self.event_tx.clone();
        
        let handle = tokio::spawn(async move {
            // Montagem será implementada em Task 6
            let _ = evt_tx.send(AppEvent::MountCompleted(Err(crate::error::Error::internal("não implementado")))).await;
        });
        
        let abort_handle = handle.abort_handle();
        self.active_operations.insert(op_id, OperationHandle { id: op_id, kind: OpKind::Mount, abort_handle });
        self.mount_op_id = Some(op_id);
    }

    /// Inicia comando chroot.
    fn spawn_chroot_command(&mut self, _command: Vec<String>) {
        if !self.can_run(OpKind::ChrootCommand) {
            let _ = self.event_tx.send(AppEvent::ChrootCommandCompleted(Err(crate::error::Error::operation_blocked("outra operação mutável em andamento"))));
            return;
        }
        let op_id = OpId::new();
        let evt_tx = self.event_tx.clone();
        
        let handle = tokio::spawn(async move {
            // Chroot será implementado em Task 7
            let _ = evt_tx.send(AppEvent::ChrootCommandCompleted(Err(crate::error::Error::internal("não implementado")))).await;
        });
        
        let abort_handle = handle.abort_handle();
        self.active_operations.insert(op_id, OperationHandle { id: op_id, kind: OpKind::ChrootCommand, abort_handle });
        self.chroot_op_id = Some(op_id);
    }

    /// Cancela uma operação em andamento.
    fn cancel_operation(&mut self, op_id: OpId) {
        if let Some(handle) = self.active_operations.remove(&op_id) {
            handle.abort_handle.abort();
            // Limpar referência
            if self.discovery_op_id == Some(op_id) {
                self.discovery_op_id = None;
            }
            if self.mount_op_id == Some(op_id) {
                self.mount_op_id = None;
            }
            if self.chroot_op_id == Some(op_id) {
                self.chroot_op_id = None;
            }
        }
    }

    /// Inicia exportação de relatório.
    fn spawn_export_report(&mut self, _output_path: std::path::PathBuf) {
        let evt_tx = self.event_tx.clone();
        tokio::spawn(async move {
            // Exportação será implementada posteriormente
            let _ = evt_tx.send(AppEvent::ReportExported(Err(crate::error::Error::internal("não implementado")))).await;
        });
    }

    /// Encerra todas as operações ativas.
    async fn shutdown_all(&mut self) {
        for (_, handle) in self.active_operations.drain() {
            handle.abort_handle.abort();
        }
    }
}