use cachyos_boot_rescue::app::{AppCommand, AppEvent};
use cachyos_boot_rescue::runtime::{
    COMMAND_CHANNEL_BUFFER, EVENT_CHANNEL_BUFFER, command_channel, event_channel,
};

#[tokio::test]
async fn test_command_channel_send_recv() {
    let (tx, mut rx) = command_channel(COMMAND_CHANNEL_BUFFER);
    tx.send(AppCommand::RunDiscovery).await.unwrap();
    let cmd = rx.recv().await.unwrap();
    assert!(matches!(cmd, AppCommand::RunDiscovery));
}

#[tokio::test]
async fn test_event_channel_send_recv() {
    let (tx, mut rx) = event_channel(EVENT_CHANNEL_BUFFER);
    tx.send(AppEvent::Tick).await.unwrap();
    let evt = rx.recv().await.unwrap();
    assert!(matches!(evt, AppEvent::Tick));
}
