//! Módulo principal de descoberta - orquestra lsblk, blkid, UEFI, ESP e Linux detection.

pub mod blkid;
pub mod esp;
pub mod linux;
pub mod lsblk;
pub mod types;
pub mod uefi;

use crate::config::LogLevel;
use crate::discovery::types::{
    BlockDevice, CommandConfig, DiagnosticLevel, DiscoveryDiagnostic, DiscoveryReport, UefiState,
};
use crate::error::Result;
use crate::logging::{LogEvent, SessionId, current_logging, log_event};
use chrono::Utc;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

// Re-exporta tipos adicionais para uso externo (testes)
pub use crate::discovery::types::{
    CommandOutput, DeviceId, DeviceType, FilesystemInfo, FilesystemType, PartitionTableType,
    PartitionType,
};

// Re-exporta funções públicas dos submódulos
pub use crate::discovery::blkid::{
    enrich_with_blkid, get_blkid_version, parse_blkid_json, run_blkid,
};
pub use crate::discovery::esp::{analyze_esp_candidate, identify_esp_candidates};
pub use crate::discovery::linux::{
    find_kernels_and_initramfs, identify_from_metadata, identify_linux_installations,
    identify_linux_installations_with_root, parse_os_release, read_fstab,
};
pub use crate::discovery::lsblk::{get_lsblk_version, parse_lsblk_json, run_lsblk};
pub use crate::discovery::uefi::{
    are_efi_variables_accessible, detect_uefi_state, list_efi_variables,
};

/// Configuração padrão para comandos de descoberta.
pub fn default_command_config() -> CommandConfig {
    CommandConfig {
        timeout_secs: 30,
        max_output_bytes: 1024 * 1024, // 1 MB
        env_vars: HashMap::new(),
    }
}

