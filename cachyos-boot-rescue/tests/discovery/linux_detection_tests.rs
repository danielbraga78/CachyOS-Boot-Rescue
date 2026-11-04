//! Testes para identificação de instalações Linux candidatas.

use cachyos_boot_rescue::discovery::identify_linux_installations_with_root;
use cachyos_boot_rescue::discovery::types::{
    BlockDevice, DeviceId, DeviceType, FilesystemInfo, FilesystemType, PartitionType,
};
use cachyos_boot_rescue::discovery::{
    find_kernels_and_initramfs, identify_from_metadata, parse_os_release, read_fstab,
};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

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

fn create_esp_candidate(
    path: &str,
    mountpoint: Option<&str>,
) -> cachyos_boot_rescue::discovery::types::EspCandidate {
    let esp = create_partition(
        path,
        PartitionType::EfiSystem,
        FilesystemType::Vfat,
        512 * 1024 * 1024,
        Some("EFI"),
        mountpoint,
    );
    let fs_info = FilesystemInfo {
        fstype: FilesystemType::Vfat,
        uuid: Some("ESP-UUID".to_string()),
        label: Some("EFI".to_string()),
        partuuid: esp.partuuid.clone(),
        version: None,
        subsystems_uuid: None,
        subsystems_type: None,
    };
    cachyos_boot_rescue::discovery::types::EspCandidate {
        device: esp,
        filesystem: fs_info,
        mountpoint: mountpoint.map(PathBuf::from),
        is_boot_efi_mount: mountpoint == Some("/boot/efi") || mountpoint == Some("/efi"),
        is_boot_mount: mountpoint == Some("/boot"),
        size: 512 * 1024 * 1024,
        free_space: None,
        confidence: 90,
        reasons: vec!["test".to_string()],
    }
}

#[test]
fn test_identify_linux_installations_empty() {
    let devices = vec![];
    let esps = vec![];
    let result = identify_linux_installations_with_root(&devices, &esps, Path::new("/"));
    assert!(result.is_empty());
}

#[test]
fn test_identify_linux_installations_non_linux_fs() {
    let devices = vec![create_partition(
        "/dev/sda1",
        PartitionType::MicrosoftBasicData,
        FilesystemType::Ntfs,
        100 * 1024 * 1024 * 1024,
        Some("Windows"),
        Some("/mnt/windows"),
    )];
    let esps = vec![];
    let result = identify_linux_installations_with_root(&devices, &esps, Path::new("/"));
    assert!(result.is_empty());
}

#[test]
fn test_identify_linux_installations_not_mounted() {
    let devices = vec![create_partition(
        "/dev/sda1",
        PartitionType::LinuxFilesystem,
        FilesystemType::Ext4,
        50 * 1024 * 1024 * 1024,
        Some("root"),
        None, // não montada
    )];
    let esps = vec![];
    let result = identify_linux_installations_with_root(&devices, &esps, Path::new("/"));
    assert!(result.is_empty());
}

#[test]
fn test_identify_linux_installations_mounted_ext4() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir_all(&root).unwrap();

    // Cria /etc/os-release
    fs::create_dir_all(root.join("etc")).unwrap();
    fs::write(
        root.join("etc/os-release"),
        r#"PRETTY_NAME="CachyOS"
VERSION_ID="2026.11"
ID=cachyos
ID_LIKE=arch"#,
    )
    .unwrap();

    // Cria /etc/fstab
    fs::write(
        root.join("etc/fstab"),
        "UUID=xxx / ext4 defaults 0 1\nUUID=yyy /boot/efi vfat defaults 0 2\n",
    )
    .unwrap();

    // Cria kernels em /boot
    let boot = root.join("boot");
    fs::create_dir_all(&boot).unwrap();
    fs::write(boot.join("vmlinuz-linux"), b"kernel").unwrap();
    fs::write(boot.join("initramfs-linux.img"), b"initramfs").unwrap();

    let devices = vec![create_partition(
        "/dev/sda2",
        PartitionType::LinuxFilesystem,
        FilesystemType::Ext4,
        50 * 1024 * 1024 * 1024,
        Some("root"),
        Some(root.to_str().unwrap()),
    )];

    let esps = vec![create_esp_candidate("/dev/sda1", Some("/boot/efi"))];

    let result = identify_linux_installations_with_root(&devices, &esps, root.as_ref());
    assert_eq!(result.len(), 1);

    let candidate = &result[0];
    assert_eq!(candidate.root_device.path, PathBuf::from("/dev/sda2"));
    assert_eq!(candidate.distribution, Some("CachyOS".to_string()));
    assert_eq!(candidate.version, Some("2026.11".to_string()));
    assert_eq!(candidate.distribution_id, Some("cachyos".to_string()));
    assert!(candidate.has_os_release);
    assert!(candidate.has_fstab);
    assert_eq!(candidate.kernel_paths.len(), 1);
    assert_eq!(candidate.initramfs_paths.len(), 1);
    assert_eq!(candidate.boot_partitions.len(), 1);
    assert!(candidate.confidence > 50);
}

