//! Eventos da aplicação (resultados/notificações).

use crate::app::model::MountStage;
use crate::diagnostic::types::DiagnosticRun;
use crate::discovery::types::DiscoveryReport;
use crate::error::Error;
use crate::storage::MountPlan;
use crate::storage::MountSession;
use std::path::PathBuf;

/// Alias para o tipo de erro da aplicação.
pub type AppError = Error;

/// Eventos que o TaskManager emite de volta para o Reducer.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum AppEvent {
    /// Discovery concluído.
    DiscoveryCompleted(Result<DiscoveryReport, AppError>),
    /// Plano de montagem construído.
    MountPlanCompleted(Result<MountPlan, AppError>),
    /// Progresso de montagem.
    MountProgress {
        stage: MountStage,
        percent: u8,
        message: String,
    },
    /// Montagem concluída.
    MountCompleted(Result<MountSession, AppError>),
    /// Comando chroot concluído.
    ChrootCommandCompleted(Result<String, AppError>),
    /// Chroot preparado.
    ChrootPrepared(Result<(), AppError>),
    /// Chroot limpo.
    ChrootCleanedUp(Result<(), AppError>),
    /// Relatório exportado.
    ReportExported(Result<PathBuf, AppError>),
    /// Diagnóstico concluído.
    DiagnosticCompleted(Result<DiagnosticRun, AppError>),
    /// Terminal redimensionado.
    TerminalResize,
    /// Tick periódico.
    Tick,
}
