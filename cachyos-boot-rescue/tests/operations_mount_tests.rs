use cachyos_boot_rescue::operations::execute_mount;
use cachyos_boot_rescue::runtime::event_channel;
use cachyos_boot_rescue::app::AppEvent;
use tempfile::tempdir;

#[tokio::test]
async fn test_execute_mount_emits_progress_and_completed() {
    let (tx, mut rx) = event_channel(cachyos_boot_rescue::runtime::EVENT_CHANNEL_BUFFER);
    let plan = cachyos_boot_rescue::storage::MountPlan::new();
    let root = tempdir().unwrap().path().to_path_buf();
    execute_mount(tx, plan, root).await;
    // Verifica MountProgress events + MountCompleted
    let mut progress_count = 0;
    let mut completed = false;
    while let Ok(evt) = rx.try_recv() {
        match evt {
            AppEvent::MountProgress { .. } => progress_count += 1,
            AppEvent::MountCompleted(_) => completed = true,
            _ => {}
        }
    }
    assert!(progress_count > 0);
    assert!(completed);
}