#[test]
fn test_identify_linux_installations_btrfs_subvolume() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir_all(&root).unwrap();

    fs::create_dir_all(root.join("etc")).unwrap();
    fs::write(root.join("etc/os-release"), "ID=arch\n").unwrap();

    // Simula subvolume @
    fs::create_dir_all(root.join("@")).unwrap();

    let devices = vec![create_partition(
        "/dev/sda2",
        PartitionType::LinuxFilesystem,
        FilesystemType::Btrfs,
        100 * 1024 * 1024 * 1024,
        Some("cachyos"),
        Some(root.to_str().unwrap()),
    )];

    let esps = vec![];

    let result = identify_linux_installations_with_root(&devices, &esps, root.as_ref());
    assert_eq!(result.len(), 1);

    let candidate = &result[0];
    assert!(candidate.is_btrfs_subvolume);
    // A detecção via mountinfo não funciona em teste, mas verifica diretório @
    assert_eq!(candidate.btrfs_subvolume, Some("@".to_string()));
}

#[test]
fn test_identify_from_metadata() {
    let devices = vec![
        create_partition(
            "/dev/sda1",
            PartitionType::LinuxFilesystem,
            FilesystemType::Ext4,
            50 * 1024 * 1024 * 1024,
            Some("root"),
            None, // não montada
        ),
        create_partition(
            "/dev/sda2",
            PartitionType::MicrosoftBasicData,
            FilesystemType::Ntfs,
            100 * 1024 * 1024 * 1024,
            Some("Windows"),
            None,
        ),
        create_partition(
            "/dev/sda3",
            PartitionType::LinuxSwap,
            FilesystemType::Swap,
            8 * 1024 * 1024 * 1024,
            None,
            None,
        ),
    ];

    let result = identify_from_metadata(&devices);
    // Deve encontrar apenas sda1 (Linux filesystem). Swap NÃO é instalação candidata.
    assert_eq!(result.len(), 1);

    let root_candidate = result
        .iter()
        .find(|c| c.root_device.path == Path::new("/dev/sda1"));
    assert!(root_candidate.is_some());
    assert_eq!(root_candidate.unwrap().confidence, 15);
    assert!(
        root_candidate
            .unwrap()
            .reasons
            .iter()
            .any(|r| r.contains("metadados"))
    );
}

#[test]
fn test_parse_os_release() {
    let dir = tempdir().unwrap();
    let os_release = dir.path().join("os-release");
    fs::write(
        &os_release,
        r#"PRETTY_NAME="CachyOS"
VERSION_ID="2026.11"
ID=cachyos
ID_LIKE=arch"#,
    )
    .unwrap();

    let (dist, ver, id, has) = parse_os_release(&os_release);
    assert!(has);
    assert_eq!(dist, Some("CachyOS".to_string()));
    assert_eq!(ver, Some("2026.11".to_string()));
    assert_eq!(id, Some("cachyos".to_string()));
}

#[test]
fn test_parse_os_release_missing() {
    let dir = tempdir().unwrap();
    let os_release = dir.path().join("os-release");
    // Não cria o arquivo

    let (dist, ver, id, has) = parse_os_release(&os_release);
    assert!(!has);
    assert_eq!(dist, None);
    assert_eq!(ver, None);
    assert_eq!(id, None);
}

#[test]
fn test_read_fstab() {
    let dir = tempdir().unwrap();
    let fstab = dir.path().join("fstab");
    fs::write(
        &fstab,
        "UUID=xxx / ext4 defaults 0 1\nUUID=yyy /boot/efi vfat defaults 0 2\n",
    )
    .unwrap();

    let (has, content) = read_fstab(&fstab);
    assert!(has);
    let content = content.unwrap();
    assert!(content.contains("UUID=xxx"));
    assert!(content.contains("/boot/efi"));
}

#[test]
fn test_read_fstab_missing() {
    let dir = tempdir().unwrap();
    let fstab = dir.path().join("fstab");
    // Não cria

    let (has, content) = read_fstab(&fstab);
    assert!(!has);
    assert!(content.is_none());
}

#[test]
fn test_find_kernels_and_initramfs() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let boot = root.join("boot");
    fs::create_dir_all(&boot).unwrap();
    fs::write(boot.join("vmlinuz-linux"), b"kernel").unwrap();
    fs::write(boot.join("vmlinuz-linux-lts"), b"kernel-lts").unwrap();
    fs::write(boot.join("initramfs-linux.img"), b"initramfs").unwrap();
    fs::write(boot.join("initramfs-linux-lts.img"), b"initramfs-lts").unwrap();
    fs::write(boot.join("config.txt"), b"config").unwrap(); // não kernel

    let (kernels, initramfs) = find_kernels_and_initramfs(root);
    assert_eq!(kernels.len(), 2);
    assert_eq!(initramfs.len(), 2);

    // Verifica nomes
    let kernel_names: Vec<_> = kernels
        .iter()
        .map(|p| p.file_name().unwrap().to_str().unwrap())
        .collect();
    assert!(kernel_names.contains(&"vmlinuz-linux"));
    assert!(kernel_names.contains(&"vmlinuz-linux-lts"));

    let initrd_names: Vec<_> = initramfs
        .iter()
        .map(|p| p.file_name().unwrap().to_str().unwrap())
        .collect();
    assert!(initrd_names.contains(&"initramfs-linux.img"));
    assert!(initrd_names.contains(&"initramfs-linux-lts.img"));
}

#[test]
fn test_find_kernels_efi_unified() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let efi = root.join("efi");
    fs::create_dir_all(&efi).unwrap();
    fs::write(efi.join("linux.efi"), b"unified kernel").unwrap();

    let (kernels, initramfs) = find_kernels_and_initramfs(root);
    assert_eq!(kernels.len(), 1);
    assert_eq!(initramfs.len(), 0);
    assert_eq!(kernels[0].file_name().unwrap(), "linux.efi");
}
