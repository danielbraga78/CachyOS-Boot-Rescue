use cachyos_boot_rescue::app::AppEvent;
use cachyos_boot_rescue::operations::run_discovery_op;
use cachyos_boot_rescue::runtime::event_channel;

#[tokio::test]
async fn test_run_discovery_op_emits_completed_event() {
    let (tx, mut rx) = event_channel(cachyos_boot_rescue::runtime::EVENT_CHANNEL_BUFFER);
    let config = cachyos_boot_rescue::discovery::default_command_config();
    run_discovery_op(tx, Some(config)).await;
    let evt = rx.recv().await.unwrap();
    assert!(matches!(evt, AppEvent::DiscoveryCompleted(_)));
}
