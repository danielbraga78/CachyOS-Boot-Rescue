use cachyos_boot_rescue::discovery::lsblk::parse_lsblk_json;
use cachyos_boot_rescue::discovery::types::{
    CommandOutput, DeviceId, DeviceType, FilesystemType, PartitionType,
};

fn parse_fixture(json: &str) -> Vec<cachyos_boot_rescue::discovery::types::BlockDevice> {
    parse_lsblk_json(&CommandOutput {
        exit_code: 0,
        stdout: json.to_owned(),
        stderr: String::new(),
        timed_out: false,
    })
    .expect("synthetic lsblk JSON should parse")
}

fn fixture_devices(json: &str) -> Vec<cachyos_boot_rescue::discovery::types::BlockDevice> {
    parse_fixture(json)
}

#[test]
fn parses_sata_disk_and_nested_partitions() {
    let devices = fixture_devices(include_str!("fixtures/lsblk/sata.json"));

    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].path.to_string_lossy(), "/dev/sda");
    assert_eq!(devices[0].device_type, DeviceType::Disk);
    assert_eq!(devices[0].device_id, Some(DeviceId::from_major_minor(8, 0)));
    assert_eq!(devices[0].size, Some(1_000_204_886_016));
    assert_eq!(devices[0].children.len(), 2);
    assert_eq!(devices[0].children[0].path.to_string_lossy(), "/dev/sda1");
    assert_eq!(
        devices[0].children[0].device_id,
        Some(DeviceId::from_major_minor(8, 1))
    );
    assert_eq!(devices[0].children[0].size, Some(536_870_912));
    assert_eq!(
        devices[0].children[0].partition_type,
        Some(PartitionType::EfiSystem)
    );
    assert_eq!(
        devices[0].children[0].mountpoint.as_deref(),
        Some(std::path::Path::new("/boot"))
    );
    assert_eq!(
        devices[0].children[1].filesystem,
        Some(FilesystemType::Btrfs)
    );
}

#[test]
fn parses_nvme_device_names_and_filesystems() {
    let devices = fixture_devices(include_str!("fixtures/lsblk/nvme.json"));

    assert_eq!(devices[0].path.to_string_lossy(), "/dev/nvme0n1");
    assert_eq!(
        devices[0].device_id,
        Some(DeviceId::from_major_minor(259, 0))
    );
    assert_eq!(devices[0].size, Some(2_000_398_934_016));
    assert_eq!(
        devices[0].children[0].path.to_string_lossy(),
        "/dev/nvme0n1p1"
    );
    assert_eq!(
        devices[0].children[1].filesystem,
        Some(FilesystemType::Ext4)
    );
}

#[test]
fn preserves_all_efi_partition_candidates_without_selecting_one() {
    let devices = fixture_devices(include_str!("fixtures/lsblk/multiple-esps.json"));
    let esp_paths: Vec<_> = devices
        .iter()
        .flat_map(|device| &device.children)
        .filter(|partition| partition.partition_type == Some(PartitionType::EfiSystem))
        .map(|partition| partition.path.to_string_lossy().into_owned())
        .collect();

    assert_eq!(esp_paths, ["/dev/sda1", "/dev/nvme0n1p1"]);
}

#[test]
fn leaves_partition_label_absent_when_fixture_has_no_label() {
    let devices = fixture_devices(include_str!("fixtures/lsblk/partition-without-label.json"));

    assert_eq!(devices[0].children[0].label, None);
    assert_eq!(devices[0].children[1].label, None);
}

#[test]
fn reports_removable_device_flag_from_fixture() {
    let devices = fixture_devices(include_str!("fixtures/lsblk/removable-disk.json"));

    assert_eq!(devices[0].path.to_string_lossy(), "/dev/sdc");
    assert_eq!(devices[0].removable, Some(true));
    assert_eq!(
        devices[0].children[0].filesystem,
        Some(FilesystemType::Unknown)
    );
}

#[test]
fn accepts_incomplete_metadata_and_preserves_unknown_filesystem() {
    let devices = fixture_devices(include_str!("fixtures/lsblk/incomplete-data.json"));

    assert_eq!(devices[0].children.len(), 2);
    assert_eq!(devices[0].children[0].filesystem, None);
    assert_eq!(devices[0].children[0].label, None);
    assert_eq!(
        devices[0].children[1].filesystem,
        Some(FilesystemType::Unknown)
    );
}
