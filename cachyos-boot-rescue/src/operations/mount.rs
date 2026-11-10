use crate::storage::{MountSession, MountPlan, mount_session::execute_mount_plan};
use crate::app::{AppEvent, MountStage};
use crate::app::events::AppError;
use crate::error::Error;
use crate::runtime::EventSender;
use std::path::PathBuf;

pub async fn execute_mount(event_tx: EventSender, plan: MountPlan, root_mount: PathBuf) {
    // Stage: Preparing
    let _ = event_tx.send(AppEvent::MountProgress { stage: MountStage::Preparing, percent: 0, message: "Preparando montagem".into() }).await;
    
    // Monta root primeiro
    let _ = event_tx.send(AppEvent::MountProgress { stage: MountStage::MountingRoot, percent: 10, message: "Montando raiz".into() }).await;
    
    // Cria session dentro do spawn_blocking para evitar problemas de move
    let result: Result<Result<MountSession, Error>, tokio::task::JoinError> = tokio::task::spawn_blocking(move || {
        let session = MountSession::new(root_mount);
        execute_mount_plan(&plan, &session)?;
        Ok::<MountSession, Error>(session)
    }).await;
    
    let event = match result {
        Ok(Ok(session)) => {
            let _ = event_tx.send(AppEvent::MountProgress { stage: MountStage::Completed, percent: 100, message: "Montagem concluída".into() }).await;
            AppEvent::MountCompleted(Ok(session))
        }
        Ok(Err(e)) => AppEvent::MountCompleted(Err(AppError::from(e))),
        Err(e) => AppEvent::MountCompleted(Err(Error::internal(e.to_string()))),
    };
    let _ = event_tx.send(event).await;
}