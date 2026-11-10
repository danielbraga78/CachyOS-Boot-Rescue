use cachyos_boot_rescue::app::events::AppError;
use cachyos_boot_rescue::app::{AppCommand, AppEvent};
use cachyos_boot_rescue::runtime::{
    COMMAND_CHANNEL_BUFFER, EVENT_CHANNEL_BUFFER, TaskManager, command_channel, event_channel,
};
use cachyos_boot_rescue::storage::MountPlan;

fn dummy_mount_plan() -> MountPlan {
    MountPlan::new()
}

#[tokio::test]
async fn test_task_manager_blocks_concurrent_mutable_ops() {
    let (_cmd_tx, cmd_rx) = command_channel(COMMAND_CHANNEL_BUFFER);
    let (evt_tx, mut evt_rx) = event_channel(EVENT_CHANNEL_BUFFER);
    let mut tm = TaskManager::new(cmd_rx, evt_tx.clone());

    // Spawn primeira montagem
    tm.handle_command(AppCommand::ExecuteMount {
        plan: dummy_mount_plan(),
    })
    .await;

    // Segunda montagem deve ser bloqueada - evento deve ser enviado sincronamente
    tm.handle_command(AppCommand::ExecuteMount {
        plan: dummy_mount_plan(),
    })
    .await;

    // Verifica se o evento OperationBlocked foi enviado
    let evt = evt_rx.recv().await.unwrap();
    assert!(matches!(
        evt,
        AppEvent::MountCompleted(Err(AppError::OperationBlocked { .. }))
    ));
}
