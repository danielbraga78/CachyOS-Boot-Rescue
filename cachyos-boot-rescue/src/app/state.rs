//! Estados de navegação da aplicação.

use crate::app::model::{MountStage, OpId, OpKind};
use crate::chroot::ChrootManager;
use crate::discovery::types::DiscoveryReport;
use crate::error::Error;
use crate::storage::MountPlan;
use crate::storage::MountSession;

/// Estado principal de navegação da aplicação.
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    /// Tela inicial de boas-vindas.
    Welcome,
    /// Tela de descoberta (auto-inicia discovery).
    Discovery,
    /// Tela de seleção de instalação Linux.
    InstallSelect,
    /// Tela de revisão do plano de montagem.
    MountReview,
    /// Tela de progresso da montagem.
    MountProgress { stage: MountStage },
    /// Tela de chroot ativo.
    ChrootActive,
    /// Tela de logs e relatórios.
    LogsReport,
    /// Tela de erro.
    Error { message: String },
    /// Encerramento ordenado da aplicação.
    Quit,
}

/// Dados compartilhados entre telas.
#[derive(Debug, Default)]
pub struct SharedData {
    /// Relatório de descoberta.
    pub discovery_report: Option<DiscoveryReport>,
    /// Plano de montagem.
    pub mount_plan: Option<MountPlan>,
    /// Sessão de montagem ativa.
    pub mount_session: Option<MountSession>,
    /// Gerenciador de chroot (propriedade única).
    pub chroot_manager: Option<ChrootManager>,
    /// ID da sessão.
    pub session_id: String,
    /// Último erro ocorrido.
    pub last_error: Option<Error>,
}

/// Estado de uma operação em andamento.
#[derive(Debug, Clone, PartialEq)]
pub enum OperationState {
    /// Nenhuma operação rodando.
    Idle,
    /// Operação rodando.
    Running {
        /// ID único da operação.
        op_id: OpId,
        /// Tipo da operação.
        kind: OpKind,
        /// Se cancelamento foi solicitado.
        cancel_requested: bool,
    },
    /// Operação bloqueada por outra incompatível.
    Blocked { reason: String },
}
