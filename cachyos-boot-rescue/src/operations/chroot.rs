use crate::chroot::{ChrootManager, ChrootCommandConfig};
use crate::app::{AppEvent, AppCommand};
use crate::app::events::AppError;
use crate::error::Error;
use crate::runtime::{EventSender, CommandReceiver};
use std::path::PathBuf;
use tracing::{info, error};

pub async fn run_chroot_task(event_tx: EventSender, mut cmd_rx: CommandReceiver, root_mount: PathBuf) {
    info!("run_chroot_task started with root_mount: {:?}", root_mount);
    let mut manager = ChrootManager::new(root_mount);
    
    match manager.prepare() {
        Ok(_) => { 
            info!("Chroot prepared successfully");
            if let Err(e) = event_tx.send(AppEvent::ChrootPrepared(Ok(()))).await {
                error!("Failed to send ChrootPrepared: {:?}", e);
                return;
            }
        }
        Err(e) => { 
            info!("Chroot prepare failed: {:?}", e);
            let _ = event_tx.send(AppEvent::ChrootPrepared(Err(Error::internal(e.to_string())))).await; 
            return; 
        }
    }
    
    info!("Entering command loop");
    while let Some(cmd) = cmd_rx.recv().await {
        info!("Received command: {:?}", cmd);
        match cmd {
            AppCommand::ExecuteChrootCommand { command } => {
                let config = ChrootCommandConfig::default();
                let cmd_refs: Vec<&str> = command.iter().map(|s| s.as_str()).collect();
                let result = manager.execute(&cmd_refs, Some(config));
                let event = match result {
                    Ok(res) => AppEvent::ChrootCommandCompleted(Ok(format!("exit={}, stdout={}", res.exit_code, res.stdout))),
                    Err(e) => AppEvent::ChrootCommandCompleted(Err(AppError::from(e))),
                };
                if let Err(e) = event_tx.send(event).await {
                    error!("Failed to send ChrootCommandCompleted: {:?}", e);
                }
            }
            AppCommand::Quit => {
                info!("Received Quit command, breaking loop");
                break;
            }
            _ => {}
        }
    }
    
    info!("Cleaning up chroot");
    let cleanup_result = manager.cleanup();
    if let Err(e) = event_tx.send(AppEvent::ChrootCleanedUp(cleanup_result.map_err(|e| Error::internal(e.to_string())))).await {
        error!("Failed to send ChrootCleanedUp: {:?}", e);
    }
}