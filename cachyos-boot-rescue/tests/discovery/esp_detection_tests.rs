//! Testes para identificação de candidatos ESP.

use cachyos_boot_rescue::discovery::identify_esp_candidates;
use cachyos_boot_rescue::discovery::types::{
    BlockDevice, DeviceId, DeviceType, FilesystemType, PartitionType,
};
use std::path::PathBuf;

fn create_partition(
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
fn test_identify_esp_efi_type_fat_mounted() {
    let devices = vec![create_partition(
        "/dev/sda1",
        PartitionType::EfiSystem,
        FilesystemType::Vfat,
        512 * 1024 * 1024,
        Some("EFI"),
        Some("/boot/efi"),
    )];

    let candidates = identify_esp_candidates(&devices);
    assert_eq!(candidates.len(), 1);
    let esp = &candidates[0];
    // EFI type (40) + FAT (30) + label EFI (10) + mount /boot/efi (15) + size (5) = 100
    assert_eq!(esp.confidence, 100);
    assert!(esp.is_boot_efi_mount);
    assert!(!esp.is_boot_mount);
    assert!(
        esp.reasons
            .iter()
            .any(|r| r.contains("EFI System Partition"))
    );
    assert!(esp.reasons.iter().any(|r| r.contains("Vfat"))); // Filesystem debug format
    assert!(esp.reasons.iter().any(|r| r.contains("/boot/efi")));
}

#[test]
fn test_identify_esp_fat_only_no_mount() {
    let devices = vec![create_partition(
        "/dev/sda1",
        PartitionType::MicrosoftBasicData,
        FilesystemType::Vfat,
        512 * 1024 * 1024,
        Some("EFI"),
        None,
    )];

    let candidates = identify_esp_candidates(&devices);
    assert_eq!(candidates.len(), 1);
    let esp = &candidates[0];
    // FAT + label EFI + tamanho = ~45
    assert!(esp.confidence >= 40 && esp.confidence < 60);
    assert!(!esp.is_boot_efi_mount);
    assert!(!esp.is_boot_mount);
}

#[test]
fn test_identify_esp_efi_type_not_fat() {
    let devices = vec![create_partition(
        "/dev/sda1",
        PartitionType::EfiSystem,
        FilesystemType::Ext4,
        512 * 1024 * 1024,
        None,
        None,
    )];

    let candidates = identify_esp_candidates(&devices);
    assert_eq!(candidates.len(), 1);
    let esp = &candidates[0];
    // EFI type (40) + size (5) = 45
    assert_eq!(esp.confidence, 45);
    assert!(
        esp.reasons
            .iter()
            .any(|r| r.contains("EFI System Partition"))
    );
}

#[test]
fn test_identify_esp_not_partition_ignored() {
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
        children: vec![create_partition(
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
    assert_eq!(candidates[0].device.path, PathBuf::from("/dev/sda1"));
}

#[test]
fn test_identify_esp_small_partition() {
    let devices = vec![create_partition(
        "/dev/sda1",
        PartitionType::EfiSystem,
        FilesystemType::Vfat,
        10 * 1024 * 1024, // 10MB - muito pequeno
        None,
        None,
    )];

    let candidates = identify_esp_candidates(&devices);
    assert_eq!(candidates.len(), 1);
    let esp = &candidates[0];
    // EFI type (40) + FAT (30) = 70, sem bônus de tamanho
    assert_eq!(esp.confidence, 70);
}

#[test]
fn test_identify_esp_large_partition() {
    let devices = vec![create_partition(
        "/dev/sda1",
        PartitionType::EfiSystem,
        FilesystemType::Vfat,
        3 * 1024 * 1024 * 1024, // 3GB - muito grande
        None,
        None,
    )];

    let candidates = identify_esp_candidates(&devices);
    assert_eq!(candidates.len(), 1);
    let esp = &candidates[0];
    // EFI type (40) + FAT (30) = 70, sem bônus de tamanho
    assert_eq!(esp.confidence, 70);
}

#[test]
fn test_identify_esp_fat16_fat32_vfat() {
    for fs in [
        FilesystemType::Fat16,
        FilesystemType::Fat32,
        FilesystemType::Vfat,
    ] {
        let devices = vec![create_partition(
            "/dev/sda1",
            PartitionType::EfiSystem,
            fs,
            512 * 1024 * 1024,
            None,
            None,
        )];

        let candidates = identify_esp_candidates(&devices);
        assert_eq!(candidates.len(), 1, "Falhou para {:?}", fs);
        assert!(candidates[0].confidence >= 40);
    }
}

#[test]
fn test_identify_esp_mount_efi_vs_boot() {
    // Montada em /efi
    let devices1 = vec![create_partition(
        "/dev/sda1",
        PartitionType::EfiSystem,
        FilesystemType::Vfat,
        512 * 1024 * 1024,
        None,
        Some("/efi"),
    )];
    let c1 = identify_esp_candidates(&devices1);
    assert!(c1[0].is_boot_efi_mount);
    assert!(!c1[0].is_boot_mount);

    // Montada em /boot
    let devices2 = vec![create_partition(
        "/dev/sda1",
        PartitionType::EfiSystem,
        FilesystemType::Vfat,
        512 * 1024 * 1024,
        None,
        Some("/boot"),
    )];
    let c2 = identify_esp_candidates(&devices2);
    assert!(!c2[0].is_boot_efi_mount);
    assert!(c2[0].is_boot_mount);

    // Montada em /mnt/efi (não conta)
    let devices3 = vec![create_partition(
        "/dev/sda1",
        PartitionType::EfiSystem,
        FilesystemType::Vfat,
        512 * 1024 * 1024,
        None,
        Some("/mnt/efi"),
    )];
    let c3 = identify_esp_candidates(&devices3);
    assert!(!c3[0].is_boot_efi_mount);
    assert!(!c3[0].is_boot_mount);
}

#[test]
fn test_identify_esp_multiple_sorted_by_confidence() {
    let devices = vec![
        create_partition(
            "/dev/sda1",
            PartitionType::MicrosoftBasicData,
            FilesystemType::Vfat,
            512 * 1024 * 1024,
            Some("DATA"),
            None,
        ),
        create_partition(
            "/dev/sda2",
            PartitionType::EfiSystem,
            FilesystemType::Vfat,
            512 * 1024 * 1024,
            Some("EFI"),
            Some("/boot/efi"),
        ),
        create_partition(
            "/dev/sda3",
            PartitionType::EfiSystem,
            FilesystemType::Vfat,
            512 * 1024 * 1024,
            None,
            None,
        ),
    ];

    let candidates = identify_esp_candidates(&devices);
    assert_eq!(candidates.len(), 3);

    // Deve estar ordenado por confiança decrescente
    assert!(candidates[0].confidence >= candidates[1].confidence);
    assert!(candidates[1].confidence >= candidates[2].confidence);

    // O melhor deve ser sda2 (EFI type + FAT + mount + label)
    assert_eq!(candidates[0].device.path, PathBuf::from("/dev/sda2"));
}
