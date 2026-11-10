use cachyos_boot_rescue::operations::run_chroot_task;
use cachyos_boot_rescue::runtime::{event_channel, command_channel};
use cachyos_boot_rescue::app::{AppEvent, AppCommand};
use tempfile::tempdir;

#[tokio::test]
async fn test_chroot_task_structure() {
    // Test that the task can be created and handles commands without actual chroot
    // This test verifies the task structure works, not the actual chroot functionality
    let (evt_tx, mut evt_rx) = event_channel(cachyos_boot_rescue::runtime::EVENT_CHANNEL_BUFFER);
    let (cmd_tx, cmd_rx) = command_channel(cachyos_boot_rescue::runtime::COMMAND_CHANNEL_BUFFER);
    let temp_dir = tempdir().unwrap();
    let root = temp_dir.path().to_path_buf();
    
    // Note: Actual chroot preparation requires root privileges and will fail in tests
    // This test verifies the task can be spawned and the command channel works
    // The actual chroot functionality is tested in integration tests with proper privileges
    let handle = tokio::spawn(run_chroot_task(evt_tx, cmd_rx, root));
    
    // Send quit immediately to test task cleanup
    cmd_tx.send(AppCommand::Quit).await.unwrap();
    
    // Wait for cleanup event (or channel close)
    let evt = tokio::time::timeout(std::time::Duration::from_secs(5), evt_rx.recv()).await;
    // Task may fail to prepare (no root), but should handle quit gracefully
    match evt {
        Ok(Some(AppEvent::ChrootPrepared(Err(_)))) => {
            // Expected - prepare fails without root
        }
        Ok(Some(AppEvent::ChrootCleanedUp(_))) => {
            // Also acceptable
        }
        Ok(Some(other)) => {
            panic!("Unexpected event: {:?}", other);
        }
        Ok(None) => {
            // Channel closed
        }
        Err(_) => {
            panic!("Timeout waiting for task response");
        }
    }
    
    handle.await.unwrap();
}