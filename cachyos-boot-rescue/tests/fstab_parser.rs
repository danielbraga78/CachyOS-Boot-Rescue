use cachyos_boot_rescue::discovery::types::{
    BlockDevice, DeviceId, DeviceType, FilesystemType, PartitionType,
};
use cachyos_boot_rescue::storage::fstab::{
    FstabDeviceSpec, FstabParseError, parse_fstab, resolve_device_specs_detailed,
};
use std::path::PathBuf;

fn device(
    path: &str,
    uuid: Option<&str>,
    partuuid: Option<&str>,
    label: Option<&str>,
) -> BlockDevice {
    BlockDevice {
        path: PathBuf::from(path),
        name: PathBuf::from(path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        device_id: Some(DeviceId::from_major_minor(8, 1)),
        device_type: DeviceType::Partition,
        size: Some(1_000_000),
        sector_size: Some(512),
        logical_sector_size: Some(512),
        model: None,
        serial: None,
        vendor: None,
        rotation_rate: None,
        removable: Some(false),
        read_only: Some(false),
        partition_table: None,
        uuid: uuid.map(str::to_owned),
        partuuid: partuuid.map(str::to_owned),
        label: label.map(str::to_owned),
        partition_type: Some(PartitionType::LinuxFilesystem),
        filesystem: Some(FilesystemType::Ext4),
        mountpoint: None,
        mountpoints: Vec::new(),
        mount_options: Vec::new(),
        parent: Some("sda".to_owned()),
        sysfs_path: None,
        children: Vec::new(),
    }
}

#[test]
fn parses_device_identifiers_mounts_options_and_fstab_escapes() {
    let input = concat!(
        "UUID=root-uuid / ext4 defaults 0 1\n",
        "PARTUUID=boot-partuuid /boot vfat nofail,rw 0 2\n",
        "LABEL=home\\040disk /home\\040data btrfs rw,relatime,subvol=@home 0 2 # home\n",
        "/dev/disk/by-id/root /mnt/data xfs noauto,ro 0 0\n",
    );

    let (entries, errors) = parse_fstab(input).unwrap();
    assert!(errors.is_empty());
    assert_eq!(entries.len(), 4);

    assert_eq!(
        entries[0].device_spec,
        FstabDeviceSpec::Uuid("root-uuid".into())
    );
    assert_eq!(
        entries[1].device_spec,
        FstabDeviceSpec::PartUuid("boot-partuuid".into())
    );
    assert!(entries[1].options.nofail);
    assert_eq!(
        entries[2].device_spec,
        FstabDeviceSpec::Label("home disk".into())
    );
    assert_eq!(entries[2].mount_point, PathBuf::from("/home data"));
    assert_eq!(entries[2].options.subvol.as_deref(), Some("@home"));
    assert_eq!(entries[2].options.other, ["relatime"]);
    assert_eq!(
        entries[3].device_spec,
        FstabDeviceSpec::Path(PathBuf::from("/dev/disk/by-id/root"))
    );
    assert!(entries[3].options.noauto);
    assert!(entries[3].options.read_only);
}

#[test]
fn parses_subvolid_and_keeps_unrecognized_options() {
    let (entries, errors) = parse_fstab(
        "UUID=root / btrfs rw,subvolid=256,compress=zstd:1,x-systemd.device-timeout=5s 0 1\n",
    )
    .unwrap();

    assert!(errors.is_empty());
    assert_eq!(entries[0].options.subvol.as_deref(), Some("id=256"));
    assert_eq!(
        entries[0].options.other,
        ["compress=zstd:1", "x-systemd.device-timeout=5s"]
    );
    assert_eq!(
        entries[0].options.to_mount_options(),
        "rw,subvolid=256,compress=zstd:1,x-systemd.device-timeout=5s"
    );
}

#[test]
fn accepts_swap_without_mountpoint_and_ignores_full_line_comments() {
    let (entries, errors) = parse_fstab("# swap entry\nUUID=swap-uuid none swap sw 0 0\n").unwrap();

    assert!(errors.is_empty());
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].mount_point, PathBuf::from("none"));
    assert_eq!(entries[0].fs_type, FilesystemType::Swap);
}

