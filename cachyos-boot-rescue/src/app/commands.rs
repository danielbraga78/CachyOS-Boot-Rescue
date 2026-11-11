//! Comandos da aplicação (solicitações de execução).

use crate::app::model::OpId;
use crate::diagnostic::types::DiagnosticConfig;
use std::path::PathBuf;

/// Comandos que podem ser despachados para o TaskManager.
#[derive(Debug, Clone)]
pub enum AppCommand {
    /// Inicia descoberta de hardware/instalações.
    RunDiscovery,
    /// Constrói plano de montagem para o candidato selecionado.
    BuildMountPlan { candidate_idx: usize },
    /// Executa montagem com o plano fornecido.
    ExecuteMount { plan: crate::storage::MountPlan },
    /// Executa comando no chroot.
    ExecuteChrootCommand { command: Vec<String> },
    /// Executa diagnóstico de bootloader para o candidato selecionado.
    RunDiagnostic {
        candidate_idx: usize,
        config: DiagnosticConfig,
    },
    /// Cancela operação em andamento.
    CancelOperation { op_id: OpId },
    /// Exporta relatório sanitizado.
    ExportReport { output_path: PathBuf },
    /// Sai da aplicação.
    Quit,
    /// Encerra o chroot atual sem encerrar a aplicação.
    LeaveChroot,
    /// Evento de redimensionamento do terminal.
    TerminalResize,
    /// Tick periódico para timeouts/animations.
    Tick,
}
