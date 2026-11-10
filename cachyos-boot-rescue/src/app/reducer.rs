//! Reducer puro para transições de estado.

use crate::app::state::{AppState, OperationState};
use crate::app::{AppCommand, AppEvent, AppModel, OpId, OpKind};

/// Reducer puro: recebe modelo e evento, retorna novo modelo e comandos a despachar.
pub fn reduce(model: AppModel, event: AppEvent) -> (AppModel, Vec<AppCommand>) {
    let mut model = model;
    let commands = Vec::new();

    match event {
        AppEvent::DiscoveryCompleted(Ok(report)) => {
            model.shared.discovery_report = Some(report);
            model.state = AppState::InstallSelect;
            model.operation = OperationState::Idle;
        }
        AppEvent::DiscoveryCompleted(Err(e)) => {
            model.state = AppState::Error {
                message: e.to_string(),
            };
            model.operation = OperationState::Idle;
        }
        AppEvent::MountPlanCompleted(Ok(plan)) => {
            model.shared.mount_plan = Some(plan);
            model.state = AppState::MountReview;
            model.operation = OperationState::Idle;
        }
        AppEvent::MountPlanCompleted(Err(e)) => {
            model.state = AppState::Error {
                message: e.to_string(),
            };
            model.operation = OperationState::Idle;
        }
        AppEvent::MountProgress {
            stage,
            percent: _,
            message: _,
        } => {
            model.state = AppState::MountProgress { stage };
            if !matches!(model.operation, OperationState::Running { .. }) {
                model.operation = OperationState::Running {
                    op_id: OpId::new(),
                    kind: OpKind::Mount,
                    cancel_requested: false,
                };
            }
        }
        AppEvent::MountCompleted(Ok(session)) => {
            model.shared.mount_session = Some(session);
            model.state = AppState::ChrootActive;
            model.operation = OperationState::Idle;
        }
        AppEvent::MountCompleted(Err(e)) => {
            model.state = AppState::Error {
                message: e.to_string(),
            };
            model.operation = OperationState::Idle;
        }
        AppEvent::ChrootCommandCompleted(_) => {
            model.operation = OperationState::Idle;
            // Mantém ChrootActive, apenas loga resultado
        }
        AppEvent::ChrootPrepared(Ok(_)) => {
            model.operation = OperationState::Idle;
            // Mantém ChrootActive
        }
        AppEvent::ChrootPrepared(Err(e)) => {
            model.state = AppState::Error {
                message: e.to_string(),
            };
            model.operation = OperationState::Idle;
        }
        AppEvent::ChrootCleanedUp(_) => {
            // Limpeza feita, volta para MountReview
            model.state = AppState::MountReview;
            model.operation = OperationState::Idle;
        }
        AppEvent::ReportExported(_) => {
            model.operation = OperationState::Idle;
            // Apenas loga sucesso
        }
        AppEvent::TerminalResize => {
            // UI trata, no-op no model
        }
        AppEvent::Tick => {
            // No-op
        }
    }

    (model, commands)
}