/// Executa descoberta completa de hardware e instalações.
pub fn run_discovery(config: Option<CommandConfig>) -> Result<DiscoveryReport> {
    let config = config.unwrap_or_else(default_command_config);
    let session_id = current_logging()
        .map(|l| l.session_id().to_string())
        .unwrap_or_else(|| SessionId::new().to_string());

    info!(session_id = %session_id, "Iniciando descoberta de hardware");

    let mut diagnostics = Vec::new();

    // 1. Detecta estado UEFI
    let uefi_state = match uefi::detect_uefi_state() {
        Ok(state) => {
            info!(uefi_state = ?state, "Estado UEFI detectado");
            state
        }
        Err(e) => {
            let msg = format!("Falha ao detectar estado UEFI: {e}");
            warn!(error = %e, "Falha ao detectar estado UEFI");
            diagnostics.push(DiscoveryDiagnostic {
                level: DiagnosticLevel::Warning,
                component: "uefi".to_string(),
                message: msg,
                details: None,
            });
            UefiState::Unknown
        }
    };

    // 2. Executa lsblk
    let lsblk_output = match lsblk::run_lsblk(&config) {
        Ok(output) => output,
        Err(e) => {
            let msg = format!("Falha ao executar lsblk: {e}");
            error!(error = %e, "Falha ao executar lsblk");
            diagnostics.push(DiscoveryDiagnostic {
                level: DiagnosticLevel::Error,
                component: "lsblk".to_string(),
                message: msg,
                details: Some(e.to_string()),
            });
            return Err(e);
        }
    };

    // 3. Parseia lsblk
    let mut devices = match lsblk::parse_lsblk_json(&lsblk_output) {
        Ok(devices) => {
            info!(
                device_count = devices.len(),
                "Dispositivos detectados via lsblk"
            );
            devices
        }
        Err(e) => {
            let msg = format!("Falha ao parsear lsblk: {e}");
            error!(error = %e, "Falha ao parsear lsblk");
            diagnostics.push(DiscoveryDiagnostic {
                level: DiagnosticLevel::Error,
                component: "lsblk".to_string(),
                message: msg,
                details: Some(e.to_string()),
            });
            return Err(e);
        }
    };

    // 4. Tenta enriquecer com blkid (não falha se não disponível)
    if blkid::is_blkid_available() {
        match blkid::run_blkid(&config) {
            Ok(blkid_output) => match blkid::parse_blkid_json(&blkid_output) {
                Ok(blkid_data) => {
                    info!(entries = blkid_data.len(), "Dados blkid obtidos");
                    blkid::enrich_with_blkid(&mut devices, &blkid_data);
                }
                Err(e) => {
                    warn!(error = %e, "Falha ao parsear blkid, continuando sem enriquecimento");
                    diagnostics.push(DiscoveryDiagnostic {
                        level: DiagnosticLevel::Warning,
                        component: "blkid".to_string(),
                        message: format!("Falha ao parsear blkid: {e}"),
                        details: Some(e.to_string()),
                    });
                }
            },
            Err(e) => {
                warn!(error = %e, "Falha ao executar blkid, continuando sem enriquecimento");
                diagnostics.push(DiscoveryDiagnostic {
                    level: DiagnosticLevel::Warning,
                    component: "blkid".to_string(),
                    message: format!("Falha ao executar blkid: {e}"),
                    details: Some(e.to_string()),
                });
            }
        }
    } else {
        debug!("blkid não disponível, pulando enriquecimento");
        diagnostics.push(DiscoveryDiagnostic {
            level: DiagnosticLevel::Info,
            component: "blkid".to_string(),
            message: "blkid não disponível no sistema".to_string(),
            details: None,
        });
    }

    // 5. Identifica candidatos a ESP
    let esp_candidates = esp::identify_esp_candidates(&devices);
    info!(
        esp_count = esp_candidates.len(),
        "Candidatos ESP identificados"
    );
    for esp in &esp_candidates {
        debug!(
            device = %esp.device.path.display(),
            confidence = esp.confidence,
            reasons = ?esp.reasons,
            "Candidato ESP"
        );
    }

    // 6. Identifica instalações Linux (apenas montadas)
    let linux_candidates_mounted = linux::identify_linux_installations(&devices, &esp_candidates);
    info!(
        linux_mounted_count = linux_candidates_mounted.len(),
        "Instalações Linux montadas identificadas"
    );

    // 7. Identifica instalações Linux via metadados (não montadas)
    let linux_candidates_metadata = linux::identify_from_metadata(&devices);
    info!(
        linux_metadata_count = linux_candidates_metadata.len(),
        "Instalações Linux via metadados identificadas"
    );

    // Combina e deduplica (prioriza montadas)
    let mut all_linux = linux_candidates_mounted;
    for meta in linux_candidates_metadata {
        // Verifica se já temos essa partição como montada
        let is_duplicate = all_linux
            .iter()
            .any(|m| m.root_device.path == meta.root_device.path);
        if !is_duplicate {
            all_linux.push(meta);
        }
    }

    // Ordena por confiança
    all_linux.sort_by_key(|a| std::cmp::Reverse(a.confidence));

    for linux in &all_linux {
        debug!(
            device = %linux.root_device.path.display(),
            distribution = ?linux.distribution,
            confidence = linux.confidence,
            reasons = ?linux.reasons,
            "Candidato Linux"
        );
    }

    // 8. Registra diagnóstico de descoberta
    log_discovery_diagnostics(&diagnostics, &session_id);

    let report = DiscoveryReport {
        timestamp: Utc::now(),
        session_id,
        uefi_state,
        devices,
        esp_candidates,
        linux_candidates: all_linux,
        diagnostics,
        lsblk_version: lsblk::get_lsblk_version().ok().flatten(),
        blkid_version: blkid::get_blkid_version().ok().flatten(),
    };

    info!(
        devices = report.devices.len(),
        esps = report.esp_candidates.len(),
        linux = report.linux_candidates.len(),
        diagnostics = report.diagnostics.len(),
        "Descoberta concluída"
    );

    Ok(report)
}

/// Registra diagnósticos no sistema de logging.
fn log_discovery_diagnostics(diagnostics: &[DiscoveryDiagnostic], session_id: &str) {
    for diag in diagnostics {
        let level = match diag.level {
            DiagnosticLevel::Error => LogLevel::Error,
            DiagnosticLevel::Warning => LogLevel::Warn,
            DiagnosticLevel::Info => LogLevel::Info,
        };

        let event = LogEvent::new(
            SessionId::from_string(session_id).unwrap_or_default(),
            "discovery",
            diag.component.clone(),
            level,
            diag.message.clone(),
        )
        .with_field("diagnostic_level", format!("{:?}", diag.level).into());

        log_event(event);
    }
}

/// Extrai todos os discos (dispositivos pai) do relatório.
pub fn get_disks(report: &DiscoveryReport) -> Vec<&BlockDevice> {
    report
        .devices
        .iter()
        .filter(|d| d.device_type == crate::discovery::types::DeviceType::Disk)
        .collect()
}

