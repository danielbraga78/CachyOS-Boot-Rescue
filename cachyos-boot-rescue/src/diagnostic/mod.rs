//! Motor de diagnóstico somente leitura de bootloaders.
//!
//! Orquestra a detecção de bootloaders (GRUB, Limine, systemd-boot),
//! verificação de layout EFI, kernels, initramfs, fstab e entradas UEFI NVRAM.

pub mod efi;
pub mod fstab_check;
pub mod grub;
pub mod kernel;
pub mod limine;
pub mod runner;
pub mod systemd_boot;
pub mod types;

use crate::diagnostic::types::*;
use crate::discovery::types::{
    DiscoveryReport, EspCandidate, LinuxInstallationCandidate, UefiState,
};
use crate::error::Error;
use crate::storage::MountSession;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

/// Executa o diagnóstico completo para uma instalação selecionada.
///
/// # Argumentos
/// - `discovery`: Relatório de descoberta da Fase 3.
/// - `selected_idx`: Índice da instalação candidata selecionada.
/// - `mount_session`: Sessão de montagem ativa (opcional, para verificações no chroot).
/// - `config`: Configuração do diagnóstico.
///
/// # Retorna
/// `DiagnosticRun` contendo o relatório e avisos não fatais.
pub fn run_diagnostic(
    discovery: &DiscoveryReport,
    selected_idx: usize,
    mount_session: Option<&MountSession>,
    config: DiagnosticConfig,
) -> Result<DiagnosticRun, Error> {
    let start = Instant::now();
    let session_id = uuid::Uuid::new_v4().to_string();

    // Obtém a instalação selecionada
    let candidate = discovery
        .linux_candidates
        .get(selected_idx)
        .ok_or_else(|| Error::validation("Índice de instalação inválido", None))?;

    let mut findings = Vec::new();
    let mut warnings = Vec::new();

    // Determina o root do chroot se disponível
    let chroot_root = if config.use_chroot {
        mount_session
            .map(|s| s.root_mount().to_path_buf())
            .or(config.chroot_root.clone())
    } else {
        None
    };

    // 1. Detecta ambiente de boot
    let environment = detect_boot_environment(discovery, candidate, &chroot_root, &config)?;

    // 2. Verificações de layout EFI/ESP
    if config.check_fstab_mount || config.check_efi_entries {
        findings.extend(check_efi_layout(
            &environment,
            &discovery.esp_candidates,
            &chroot_root,
        )?);
    }

    // 3. Verificações GRUB
    if config.check_grub {
        match grub::check_grub(&environment, &candidate, &chroot_root, &config) {
            Ok(mut f) => findings.append(&mut f),
            Err(e) => warnings.push(format!("Verificação GRUB falhou: {e}")),
        }
    }

    // 4. Verificações Limine
    if config.check_limine {
        match limine::check_limine(&environment, &candidate, &chroot_root, &config) {
            Ok(mut f) => findings.append(&mut f),
            Err(e) => warnings.push(format!("Verificação Limine falhou: {e}")),
        }
    }

    // 5. Verificações systemd-boot
    if config.check_systemd_boot {
        match systemd_boot::check_systemd_boot(&environment, &candidate, &chroot_root, &config) {
            Ok(mut f) => findings.append(&mut f),
            Err(e) => warnings.push(format!("Verificação systemd-boot falhou: {e}")),
        }
    }

    // 6. Verificações de kernels e initramfs
    if config.check_kernels {
        findings.extend(kernel::check_kernels_initramfs(
            &environment,
            &candidate,
            &chroot_root,
        )?);
    }

    // 7. Verificações fstab/montagem
    if config.check_fstab_mount {
        findings.extend(fstab_check::check_fstab_mount(
            &environment,
            &candidate,
            &chroot_root,
        )?);
    }

    // 8. Verificações EFI NVRAM
    if config.check_efi_entries && environment.firmware_mode == FirmwareMode::Uefi {
        findings.extend(efi::check_efi_entries(&environment, &chroot_root, &config)?);
    }

    // 8.1. Verificação Secure Boot
    if config.check_secure_boot && environment.firmware_mode == FirmwareMode::Uefi {
        findings.extend(efi::check_secure_boot(&chroot_root)?);
    }

    // Determina bootloader principal se não detectado
    let primary_bootloader = if environment.primary_bootloader.is_none() {
        detect_primary_bootloader(&findings)
    } else {
        environment.primary_bootloader
    };

    let mut final_environment = environment;
    final_environment.primary_bootloader = primary_bootloader;

    let duration_ms = start.elapsed().as_millis() as u64;

    let report = DiagnosticReport {
        timestamp: chrono::Utc::now(),
        session_id,
        environment: final_environment,
        findings: findings.clone(),
        summary: DiagnosticSummary::from_findings(&findings),
        duration_ms,
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let mut run = DiagnosticRun::new(report);
    for w in warnings {
        run.add_warning(w);
    }

    Ok(run)
}

/// Verifica layout EFI/ESP.
fn check_efi_layout(
    environment: &BootEnvironment,
    esp_candidates: &[EspCandidate],
    chroot_root: &Option<PathBuf>,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // Verifica se há ESP primária montada
    if environment.primary_esp.is_none() {
        findings.push(Finding {
            id: DiagnosticId::EfiNotMounted,
            severity: Severity::Error,
            title: "Nenhuma ESP primária identificada".to_string(),
            description: "Não foi possível identificar uma ESP primária para a instalação. O boot UEFI pode falhar.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::DeviceMissing,
                description: "ESP primária não encontrada".to_string(),
                path: None,
                expected: Some("ESP com confidence > 0".to_string()),
                observed: Some("Nenhuma".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![Recommendation {
                id: "identify_esp".to_string(),
                description: "Identificar e montar a ESP correta.".to_string(),
                action: "Verificar partições com tipo EFI System e filesystem FAT32".to_string(),
                is_mutating: true,
                risk: RiskLevel::High,
                prerequisites: vec!["Partições descobertas".to_string()],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    // Verifica múltiplas ESPs ambíguas
    let high_confidence_esps: Vec<_> = esp_candidates
        .iter()
        .filter(|e| e.confidence >= 70)
        .collect();
    if high_confidence_esps.len() > 1 {
        findings.push(Finding {
            id: DiagnosticId::MultipleEspsAmbiguous,
            severity: Severity::Warning,
            title: format!("Múltiplas ESPs com alta confiança ({})", high_confidence_esps.len()),
            description: "Foram detectadas múltiplas partições ESP com alta confiança. Isso pode causar ambiguidade no boot UEFI.".to_string(),
            evidence: high_confidence_esps.iter().map(|e| Evidence {
                kind: EvidenceKind::DeviceMismatch,
                description: format!("ESP candidata: {} (confidence: {})", e.device.path.display(), e.confidence),
                path: Some(e.device.path.clone()),
                expected: Some("Uma única ESP primária".to_string()),
                observed: Some("Múltiplas ESPs".to_string()),
                raw_data: HashMap::from([
                    ("confidence".to_string(), e.confidence.to_string()),
                    ("fstype".to_string(), format!("{:?}", e.filesystem.fstype)),
                ]),
            }).collect(),
            recommendations: vec![Recommendation {
                id: "select_primary_esp".to_string(),
                description: "Selecionar qual ESP usar como primária baseada no fstab e mountpoint.".to_string(),
                action: "Verificar /etc/fstab e mountpoint atual para determinar ESP correta".to_string(),
                is_mutating: false,
                risk: RiskLevel::Low,
                prerequisites: vec!["fstab acessível".to_string()],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    // Verifica ESPs não FAT32
    for esp in esp_candidates {
        if !esp.filesystem.fstype.is_fat() {
            findings.push(Finding {
                id: DiagnosticId::EspNotFat32,
                severity: Severity::Error,
                title: format!("ESP candidata {} não é FAT32", esp.device.path.display()),
                description: format!("A partição {} é marcada como ESP mas tem filesystem {:?}. A spec UEFI exige FAT32.", esp.device.path.display(), esp.filesystem.fstype),
                evidence: vec![Evidence {
                    kind: EvidenceKind::ConfigValueMismatch,
                    description: "ESP com filesystem incorreto".to_string(),
                    path: Some(esp.device.path.clone()),
                    expected: Some("FAT32/VFAT".to_string()),
                    observed: Some(format!("{:?}", esp.filesystem.fstype)),
                    raw_data: HashMap::new(),
                }],
                recommendations: vec![Recommendation {
                    id: "reformat_esp".to_string(),
                    description: "Reformatar a partição como FAT32 (CUIDADO: apaga dados).".to_string(),
                    action: "mkfs.fat -F 32 <device>".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::Critical,
                    prerequisites: vec!["Backup dos dados da ESP".to_string(), "Confirmar que é a ESP correta".to_string()],
                }],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
        }
    }

    Ok(findings)
}

/// Detecta o ambiente de boot com base na descoberta e montagem.
fn detect_boot_environment(
    discovery: &DiscoveryReport,
    candidate: &LinuxInstallationCandidate,
    chroot_root: &Option<PathBuf>,
    _config: &DiagnosticConfig,
) -> Result<BootEnvironment, Error> {
    use crate::discovery::uefi::detect_uefi_state;

    let firmware_mode = match detect_uefi_state() {
        Ok(UefiState::Uefi) => FirmwareMode::Uefi,
        Ok(UefiState::BiosLegacy) => FirmwareMode::BiosLegacy,
        _ => FirmwareMode::Unknown,
    };

    // Identifica ESPs candidatas associadas a esta instalação
    let mut primary_esp = None;
    let mut other_esps = Vec::new();

    // Primeiro, ESPs das partições de boot do candidato
    for boot_esp in &candidate.boot_partitions {
        let esp_info = EspInfo {
            device_path: boot_esp.device.path.clone(),
            uuid: boot_esp.filesystem.uuid.clone(),
            partuuid: boot_esp.device.partuuid.clone(),
            label: boot_esp.device.label.clone(),
            mountpoint: boot_esp.mountpoint.clone(),
            size: boot_esp.size,
            free_space: boot_esp.free_space,
            confidence: boot_esp.confidence,
        };

        if primary_esp.is_none() && (boot_esp.is_boot_efi_mount || boot_esp.is_boot_mount) {
            primary_esp = Some(esp_info);
        } else {
            other_esps.push(esp_info);
        }
    }

    // Se não encontrou primária, usa a primeira ESP candidata global com maior confiança
    if primary_esp.is_none() {
        let mut sorted_esps = discovery.esp_candidates.clone();
        sorted_esps.sort_by(|a, b| b.confidence.cmp(&a.confidence));

        if let Some(esp) = sorted_esps.first() {
            primary_esp = Some(EspInfo {
                device_path: esp.device.path.clone(),
                uuid: esp.filesystem.uuid.clone(),
                partuuid: esp.device.partuuid.clone(),
                label: esp.device.label.clone(),
                mountpoint: esp.mountpoint.clone(),
                size: esp.size,
                free_space: esp.free_space,
                confidence: esp.confidence,
            });

            for esp in sorted_esps.iter().skip(1) {
                other_esps.push(EspInfo {
                    device_path: esp.device.path.clone(),
                    uuid: esp.filesystem.uuid.clone(),
                    partuuid: esp.device.partuuid.clone(),
                    label: esp.device.label.clone(),
                    mountpoint: esp.mountpoint.clone(),
                    size: esp.size,
                    free_space: esp.free_space,
                    confidence: esp.confidence,
                });
            }
        }
    }

    // Detecta layout de /boot
    let boot_layout = detect_boot_layout(candidate, &primary_esp, &discovery.uefi_state);

    // Coleta kernels e initramfs
    let (kernels, initramfs) = kernel::collect_kernels_initramfs(candidate, chroot_root)?;

    // Coleta entradas EFI (somente leitura)
    let (efi_boot_entries, efi_boot_order) = if firmware_mode == FirmwareMode::Uefi {
        efi::collect_efi_entries()?
    } else {
        (Vec::new(), None)
    };

    // Status Secure Boot
    let secure_boot = if firmware_mode == FirmwareMode::Uefi {
        efi::detect_secure_boot()?
    } else {
        SecureBootState::Unsupported
    };

    Ok(BootEnvironment {
        firmware_mode,
        primary_bootloader: None, // Será preenchido depois
        additional_bootloaders: Vec::new(),
        primary_esp,
        other_esps,
        boot_layout,
        kernels,
        initramfs,
        efi_boot_entries,
        efi_boot_order,
        secure_boot,
        distribution: candidate.distribution.clone(),
        distribution_version: candidate.version.clone(),
    })
}

/// Detecta o layout de /boot.
fn detect_boot_layout(
    candidate: &LinuxInstallationCandidate,
    primary_esp: &Option<EspInfo>,
    uefi_state: &crate::discovery::types::UefiState,
) -> BootLayout {
    let has_separate_boot = candidate.additional_partitions.iter().any(|p| {
        p.mountpoint
            .as_ref()
            .is_some_and(|m| m == std::path::Path::new("/boot"))
    });

    let esp_on_boot = primary_esp.as_ref().is_some_and(|e| {
        e.mountpoint
            .as_ref()
            .is_some_and(|m| m == std::path::Path::new("/boot"))
    });

    let esp_on_boot_efi = primary_esp.as_ref().is_some_and(|e| {
        e.mountpoint
            .as_ref()
            .is_some_and(|m| m == std::path::Path::new("/boot/efi"))
    });

    let esp_on_efi = primary_esp.as_ref().is_some_and(|e| {
        e.mountpoint
            .as_ref()
            .is_some_and(|m| m == std::path::Path::new("/efi"))
    });

    match (
        uefi_state,
        has_separate_boot,
        esp_on_boot,
        esp_on_boot_efi,
        esp_on_efi,
    ) {
        (crate::discovery::types::UefiState::Uefi, true, false, true, false) => {
            BootLayout::SeparateBootEfi
        }
        (crate::discovery::types::UefiState::Uefi, false, true, false, false) => {
            BootLayout::UnifiedBootOnEsp
        }
        (crate::discovery::types::UefiState::Uefi, true, false, false, true) => {
            BootLayout::EfiAtEfiBootSeparate
        }
        (crate::discovery::types::UefiState::BiosLegacy, true, false, false, false) => {
            BootLayout::BiosLegacySeparateBoot
        }
        (crate::discovery::types::UefiState::BiosLegacy, false, false, false, false) => {
            BootLayout::BiosLegacyBootOnRoot
        }
        _ => BootLayout::Unknown,
    }
}

/// Detecta bootloader principal baseado nos achados.
fn detect_primary_bootloader(findings: &[Finding]) -> Option<BootloaderType> {
    let mut grub_evidence = 0;
    let mut limine_evidence = 0;
    let mut systemd_boot_evidence = 0;

    for finding in findings {
        match finding.id {
            DiagnosticId::GrubEfiBinaryMissing
            | DiagnosticId::GrubConfigMissing
            | DiagnosticId::GrubConfigInvalid
            | DiagnosticId::GrubModulesMissing
            | DiagnosticId::GrubDefaultEntryMissing
            | DiagnosticId::GrubUuidsMismatch
            | DiagnosticId::GrubKernelNotFound
            | DiagnosticId::GrubInitramfsNotFound
            | DiagnosticId::GrubBiosOnUefi
            | DiagnosticId::GrubUefiOnBios => {
                grub_evidence += 1;
            }
            DiagnosticId::LimineEfiBinaryMissing
            | DiagnosticId::LimineConfigMissing
            | DiagnosticId::LimineConfigInvalid
            | DiagnosticId::LimineKernelPathInvalid
            | DiagnosticId::LimineInitramfsPathInvalid
            | DiagnosticId::LimineEspLayoutMismatch => {
                limine_evidence += 1;
            }
            _ => {}
        }
    }

    // Verifica também por systemd-boot via achados de EFI
    for finding in findings {
        if finding.id == DiagnosticId::EfiBootEntryInvalid
            || finding.id == DiagnosticId::EfiBootEntryPointsToMissingFile
        {
            for evidence in &finding.evidence {
                if evidence
                    .raw_data
                    .get("loader_path")
                    .is_some_and(|p| p.contains("systemd-boot") || p.contains("systemd"))
                {
                    systemd_boot_evidence += 1;
                }
            }
        }
    }

    // Retorna o com mais evidências
    if grub_evidence > limine_evidence && grub_evidence > systemd_boot_evidence {
        Some(BootloaderType::Grub)
    } else if limine_evidence > grub_evidence && limine_evidence > systemd_boot_evidence {
        Some(BootloaderType::Limine)
    } else if systemd_boot_evidence > grub_evidence && systemd_boot_evidence > limine_evidence {
        Some(BootloaderType::SystemdBoot)
    } else if grub_evidence > 0 || limine_evidence > 0 || systemd_boot_evidence > 0 {
        Some(BootloaderType::Unknown)
    } else {
        None
    }
}

/// Cria um achado padrão para bootloader não detectado.
pub fn create_bootloader_not_detected_finding() -> Finding {
    Finding {
        id: DiagnosticId::BootloaderNotDetected,
        severity: Severity::Error,
        title: "Nenhum bootloader detectado".to_string(),
        description: "Não foi possível identificar GRUB, Limine, systemd-boot ou outro bootloader conhecido na instalação selecionada.".to_string(),
        evidence: vec![Evidence {
            kind: EvidenceKind::Other,
            description: "Nenhuma evidência de bootloader encontrado em /boot, /boot/efi, /efi ou ESPs".to_string(),
            path: None,
            expected: Some("Bootloader files".to_string()),
            observed: Some("None found".to_string()),
            raw_data: HashMap::new(),
        }],
        recommendations: vec![Recommendation {
            id: "install_bootloader".to_string(),
            description: "A instalação não possui bootloader configurado. É necessário instalar um bootloader compatível com o firmware (GRUB para UEFI/BIOS, Limine para UEFI, systemd-boot para UEFI).".to_string(),
            action: "Instalar bootloader (GRUB, Limine ou systemd-boot) via chroot".to_string(),
            is_mutating: true,
            risk: RiskLevel::High,
            prerequisites: vec!["Backup da ESP".to_string(), "Chroot preparado".to_string()],
        }],
        verified_in_chroot: false,
        context: HashMap::new(),
    }
}

/// Cria achado para múltiplos bootloaders detectados.
pub fn create_multiple_bootloaders_finding(bootloaders: Vec<BootloaderType>) -> Finding {
    let names: Vec<String> = bootloaders.iter().map(|b| format!("{:?}", b)).collect();
    Finding {
        id: DiagnosticId::MultipleBootloadersDetected,
        severity: Severity::Warning,
        title: "Múltiplos bootloaders detectados".to_string(),
        description: format!("Foram detectados múltiplos bootloaders: {}. Isso pode causar ambiguidades no boot.", names.join(", ")),
        evidence: vec![Evidence {
            kind: EvidenceKind::Other,
            description: format!("Bootloaders detectados: {}", names.join(", ")),
            path: None,
            expected: Some("Single bootloader".to_string()),
            observed: Some(names.join(", ")),
            raw_data: HashMap::new(),
        }],
        recommendations: vec![
            Recommendation {
                id: "choose_primary_bootloader".to_string(),
                description: "Mantenha apenas um bootloader como primário e remova/desative os outros para evitar conflitos.".to_string(),
                action: "Configurar um bootloader como primário e remover entradas dos outros".to_string(),
                is_mutating: true,
                risk: RiskLevel::Medium,
                prerequisites: vec!["Backup da ESP".to_string()],
            },
            Recommendation {
                id: "keep_both_documented".to_string(),
                description: "Se intencional (ex: dual boot com fallback), documente a configuração e verifique se as entradas EFI NVRAM estão corretas.".to_string(),
                action: "Verificar entradas EFI e documentar configuração".to_string(),
                is_mutating: false,
                risk: RiskLevel::Low,
                prerequisites: vec![],
            },
        ],
        verified_in_chroot: false,
        context: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bootloader_not_detected_finding() {
        let finding = create_bootloader_not_detected_finding();
        assert_eq!(finding.id, DiagnosticId::BootloaderNotDetected);
        assert_eq!(finding.severity, Severity::Error);
        assert!(!finding.recommendations.is_empty());
        assert!(finding.recommendations[0].is_mutating);
    }

    #[test]
    fn test_create_multiple_bootloaders_finding() {
        let finding =
            create_multiple_bootloaders_finding(vec![BootloaderType::Grub, BootloaderType::Limine]);
        assert_eq!(finding.id, DiagnosticId::MultipleBootloadersDetected);
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.recommendations.len(), 2);
    }

    #[test]
    fn test_boot_layout_detection() {
        use crate::discovery::types::{
            BlockDevice, DeviceId, DeviceType, FilesystemInfo, FilesystemType, PartitionType,
            UefiState,
        };
        use std::path::PathBuf;

        let mut candidate = LinuxInstallationCandidate {
            root_device: BlockDevice {
                path: PathBuf::from("/dev/sda2"),
                name: "sda2".to_string(),
                device_id: Some(DeviceId::from_major_minor(8, 2)),
                device_type: DeviceType::Partition,
                size: Some(50 * 1024 * 1024 * 1024),
                sector_size: Some(512),
                logical_sector_size: Some(512),
                model: None,
                serial: None,
                vendor: None,
                rotation_rate: None,
                removable: Some(false),
                read_only: Some(false),
                partition_table: None,
                uuid: Some("root-uuid".to_string()),
                partuuid: Some("root-partuuid".to_string()),
                label: Some("root".to_string()),
                partition_type: Some(PartitionType::LinuxFilesystem),
                filesystem: Some(FilesystemType::Ext4),
                mountpoint: Some(PathBuf::from("/")),
                mountpoints: vec![],
                mount_options: vec![],
                parent: Some("sda".to_string()),
                sysfs_path: None,
                children: vec![],
            },
            root_filesystem: FilesystemInfo {
                fstype: FilesystemType::Ext4,
                uuid: Some("root-uuid".to_string()),
                label: Some("root".to_string()),
                partuuid: Some("root-partuuid".to_string()),
                version: None,
                subsystems_uuid: None,
                subsystems_type: None,
            },
            boot_partitions: vec![],
            additional_partitions: vec![],
            distribution: Some("CachyOS".to_string()),
            version: Some("2026.11".to_string()),
            distribution_id: Some("cachyos".to_string()),
            kernel_paths: vec![],
            initramfs_paths: vec![],
            has_fstab: false,
            fstab_content: None,
            has_os_release: true,
            is_btrfs_subvolume: false,
            btrfs_subvolume: None,
            confidence: 80,
            reasons: vec![],
        };

        // Teste: /boot separado, ESP em /boot/efi
        let esp = Some(EspInfo {
            device_path: PathBuf::from("/dev/sda1"),
            uuid: Some("ESP-UUID".to_string()),
            partuuid: None,
            label: None,
            mountpoint: Some(PathBuf::from("/boot/efi")),
            size: 512 * 1024 * 1024,
            free_space: Some(256 * 1024 * 1024),
            confidence: 90,
        });

        candidate.additional_partitions.push(BlockDevice {
            path: PathBuf::from("/dev/sda3"),
            name: "sda3".to_string(),
            device_id: Some(DeviceId::from_major_minor(8, 3)),
            device_type: DeviceType::Partition,
            size: Some(1024 * 1024 * 1024),
            sector_size: Some(512),
            logical_sector_size: Some(512),
            model: None,
            serial: None,
            vendor: None,
            rotation_rate: None,
            removable: Some(false),
            read_only: Some(false),
            partition_table: None,
            uuid: Some("boot-uuid".to_string()),
            partuuid: Some("boot-partuuid".to_string()),
            label: Some("boot".to_string()),
            partition_type: Some(PartitionType::LinuxFilesystem),
            filesystem: Some(FilesystemType::Ext4),
            mountpoint: Some(PathBuf::from("/boot")),
            mountpoints: vec![PathBuf::from("/boot")],
            mount_options: vec![],
            parent: Some("sda".to_string()),
            sysfs_path: None,
            children: vec![],
        });

        let layout = detect_boot_layout(&candidate, &esp, &UefiState::Uefi);
        assert_eq!(layout, BootLayout::SeparateBootEfi);
    }
}
