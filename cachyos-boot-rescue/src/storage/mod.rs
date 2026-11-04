//! Módulo de gerenciamento de armazenamento e montagem.
//!
//! Inclui parser fstab, plano de montagem, sessão de montagem,
//! resolução de dispositivos, Btrfs subvolumes e LUKS.

pub mod btrfs;
pub mod device_resolver;
pub mod fstab;
pub mod luks;
pub mod mount_plan;
pub mod mount_session;

pub use btrfs::*;
pub use device_resolver::*;
pub use fstab::*;
pub use luks::*;
pub use mount_plan::*;
pub use mount_session::*;

// Re-exporta resolve_device_specs do device_resolver (versão mais completa)
pub use device_resolver::resolve_device_specs;

/// Constrói MountPlan a partir de candidato de instalação e dispositivos descobertos.
/// Esta é a função principal de integração do módulo storage.
pub fn build_mount_plan(
    candidate: &crate::discovery::types::LinuxInstallationCandidate,
    all_devices: &[crate::discovery::types::BlockDevice],
    fstab_content: Option<&str>,
    root_mount: &std::path::Path,
) -> MountPlan {
    mount_plan::build_mount_plan(candidate, all_devices, fstab_content, root_mount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::types::{
        BlockDevice, DeviceId, DeviceType, EspCandidate, FilesystemInfo, FilesystemType,
        LinuxInstallationCandidate, PartitionType,
    };
    use std::path::PathBuf;

    fn create_test_candidate() -> LinuxInstallationCandidate {
        let root_dev = BlockDevice {
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
        };

        LinuxInstallationCandidate {
            root_device: root_dev.clone(),
            root_filesystem: FilesystemInfo {
                fstype: FilesystemType::Ext4,
                uuid: Some("root-uuid".to_string()),
                label: Some("root".to_string()),
                partuuid: Some("root-partuuid".to_string()),
                version: None,
                subsystems_uuid: None,
                subsystems_type: None,
            },
            boot_partitions: vec![EspCandidate {
                device: BlockDevice {
                    path: PathBuf::from("/dev/sda1"),
                    name: "sda1".to_string(),
                    device_id: Some(DeviceId::from_major_minor(8, 1)),
                    device_type: DeviceType::Partition,
                    size: Some(512 * 1024 * 1024),
                    sector_size: Some(512),
                    logical_sector_size: Some(512),
                    model: None,
                    serial: None,
                    vendor: None,
                    rotation_rate: None,
                    removable: Some(false),
                    read_only: Some(false),
                    partition_table: None,
                    uuid: Some("esp-uuid".to_string()),
                    partuuid: Some("esp-partuuid".to_string()),
                    label: Some("EFI".to_string()),
                    partition_type: Some(PartitionType::EfiSystem),
                    filesystem: Some(FilesystemType::Vfat),
                    mountpoint: Some(PathBuf::from("/boot/efi")),
                    mountpoints: vec![],
                    mount_options: vec![],
                    parent: Some("sda".to_string()),
                    sysfs_path: None,
                    children: vec![],
                },
                filesystem: FilesystemInfo {
                    fstype: FilesystemType::Vfat,
                    uuid: Some("esp-uuid".to_string()),
                    label: Some("EFI".to_string()),
                    partuuid: Some("esp-partuuid".to_string()),
                    version: None,
                    subsystems_uuid: None,
                    subsystems_type: None,
                },
                mountpoint: Some(PathBuf::from("/boot/efi")),
                is_boot_efi_mount: true,
                is_boot_mount: false,
                size: 512 * 1024 * 1024,
                free_space: None,
                confidence: 90,
                reasons: vec!["test".to_string()],
            }],
            additional_partitions: vec![],
            distribution: Some("CachyOS".to_string()),
            version: Some("2026.11".to_string()),
            distribution_id: Some("cachyos".to_string()),
            kernel_paths: vec![],
            initramfs_paths: vec![],
            has_fstab: true,
            fstab_content: Some(
                "UUID=root-uuid / ext4 defaults 0 1\nUUID=esp-uuid /boot/efi vfat defaults 0 2\n"
                    .to_string(),
            ),
            has_os_release: true,
            is_btrfs_subvolume: false,
            btrfs_subvolume: None,
            confidence: 80,
            reasons: vec!["test".to_string()],
        }
    }

    fn create_test_devices() -> Vec<BlockDevice> {
        vec![BlockDevice {
            path: PathBuf::from("/dev/sda"),
            name: "sda".to_string(),
            device_id: Some(DeviceId::from_major_minor(8, 0)),
            device_type: DeviceType::Disk,
            size: Some(500 * 1024 * 1024 * 1024),
            sector_size: Some(512),
            logical_sector_size: Some(512),
            model: Some("Test Disk".to_string()),
            serial: None,
            vendor: None,
            rotation_rate: Some(0),
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
                BlockDevice {
                    path: PathBuf::from("/dev/sda1"),
                    name: "sda1".to_string(),
                    device_id: Some(DeviceId::from_major_minor(8, 1)),
                    device_type: DeviceType::Partition,
                    size: Some(512 * 1024 * 1024),
                    sector_size: Some(512),
                    logical_sector_size: Some(512),
                    model: None,
                    serial: None,
                    vendor: None,
                    rotation_rate: None,
                    removable: Some(false),
                    read_only: Some(false),
                    partition_table: None,
                    uuid: Some("esp-uuid".to_string()),
                    partuuid: Some("esp-partuuid".to_string()),
                    label: Some("EFI".to_string()),
                    partition_type: Some(PartitionType::EfiSystem),
                    filesystem: Some(FilesystemType::Vfat),
                    mountpoint: Some(PathBuf::from("/boot/efi")),
                    mountpoints: vec![],
                    mount_options: vec![],
                    parent: Some("sda".to_string()),
                    sysfs_path: None,
                    children: vec![],
                },
                BlockDevice {
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
            ],
        }]
    }

    #[test]
    fn test_storage_module_integration() {
        let candidate = create_test_candidate();
        let devices = create_test_devices();
        let root_mount = PathBuf::from("/mnt/target");
        let plan = build_mount_plan(
            &candidate,
            &devices,
            candidate.fstab_content.as_deref(),
            &root_mount,
        );

        assert!(!plan.has_blocking_errors());
        assert_eq!(plan.entries.len(), 2);

        // Verifica que mount points são resolvidos corretamente
        let esp_entry = plan
            .entries
            .iter()
            .find(|e| e.mount_point.as_os_str() == "/boot/efi")
            .unwrap();
        assert_eq!(
            esp_entry.resolved_mount_point(&root_mount),
            PathBuf::from("/mnt/target/boot/efi")
        );
    }
}
