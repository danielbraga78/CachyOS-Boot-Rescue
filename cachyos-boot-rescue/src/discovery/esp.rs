//! Identificação de candidatos a EFI System Partition (ESP).
//!
//! Identifica ESPs por tipo de partição GPT e filesystem FAT,
//! sem assumir que toda partição FAT é EFI.

use crate::discovery::types::{BlockDevice, EspCandidate, FilesystemInfo, FilesystemType};

/// Identifica candidatos a ESP a partir de dispositivos de bloco.
pub fn identify_esp_candidates(devices: &[BlockDevice]) -> Vec<EspCandidate> {
    let mut candidates = Vec::new();

    for device in devices {
        // Verifica partições no nível superior
        if let Some(candidate) = analyze_esp_candidate(device) {
            candidates.push(candidate);
        }

        // Verifica partições filhas recursivamente
        for child in &device.children {
            if let Some(candidate) = analyze_esp_candidate(child) {
                candidates.push(candidate);
            }
        }
    }

    // Ordena por confiança (maior primeiro)
    candidates.sort_by_key(|a| std::cmp::Reverse(a.confidence));

    candidates
}

/// Analisa se um dispositivo é candidato a ESP.
pub fn analyze_esp_candidate(device: &BlockDevice) -> Option<EspCandidate> {
    // Só considera partições
    if device.device_type != crate::discovery::types::DeviceType::Partition {
        return None;
    }

    let mut reasons = Vec::new();
    let mut confidence: u8 = 0;

    // Critério 1: Tipo de partição GPT EFI System
    let is_efi_partition_type = device.partition_type.as_ref().is_some_and(|p| p.is_esp());

    if is_efi_partition_type {
        confidence += 40;
        reasons.push("Tipo de partição GPT: EFI System Partition".to_string());
    }

    // Critério 2: Filesystem FAT/VFAT
    let is_fat_fs = device.filesystem.as_ref().is_some_and(|f| f.is_fat());

    if is_fat_fs {
        confidence += 30;
        reasons.push(format!(
            "Filesystem: {:?}",
            device.filesystem.as_ref().unwrap()
        ));
    }

    // Critério 3: PARTUUID conhecido de ESP (fallback)
    if !is_efi_partition_type && is_fat_fs {
        // Verifica se PARTUUID corresponde ao padrão ESP
        if let Some(partuuid) = &device.partuuid
            && partuuid.to_lowercase().starts_with("c12a7328")
        {
            confidence += 20;
            reasons.push("PARTUUID corresponde a EFI System".to_string());
        }
    }

    // Critério 4: Label sugere ESP
    if let Some(label) = &device.label {
        let lower = label.to_lowercase();
        if lower.contains("efi") || lower.contains("boot") || lower == "esp" {
            confidence += 10;
            reasons.push(format!("Label sugere ESP: {}", label));
        }
    }

    // Critério 5: Montada em /boot/efi ou /efi
    let (is_boot_efi_mount, is_boot_mount) = check_mount_points(device);

    if is_boot_efi_mount {
        confidence += 15;
        reasons.push("Montada em /boot/efi".to_string());
    } else if is_boot_mount {
        confidence += 5;
        reasons.push("Montada em /boot".to_string());
    }

    // Critério 6: Tamanho típico de ESP (100MB - 1GB)
    if let Some(size) = device.size
        && (50 * 1024 * 1024..=2 * 1024 * 1024 * 1024).contains(&size)
    {
        confidence += 5;
        reasons.push("Tamanho compatível com ESP".to_string());
    }

    // Só considera candidato se tiver pontuação mínima
    if confidence == 0 {
        return None;
    }

    // Informações de filesystem
    let filesystem = FilesystemInfo {
        fstype: device.filesystem.unwrap_or(FilesystemType::Unknown),
        uuid: device.uuid.clone(),
        label: device.label.clone(),
        partuuid: device.partuuid.clone(),
        version: None,
        subsystems_uuid: None,
        subsystems_type: None,
    };

    // Estima espaço livre se montado
    let free_space = estimate_free_space(device);

    Some(EspCandidate {
        device: device.clone(),
        filesystem,
        mountpoint: device.mountpoint.clone(),
        is_boot_efi_mount,
        is_boot_mount,
        size: device.size.unwrap_or(0),
        free_space,
        confidence: confidence.min(100),
        reasons,
    })
}