#[test]
fn collects_invalid_lines_and_preserves_valid_entries() {
    let input = concat!(
        "UUID=root / ext4 defaults 0 1\n",
        "UUID= / ext4 defaults 0 1\n",
        "UUID=short / ext4 defaults 0 1 extra\n",
        "UUID=missing / ext4 defaults 0\n",
        "UUID=relative relative ext4 defaults 0 1\n",
        "UUID=bad / ext4 defaults invalid 1\n",
        "UUID=bad-pass / ext4 defaults 0 invalid\n",
    );

    let (entries, errors) = parse_fstab(input).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(errors.len(), 6);
    assert!(matches!(
        errors[0],
        FstabParseError::InvalidUuidFormat { line: 2, .. }
    ));
    assert!(matches!(
        errors[1],
        FstabParseError::InvalidFieldCount { line: 3, found: 7 }
    ));
    assert!(matches!(
        errors[2],
        FstabParseError::InvalidFieldCount { line: 4, found: 5 }
    ));
    assert!(matches!(
        errors[3],
        FstabParseError::InvalidMountPoint { line: 5, .. }
    ));
    assert!(matches!(
        errors[4],
        FstabParseError::InvalidDump { line: 6, .. }
    ));
    assert!(matches!(
        errors[5],
        FstabParseError::InvalidPass { line: 7, .. }
    ));
}

#[test]
fn device_resolution_reports_missing_and_ambiguous_specs_without_guessing() {
    let input = concat!(
        "UUID=shared-uuid / ext4 defaults 0 1\n",
        "LABEL=not-present /home ext4 defaults 0 2\n",
        "PARTUUID=unique-partuuid /boot ext4 defaults 0 2\n",
    );
    let (entries, errors) = parse_fstab(input).unwrap();
    assert!(errors.is_empty());

    let devices = vec![
        device(
            "/dev/sda1",
            Some("shared-uuid"),
            Some("part-a"),
            Some("root-a"),
        ),
        device(
            "/dev/sdb1",
            Some("shared-uuid"),
            Some("part-b"),
            Some("root-b"),
        ),
        device(
            "/dev/sdc1",
            Some("other-uuid"),
            Some("unique-partuuid"),
            Some("boot"),
        ),
    ];
    let resolution = resolve_device_specs_detailed(&entries, &devices);

    assert_eq!(resolution.resolved.len(), 1);
    assert_eq!(
        resolution
            .resolved
            .get(&FstabDeviceSpec::PartUuid("unique-partuuid".into())),
        Some(&PathBuf::from("/dev/sdc1"))
    );
    assert_eq!(
        resolution
            .ambiguous
            .get(&FstabDeviceSpec::Uuid("shared-uuid".into())),
        Some(&vec![
            PathBuf::from("/dev/sda1"),
            PathBuf::from("/dev/sdb1")
        ])
    );
    assert_eq!(
        resolution.missing,
        [FstabDeviceSpec::Label("not-present".into())]
    );
}

#[test]
fn resolves_identifiers_on_nested_devices() {
    let mut disk = device("/dev/sda", None, None, None);
    disk.device_type = DeviceType::Disk;
    disk.partition_type = None;
    disk.filesystem = None;
    disk.children = vec![device(
        "/dev/sda2",
        Some("root-uuid"),
        Some("root-partuuid"),
        Some("root"),
    )];
    let (entries, errors) = parse_fstab("UUID=root-uuid / ext4 defaults 0 1\n").unwrap();
    assert!(errors.is_empty());

    let resolution = resolve_device_specs_detailed(&entries, &[disk]);
    assert_eq!(
        resolution
            .resolved
            .get(&FstabDeviceSpec::Uuid("root-uuid".into())),
        Some(&PathBuf::from("/dev/sda2"))
    );
}
