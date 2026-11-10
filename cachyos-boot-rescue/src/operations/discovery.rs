use crate::discovery::{run_discovery, default_command_config};
use crate::discovery::types::CommandConfig;
use crate::app::AppEvent;
use crate::app::events::AppError;
use crate::error::Error;
use crate::runtime::EventSender;

pub async fn run_discovery_op(event_tx: EventSender, config: Option<CommandConfig>) {
    let config = config.unwrap_or_else(default_command_config);
    let result = tokio::task::spawn_blocking(move || run_discovery(Some(config))).await;
    let event = match result {
        Ok(Ok(report)) => AppEvent::DiscoveryCompleted(Ok(report)),
        Ok(Err(e)) => AppEvent::DiscoveryCompleted(Err(AppError::from(e))),
        Err(e) => AppEvent::DiscoveryCompleted(Err(Error::internal(e.to_string()))),
    };
    let _ = event_tx.send(event).await;
}