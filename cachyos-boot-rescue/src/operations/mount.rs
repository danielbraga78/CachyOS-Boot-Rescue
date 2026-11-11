use crate::app::events::AppError;
use crate::app::{AppEvent, MountStage};
use crate::error::Error;
use crate::runtime::EventSender;
use crate::storage::{MountPlan, MountSession, mount_session::execute_mount_plan};
use std::path::PathBuf;

pub async fn execute_mount(
    event_tx: EventSender,
    plan: MountPlan,
    root_mount: PathBuf,
) -> Result<MountSession, Error> {
    // Stage: Preparing
    let _ = event_tx
        .send(AppEvent::MountProgress {
            stage: MountStage::Preparing,
            percent: 0,
            message: "Preparando montagem".into(),
        })
        .await;

    // Monta root primeiro
    let _ = event_tx
        .send(AppEvent::MountProgress {
            stage: MountStage::MountingRoot,
            percent: 10,
            message: "Montando raiz".into(),
        })
        .await;

    // Cria session dentro do spawn_blocking para evitar problemas de move
    let result: Result<Result<MountSession, Error>, tokio::task::JoinError> =
        tokio::task::spawn_blocking(move || {
            let session = MountSession::new(root_mount);
            execute_mount_plan(&plan, &session)?;
            Ok::<MountSession, Error>(session)
        })
        .await;

    match result {
        Ok(Ok(session)) => {
            let _ = event_tx
                .send(AppEvent::MountProgress {
                    stage: MountStage::Completed,
                    percent: 100,
                    message: "Montagem concluída".into(),
                })
                .await;
            let _ = event_tx
                .send(AppEvent::MountCompleted(Ok(session.clone())))
                .await;
            Ok(session)
        }
        Ok(Err(e)) => {
            let _ = event_tx
                .send(AppEvent::MountCompleted(Err(AppError::from(e))))
                .await;
            Err(Error::internal("Falha na montagem".to_string()))
        }
        Err(e) => {
            let err = Error::internal(e.to_string());
            let _ = event_tx
                .send(AppEvent::MountCompleted(Err(AppError::from(err))))
                .await;
            Err(Error::internal("Erro interno na montagem".to_string()))
        }
    }
}
