//! Reducer puro para transições de estado.

use crate::app::{AppModel, AppEvent, AppCommand};
use crate::app::state::AppState;

/// Reducer puro: recebe modelo e evento, retorna novo modelo e comandos a despachar.
pub fn reduce(model: AppModel, event: AppEvent) -> (AppModel, Vec<AppCommand>) {
    let mut model = model;
    let commands = Vec::new();

    match event {
        AppEvent::DiscoveryCompleted(Ok(report)) => {
            model.shared.discovery_report = Some(report);
            model.state = AppState::InstallSelect;
        }
        AppEvent::DiscoveryCompleted(Err(e)) => {
            model.state = AppState::Error { message: e.to_string() };
        }
        AppEvent::MountPlanCompleted(Ok(plan)) => {
            model.shared.mount_plan = Some(plan);
            model.state = AppState::MountReview;
        }
        AppEvent::MountPlanCompleted(Err(e)) => {
            model.state = AppState::Error { message: e.to_string() };
        }
        AppEvent::MountProgress { stage, percent: _, message: _ } => {
            model.state = AppState::MountProgress { stage };
        }
        AppEvent::MountCompleted(Ok(session)) => {
            model.shared.mount_session = Some(session);
            model.state = AppState::ChrootActive;
        }
        AppEvent::MountCompleted(Err(e)) => {
            model.state = AppState::Error { message: e.to_string() };
        }
        AppEvent::ChrootCommandCompleted(_) => {
            // Mantém ChrootActive, apenas loga resultado
        }
        AppEvent::ChrootPrepared(Ok(_)) => {
            // Mantém ChrootActive
        }
        AppEvent::ChrootPrepared(Err(e)) => {
            model.state = AppState::Error { message: e.to_string() };
        }
        AppEvent::ChrootCleanedUp(_) => {
            // Limpeza feita, volta para MountReview
            model.state = AppState::MountReview;
        }
        AppEvent::ReportExported(_) => {
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