/// Extrai todas as partições do relatório.
pub fn get_partitions(report: &DiscoveryReport) -> Vec<&BlockDevice> {
    fn collect_partitions<'b>(device: &'b BlockDevice, result: &mut Vec<&'b BlockDevice>) {
        if device.device_type == crate::discovery::types::DeviceType::Partition {
            result.push(device);
        }
        for child in &device.children {
            collect_partitions(child, result);
        }
    }

    let mut partitions = Vec::new();
    for device in &report.devices {
        collect_partitions(device, &mut partitions);
    }
    partitions
}

/// Encontra dispositivo por caminho.
pub fn find_device_by_path<'a>(
    report: &'a DiscoveryReport,
    path: &std::path::Path,
) -> Option<&'a BlockDevice> {
    fn search<'b>(device: &'b BlockDevice, target: &std::path::Path) -> Option<&'b BlockDevice> {
        if device.path == target {
            return Some(device);
        }
        for child in &device.children {
            if let Some(found) = search(child, target) {
                return Some(found);
            }
        }
        None
    }

    for device in &report.devices {
        if let Some(found) = search(device, path) {
            return Some(found);
        }
    }
    None
}

/// Obtém resumo textual do relatório para exibição.
pub fn format_discovery_summary(report: &DiscoveryReport) -> String {
    let mut lines = Vec::new();

    lines.push("=== Relatório de Descoberta ===".to_string());
    lines.push(format!("Timestamp: {}", report.timestamp.to_rfc3339()));
    lines.push(format!("Sessão: {}", report.session_id));
    lines.push(format!("Firmware: {:?}", report.uefi_state));
    lines.push(format!("Dispositivos: {}", report.devices.len()));
    lines.push(format!("Candidatos ESP: {}", report.esp_candidates.len()));
    lines.push(format!(
        "Instalações Linux: {}",
        report.linux_candidates.len()
    ));
    lines.push(format!("Diagnósticos: {}", report.diagnostics.len()));

    if let Some(v) = &report.lsblk_version {
        lines.push(format!("lsblk: {}", v));
    }
    if let Some(v) = &report.blkid_version {
        lines.push(format!("blkid: {}", v));
    }

    lines.push(String::new());
    lines.push("--- Discos ---".to_string());
    for disk in get_disks(report) {
        let model = disk.model.as_deref().unwrap_or("Desconhecido");
        let size_gb = disk.size.map(|s| s as f64 / (1024.0 * 1024.0 * 1024.0));
        let size_str = size_gb.map_or("desconhecido".to_string(), |s| format!("{:.1}", s));
        lines.push(format!(
            "  {} ({} GB) - {} - {}",
            disk.path.display(),
            size_str,
            model,
            disk.partition_table
                .map_or("sem tabela".to_string(), |t| format!("{:?}", t))
        ));
    }

    lines.push(String::new());
    lines.push("--- Candidatos ESP ---".to_string());
    for esp in &report.esp_candidates {
        lines.push(format!(
            "  {} - confiança: {}% - {}",
            esp.device.path.display(),
            esp.confidence,
            esp.reasons.join(", ")
        ));
    }

    lines.push(String::new());
    lines.push("--- Instalações Linux ---".to_string());
    for linux in &report.linux_candidates {
        let distro = linux.distribution.as_deref().unwrap_or("Desconhecida");
        lines.push(format!(
            "  {} - {} - confiança: {}% - {}",
            linux.root_device.path.display(),
            distro,
            linux.confidence,
            linux.reasons.join(", ")
        ));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::types::{
        BlockDevice, DeviceId, DeviceType, FilesystemType, PartitionType,
    };
    use std::path::PathBuf;

    fn create_test_disk() -> BlockDevice {
        BlockDevice {
            path: PathBuf::from("/dev/sda"),
            name: "sda".to_string(),
            device_id: Some(DeviceId::from_major_minor(8, 0)),
            device_type: DeviceType::Disk,
            size: Some(500 * 1024 * 1024 * 1024),
            sector_size: Some(512),
            logical_sector_size: Some(512),
            model: Some("Test Disk".to_string()),
            serial: Some("12345".to_string()),
            vendor: Some("Test".to_string()),
            rotation_rate: Some(7200),
            removable: Some(false),
            read_only: Some(false),
            partition_table: Some(crate::discovery::types::PartitionTableType::Gpt),
            uuid: None,
            partuuid: None,
            label: None,
            partition_type: None,
            filesystem: None,
            mountpoint: None,
            mountpoints: vec![],
            mount_options: vec![],
            parent: None,
            sysfs_path: None,
            children: vec![
                create_test_partition(
                    "/dev/sda1",
                    PartitionType::EfiSystem,
                    FilesystemType::Vfat,
                    512 * 1024 * 1024,
                    Some("EFI"),
                    Some("/boot/efi"),
                ),
                create_test_partition(
                    "/dev/sda2",
                    PartitionType::LinuxFilesystem,
                    FilesystemType::Ext4,
                    50 * 1024 * 1024 * 1024,
                    Some("root"),
                    Some("/"),
                ),
            ],
        }
    }

    fn create_test_partition(
        path: &str,
        part_type: PartitionType,
        fs: FilesystemType,
        size: u64,
        label: Option<&str>,
        mountpoint: Option<&str>,
    ) -> BlockDevice {
        BlockDevice {
            path: PathBuf::from(path),
            name: path.trim_start_matches("/dev/").to_string(),
            device_id: Some(DeviceId::from_major_minor(0, 0)),
            device_type: DeviceType::Partition,
            size: Some(size),
            sector_size: Some(512),
            logical_sector_size: Some(512),
            model: None,
            serial: None,
            vendor: None,
            rotation_rate: None,
            removable: Some(false),
            read_only: Some(false),
            partition_table: None,
            uuid: Some("test-uuid".to_string()),
            partuuid: Some("test-partuuid".to_string()),
            label: label.map(String::from),
            partition_type: Some(part_type),
            filesystem: Some(fs),
            mountpoint: mountpoint.map(PathBuf::from),
            mountpoints: vec![],
            mount_options: vec![],
            parent: Some("sda".to_string()),
            sysfs_path: None,
            children: vec![],
        }
    }

    #[test]
    fn test_get_disks() {
        let disk = create_test_disk();
        let report = DiscoveryReport {
            timestamp: Utc::now(),
            session_id: "test".to_string(),
            uefi_state: UefiState::Uefi,
            devices: vec![disk],
            esp_candidates: vec![],
            linux_candidates: vec![],
            diagnostics: vec![],
            lsblk_version: None,
            blkid_version: None,
        };

        let disks = get_disks(&report);
        assert_eq!(disks.len(), 1);
    }

    #[test]
    fn test_get_partitions() {
        let disk = create_test_disk();
        let report = DiscoveryReport {
            timestamp: Utc::now(),
            session_id: "test".to_string(),
            uefi_state: UefiState::Uefi,
            devices: vec![disk],
            esp_candidates: vec![],
            linux_candidates: vec![],
            diagnostics: vec![],
            lsblk_version: None,
            blkid_version: None,
        };

        let partitions = get_partitions(&report);
        assert_eq!(partitions.len(), 2);
    }

    #[test]
    fn test_find_device_by_path() {
        let disk = create_test_disk();
        let report = DiscoveryReport {
            timestamp: Utc::now(),
            session_id: "test".to_string(),
            uefi_state: UefiState::Uefi,
            devices: vec![disk],
            esp_candidates: vec![],
            linux_candidates: vec![],
            diagnostics: vec![],
            lsblk_version: None,
            blkid_version: None,
        };

        let found = find_device_by_path(&report, PathBuf::from("/dev/sda1").as_path());
        assert!(found.is_some());
        assert_eq!(found.unwrap().path, PathBuf::from("/dev/sda1"));

        let not_found = find_device_by_path(&report, PathBuf::from("/dev/sdb1").as_path());
        assert!(not_found.is_none());
    }

    #[test]
    fn test_format_discovery_summary() {
        let disk = create_test_disk();
        let report = DiscoveryReport {
            timestamp: Utc::now(),
            session_id: "test-session".to_string(),
            uefi_state: UefiState::Uefi,
            devices: vec![disk],
            esp_candidates: vec![],
            linux_candidates: vec![],
            diagnostics: vec![],
            lsblk_version: Some("2.38".to_string()),
            blkid_version: None,
        };

        let summary = format_discovery_summary(&report);
        assert!(summary.contains("Relatório de Descoberta"));
        assert!(summary.contains("test-session"));
        assert!(summary.contains("Uefi")); // Debug format uses Uefi
        assert!(summary.contains("lsblk: 2.38"));
    }
}
