//! Wrapper assíncrono para execução do diagnóstico.
//!
//! Integra com o sistema de operações e TaskManager da aplicação.

use crate::app::AppEvent;
use crate::app::events::AppError;
use crate::diagnostic::types::DiagnosticConfig;
use crate::discovery::types::DiscoveryReport;
use crate::error::Error;
use crate::runtime::EventSender;
use crate::storage::MountSession;

/// Executa diagnóstico como operação assíncrona.
pub async fn run_diagnostic_op(
    event_tx: EventSender,
    discovery: DiscoveryReport,
    selected_idx: usize,
    mount_session: Option<MountSession>,
    config: DiagnosticConfig,
) {
    let result = tokio::task::spawn_blocking(move || {
        crate::diagnostic::run_diagnostic(&discovery, selected_idx, mount_session.as_ref(), config)
    })
    .await;

    let event = match result {
        Ok(Ok(run)) => AppEvent::DiagnosticCompleted(Ok(run)),
        Ok(Err(e)) => AppEvent::DiagnosticCompleted(Err(AppError::from(e))),
        Err(e) => AppEvent::DiagnosticCompleted(Err(Error::internal(e.to_string()))),
    };

    let _ = event_tx.send(event).await;
}

/// Configuração padrão para diagnóstico.
pub fn default_diagnostic_config() -> DiagnosticConfig {
    DiagnosticConfig::default()
}

/// Configuração para diagnóstico rápido (apenas verificações essenciais).
pub fn quick_diagnostic_config() -> DiagnosticConfig {
    DiagnosticConfig {
        check_grub: true,
        check_limine: true,
        check_systemd_boot: true,
        check_efi_entries: true,
        check_secure_boot: false,
        check_kernels: true,
        check_fstab_mount: true,
        check_timeout_secs: 10,
        use_chroot: false,
        chroot_root: None,
    }
}

/// Configuração para diagnóstico completo (inclui chroot se disponível).
pub fn full_diagnostic_config(chroot_root: Option<std::path::PathBuf>) -> DiagnosticConfig {
    DiagnosticConfig {
        check_grub: true,
        check_limine: true,
        check_systemd_boot: true,
        check_efi_entries: true,
        check_secure_boot: true,
        check_kernels: true,
        check_fstab_mount: true,
        check_timeout_secs: 60,
        use_chroot: chroot_root.is_some(),
        chroot_root,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_diagnostic_config() {
        let config = default_diagnostic_config();
        assert!(config.check_grub);
        assert!(config.check_limine);
        assert!(config.check_systemd_boot);
        assert!(config.check_efi_entries);
        assert!(config.check_secure_boot);
        assert!(config.check_kernels);
        assert!(config.check_fstab_mount);
        assert_eq!(config.check_timeout_secs, 30);
        assert!(!config.use_chroot);
    }

    #[test]
    fn test_quick_diagnostic_config() {
        let config = quick_diagnostic_config();
        assert!(config.check_grub);
        assert!(!config.check_secure_boot);
        assert_eq!(config.check_timeout_secs, 10);
    }

    #[test]
    fn test_full_diagnostic_config() {
        let config = full_diagnostic_config(Some(std::path::PathBuf::from("/mnt")));
        assert!(config.use_chroot);
        assert_eq!(config.chroot_root, Some(std::path::PathBuf::from("/mnt")));
        assert_eq!(config.check_timeout_secs, 60);
    }
}
