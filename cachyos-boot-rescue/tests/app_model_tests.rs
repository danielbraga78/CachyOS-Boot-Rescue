use cachyos_boot_rescue::app::{AppModel, AppState, OpId, OpKind, OperationState};

#[test]
fn test_app_model_default_state_is_welcome() {
    let model = AppModel::default();
    assert!(matches!(model.state, AppState::Welcome));
    assert!(model.shared.discovery_report.is_none());
    assert!(matches!(model.operation, OperationState::Idle));
}

#[test]
fn test_op_id_unique() {
    let id1 = OpId::new();
    let id2 = OpId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_op_kind_conflicts() {
    assert!(OpKind::Mount.conflicts_with(&OpKind::Mount));
    assert!(OpKind::Mount.conflicts_with(&OpKind::ChrootCommand));
    assert!(!OpKind::Discovery.conflicts_with(&OpKind::Discovery));
}