/// Verifica pontos de montagem típicos de ESP.
fn check_mount_points(device: &BlockDevice) -> (bool, bool) {
    let mut is_boot_efi = false;
    let mut is_boot = false;

    // Verifica mountpoint principal
    if let Some(mountpoint) = &device.mountpoint {
        let path_str = mountpoint.to_string_lossy();
        if path_str == "/boot/efi" || path_str == "/efi" {
            is_boot_efi = true;
        } else if path_str == "/boot" {
            is_boot = true;
        }
    }

    // Também verifica mountpoints alternativos (lsblk MOUNTPOINTS field)
    for mp in &device.mountpoints {
        let path_str = mp.to_string_lossy();
        if path_str == "/boot/efi" || path_str == "/efi" {
            is_boot_efi = true;
        } else if path_str == "/boot" {
            is_boot = true;
        }
    }

    (is_boot_efi, is_boot)
}

/// Estima espaço livre na partição se montada.
/// Na fase de descoberta somente leitura, retorna None para evitar execução de comandos externos.
/// O espaço livre pode ser verificado posteriormente durante a fase de montagem, se necessário.
fn estimate_free_space(_device: &BlockDevice) -> Option<u64> {
    None
}

/// Filtra ESPs duplicados (mesmo dispositivo).
pub fn deduplicate_esp_candidates(candidates: Vec<EspCandidate>) -> Vec<EspCandidate> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for candidate in candidates {
        let key = candidate.device.path.to_string_lossy().to_string();
        if seen.insert(key) {
            result.push(candidate);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::types::{
        BlockDevice, DeviceId, DeviceType, FilesystemType, PartitionType,
    };
    use std::path::PathBuf;

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
    fn test_identify_esp_candidates_efi_type_fat() {
        let devices = vec![create_test_partition(
            "/dev/sda1",
            PartitionType::EfiSystem,
            FilesystemType::Vfat,
            512 * 1024 * 1024,
            Some("EFI"),
            Some("/boot/efi"),
        )];

        let candidates = identify_esp_candidates(&devices);
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].confidence > 80);
        assert!(candidates[0].is_boot_efi_mount);
    }

    #[test]
    fn test_identify_esp_candidates_fat_only() {
        let devices = vec![create_test_partition(
            "/dev/sda1",
            PartitionType::MicrosoftBasicData,
            FilesystemType::Vfat,
            512 * 1024 * 1024,
            Some("EFI"),
            None,
        )];

        let candidates = identify_esp_candidates(&devices);
        assert_eq!(candidates.len(), 1);
        // FAT + label EFI + tamanho OK = ~45 confiança
        assert!(candidates[0].confidence >= 40);
    }

    #[test]
    fn test_identify_esp_candidates_not_fat() {
        let devices = vec![create_test_partition(
            "/dev/sda1",
            PartitionType::EfiSystem,
            FilesystemType::Ext4,
            512 * 1024 * 1024,
            None,
            None,
        )];

        let candidates = identify_esp_candidates(&devices);
        // Deve ter confiança baixa (só tipo de partição)
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].confidence < 50);
    }

    #[test]
    fn test_identify_esp_candidates_ignores_non_partitions() {
        let disk = BlockDevice {
            path: PathBuf::from("/dev/sda"),
            name: "sda".to_string(),
            device_id: Some(DeviceId::from_major_minor(0, 0)),
            device_type: DeviceType::Disk,
            size: Some(500 * 1024 * 1024 * 1024),
            sector_size: Some(512),
            logical_sector_size: Some(512),
            model: None,
            serial: None,
            vendor: None,
            rotation_rate: None,
            removable: Some(false),
            read_only: Some(false),
            partition_table: None,
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
            children: vec![create_test_partition(
                "/dev/sda1",
                PartitionType::EfiSystem,
                FilesystemType::Vfat,
                512 * 1024 * 1024,
                None,
                None,
            )],
        };

        let candidates = identify_esp_candidates(&[disk]);
        assert_eq!(candidates.len(), 1); // Apenas a partição filha
    }

    #[test]
    fn test_deduplicate_esp_candidates() {
        let esp = create_test_partition(
            "/dev/sda1",
            PartitionType::EfiSystem,
            FilesystemType::Vfat,
            512 * 1024 * 1024,
            None,
            None,
        );
        let candidate = analyze_esp_candidate(&esp).unwrap();

        let candidates = vec![candidate.clone(), candidate];
        let deduped = deduplicate_esp_candidates(candidates);
        assert_eq!(deduped.len(), 1);
    }

    #[test]
    fn test_check_mount_points() {
        let mut device = create_test_partition(
            "/dev/sda1",
            PartitionType::EfiSystem,
            FilesystemType::Vfat,
            512 * 1024 * 1024,
            None,
            Some("/boot/efi"),
        );

        let (is_efi, is_boot) = check_mount_points(&device);
        assert!(is_efi);
        assert!(!is_boot);

        device.mountpoint = Some(PathBuf::from("/boot"));
        let (is_efi, is_boot) = check_mount_points(&device);
        assert!(!is_efi);
        assert!(is_boot);
    }
}
