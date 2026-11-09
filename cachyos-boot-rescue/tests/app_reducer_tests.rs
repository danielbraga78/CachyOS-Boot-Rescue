use cachyos_boot_rescue::app::{AppModel, AppState, AppEvent, MountStage};
use cachyos_boot_rescue::storage::{MountPlan, MountSession};
use cachyos_boot_rescue::discovery::types::DiscoveryReport;
use cachyos_boot_rescue::discovery::types::UefiState;
use cachyos_boot_rescue::error::Error;
use chrono::Utc;
use std::path::PathBuf;

fn dummy_discovery_report() -> DiscoveryReport {
    DiscoveryReport {
        timestamp: Utc::now(),
        session_id: "test-session".to_string(),
        uefi_state: UefiState::Uefi,
        devices: vec![],
        esp_candidates: vec![],
        linux_candidates: vec![],
        diagnostics: vec![],
        lsblk_version: None,
        blkid_version: None,
    }
}

fn dummy_mount_plan() -> MountPlan {
    MountPlan::new()
}

#[test]
fn test_welcome_to_discovery_on_start() {
    let model = AppModel::default();
    let (model, cmds) = cachyos_boot_rescue::app::reduce(model, AppEvent::Tick);
    assert!(matches!(model.state, AppState::Welcome));
    assert!(cmds.is_empty());
}

#[test]
fn test_discovery_completed_ok_transitions_to_install_select() {
    let mut model = AppModel::default();
    model.state = AppState::Discovery;
    let report = dummy_discovery_report();
    let (model, cmds) = cachyos_boot_rescue::app::reduce(model, AppEvent::DiscoveryCompleted(Ok(report)));
    assert!(matches!(model.state, AppState::InstallSelect));
    assert!(model.shared.discovery_report.is_some());
    assert!(cmds.is_empty());
}

#[test]
fn test_discovery_completed_err_transitions_to_error() {
    let mut model = AppModel::default();
    model.state = AppState::Discovery;
    let (model, cmds) = cachyos_boot_rescue::app::reduce(model, AppEvent::DiscoveryCompleted(Err(Error::command("lsblk", "failed", None, None))));
    assert!(matches!(model.state, AppState::Error { .. }));
    assert!(cmds.is_empty());
}

#[test]
fn test_install_select_to_mount_review_on_build_plan() {
    let mut model = AppModel::default();
    model.state = AppState::InstallSelect;
    model.selected_install = Some(0);
    model.shared.discovery_report = Some(dummy_discovery_report());
    let (model, cmds) = cachyos_boot_rescue::app::reduce(model, AppEvent::MountPlanCompleted(Ok(dummy_mount_plan())));
    assert!(matches!(model.state, AppState::MountReview));
    assert!(model.shared.mount_plan.is_some());
    assert!(cmds.is_empty());
}

#[test]
fn test_mount_review_to_mount_progress_on_execute_mount() {
    let mut model = AppModel::default();
    model.state = AppState::MountReview;
    model.shared.mount_plan = Some(dummy_mount_plan());
    let (model, _cmds) = cachyos_boot_rescue::app::reduce(model, AppEvent::MountProgress { stage: MountStage::Preparing, percent: 0, message: "Starting".into() });
    assert!(matches!(model.state, AppState::MountProgress { .. }));
}

#[test]
fn test_mount_completed_ok_transitions_to_chroot_active() {
    let mut model = AppModel::default();
    model.state = AppState::MountProgress { stage: MountStage::Finalizing };
    let session = MountSession::new(PathBuf::from("/mnt/root"));
    let (model, cmds) = cachyos_boot_rescue::app::reduce(model, AppEvent::MountCompleted(Ok(session)));
    assert!(matches!(model.state, AppState::ChrootActive));
    assert!(model.shared.mount_session.is_some());
    assert!(cmds.is_empty());
}