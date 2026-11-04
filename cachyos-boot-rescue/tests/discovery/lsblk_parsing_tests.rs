//! Testes de contrato/parsing para lsblk usando fixtures JSON.
//!
//! Cobre: discos SATA, NVMe, múltiplas ESPs, partições sem label,
//! discos removíveis e dados incompletos.

use cachyos_boot_rescue::discovery::types::{
    DeviceType, FilesystemType, PartitionTableType, PartitionType,
};
use cachyos_boot_rescue::discovery::{CommandOutput, parse_lsblk_json};
use std::fs;
use std::path::PathBuf;

/// Carrega uma fixture JSON de lsblk.
fn load_fixture(name: &str) -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_path = manifest_dir.join("fixtures").join("lsblk").join(name);
    fs::read_to_string(fixture_path).unwrap_or_else(|_| panic!("Fixture não encontrada: {}", name))
}

/// Cria CommandOutput a partir de fixture.
fn fixture_output(name: &str) -> CommandOutput {
    CommandOutput {
        exit_code: 0,
        stdout: load_fixture(name),
        stderr: String::new(),
        timed_out: false,
    }
}

#[test]
fn test_parse_sata_single_disk() {
    let output = fixture_output("sata_single_disk.json");
    let devices = parse_lsblk_json(&output).unwrap();

    assert_eq!(devices.len(), 1);
    let disk = &devices[0];

    // Disco
    assert_eq!(disk.name, "sda");
    assert_eq!(disk.path, PathBuf::from("/dev/sda"));
    assert_eq!(disk.device_type, DeviceType::Disk);
    assert_eq!(disk.size, Some(500107862016));
    assert_eq!(disk.model, Some("Samsung SSD 870 EVO 500GB".to_string()));
    assert_eq!(disk.serial, Some("S123456789".to_string()));
    assert_eq!(disk.vendor, Some("Samsung".to_string()));
    assert_eq!(disk.rotation_rate, Some(0));
    assert_eq!(disk.removable, Some(false));
    assert_eq!(disk.read_only, Some(false));
    assert_eq!(disk.partition_table, Some(PartitionTableType::Gpt));
    assert_eq!(disk.children.len(), 3);

    // Partição 1 - ESP
    let p1 = &disk.children[0];
    assert_eq!(p1.name, "sda1");
    assert_eq!(p1.device_type, DeviceType::Partition);
    assert_eq!(p1.size, Some(536870912));
    assert_eq!(p1.filesystem, Some(FilesystemType::Vfat));
    assert_eq!(p1.label, Some("EFI".to_string()));
    assert_eq!(p1.uuid, Some("ABCD-1234".to_string()));
    assert_eq!(
        p1.partuuid,
        Some("c12a7328-f81f-11d2-ba4b-00a0c93ec93b".to_string())
    );
    assert_eq!(p1.partition_type, Some(PartitionType::EfiSystem));
    assert_eq!(p1.mountpoint, Some(PathBuf::from("/boot/efi")));
    assert!(p1.mount_options.contains(&"rw".to_string()));

    // Partição 2 - root ext4
    let p2 = &disk.children[1];
    assert_eq!(p2.name, "sda2");
    assert_eq!(p2.filesystem, Some(FilesystemType::Ext4));
    assert_eq!(p2.label, Some("root".to_string()));
    assert_eq!(p2.partition_type, Some(PartitionType::LinuxFilesystem));
    assert_eq!(p2.mountpoint, Some(PathBuf::from("/")));

    // Partição 3 - home ext4
    let p3 = &disk.children[2];
    assert_eq!(p3.name, "sda3");
    assert_eq!(p3.filesystem, Some(FilesystemType::Ext4));
    assert_eq!(p3.label, Some("home".to_string()));
    assert_eq!(p3.mountpoint, Some(PathBuf::from("/home")));
}

#[test]
fn test_parse_nvme_single_disk() {
    let output = fixture_output("nvme_single_disk.json");
    let devices = parse_lsblk_json(&output).unwrap();

    assert_eq!(devices.len(), 1);
    let disk = &devices[0];

    assert_eq!(disk.name, "nvme0n1");
    assert_eq!(disk.path, PathBuf::from("/dev/nvme0n1"));
    assert_eq!(disk.device_type, DeviceType::Disk);
    assert_eq!(disk.size, Some(1000204886016));
    assert_eq!(disk.model, Some("Samsung SSD 970 EVO Plus 1TB".to_string()));
    assert_eq!(disk.rotation_rate, Some(0));
    assert_eq!(disk.partition_table, Some(PartitionTableType::Gpt));
    assert_eq!(disk.children.len(), 3);

    // Partição 1 - ESP em /efi
    let p1 = &disk.children[0];
    assert_eq!(p1.name, "nvme0n1p1");
    assert_eq!(p1.filesystem, Some(FilesystemType::Vfat));
    assert_eq!(p1.mountpoint, Some(PathBuf::from("/efi")));

    // Partição 2 - root btrfs com subvolume
    let p2 = &disk.children[1];
    assert_eq!(p2.name, "nvme0n1p2");
    assert_eq!(p2.filesystem, Some(FilesystemType::Btrfs));
    assert!(p2.mount_options.contains(&"subvol=@".to_string()));

    // Partição 3 - home btrfs com subvolume
    let p3 = &disk.children[2];
    assert_eq!(p3.name, "nvme0n1p3");
    assert_eq!(p3.filesystem, Some(FilesystemType::Btrfs));
    assert!(p3.mount_options.contains(&"subvol=@home".to_string()));
}

#[test]
fn test_parse_multiple_esps() {
    let output = fixture_output("multiple_esps.json");
    let devices = parse_lsblk_json(&output).unwrap();

    assert_eq!(devices.len(), 2); // sda e sdb

    // sda tem 4 partições
    let sda = &devices[0];
    assert_eq!(sda.name, "sda");
    assert_eq!(sda.children.len(), 4);

    // sda1 - ESP Linux
    let sda1 = &sda.children[0];
    assert_eq!(sda1.partition_type, Some(PartitionType::EfiSystem));
    assert_eq!(sda1.filesystem, Some(FilesystemType::Vfat));
    assert_eq!(sda1.mountpoint, Some(PathBuf::from("/boot/efi")));

    // sda2 - root Linux
    let sda2 = &sda.children[1];
    assert_eq!(sda2.partition_type, Some(PartitionType::LinuxFilesystem));
    assert_eq!(sda2.filesystem, Some(FilesystemType::Ext4));

    // sda3 - ESP Windows (não montada)
    let sda3 = &sda.children[2];
    assert_eq!(sda3.partition_type, Some(PartitionType::EfiSystem));
    assert_eq!(sda3.filesystem, Some(FilesystemType::Vfat));
    assert!(sda3.mountpoint.is_none());

    // sda4 - Windows NTFS
    let sda4 = &sda.children[3];
    assert_eq!(sda4.partition_type, Some(PartitionType::MicrosoftBasicData));
    assert_eq!(sda4.filesystem, Some(FilesystemType::Ntfs));

    // sdb tem 2 partições
    let sdb = &devices[1];
    assert_eq!(sdb.name, "sdb");
    assert_eq!(sdb.children.len(), 2);

    // sdb1 - ESP backup (não montada)
    let sdb1 = &sdb.children[0];
    assert_eq!(sdb1.partition_type, Some(PartitionType::EfiSystem));
    assert!(sdb1.mountpoint.is_none());

    // sdb2 - data ext4
    let sdb2 = &sdb.children[1];
    assert_eq!(sdb2.filesystem, Some(FilesystemType::Ext4));
    assert_eq!(sdb2.mountpoint, Some(PathBuf::from("/mnt/data")));
}

#[test]
fn test_parse_no_labels() {
    let output = fixture_output("no_labels.json");
    let devices = parse_lsblk_json(&output).unwrap();

    assert_eq!(devices.len(), 1);
    let disk = &devices[0];

    // Partições sem label devem ter None
    let p1 = &disk.children[0];
    assert_eq!(p1.label, None);
    assert_eq!(p1.uuid, Some("1234-5678".to_string()));
    assert_eq!(p1.filesystem, Some(FilesystemType::Vfat));

    let p2 = &disk.children[1];
    assert_eq!(p2.label, None);
    assert_eq!(
        p2.uuid,
        Some("bbbbbbbb-cccc-dddd-eeee-ffffffffffff".to_string())
    );
    assert_eq!(p2.filesystem, Some(FilesystemType::Ext4));
}

#[test]
fn test_parse_removable_disks() {
    let output = fixture_output("removable_disks.json");
    let devices = parse_lsblk_json(&output).unwrap();

    assert_eq!(devices.len(), 3);

    // sda - interno
    let sda = &devices[0];
    assert_eq!(sda.name, "sda");
    assert_eq!(sda.removable, Some(false));
    assert_eq!(sda.rotation_rate, Some(0));

    // sdb - USB removível
    let sdb = &devices[1];
    assert_eq!(sdb.name, "sdb");
    assert_eq!(sdb.removable, Some(true));
    assert_eq!(sdb.model, Some("USB Flash Drive".to_string()));
    assert_eq!(sdb.children.len(), 1);
    assert_eq!(
        sdb.children[0].mountpoint,
        Some(PathBuf::from("/run/media/user/LIVE_BOOT"))
    );

    // sdc - SD card MBR
    let sdc = &devices[2];
    assert_eq!(sdc.name, "sdc");
    assert_eq!(sdc.removable, Some(true));
    assert_eq!(sdc.partition_table, Some(PartitionTableType::Mbr));
    assert_eq!(sdc.children.len(), 1);
    let sdc1 = &sdc.children[0];
    assert_eq!(sdc1.partition_type, Some(PartitionType::Mbr(0x0c))); // Win95 FAT32 LBA
}

#[test]
fn test_parse_incomplete_data() {
    let output = fixture_output("incomplete_data.json");
    let devices = parse_lsblk_json(&output).unwrap();

    assert_eq!(devices.len(), 2);

    // sda com dados parciais
    let sda = &devices[0];
    assert_eq!(sda.name, "sda");
    assert_eq!(sda.model, None);
    assert_eq!(sda.serial, None);
    assert_eq!(sda.vendor, None);
    assert_eq!(sda.rotation_rate, None);

    // sda1 - partição sem quase nada
    let sda1 = &sda.children[0];
    assert_eq!(sda1.filesystem, None);
    assert_eq!(sda1.label, None);
    assert_eq!(sda1.uuid, None);
    assert_eq!(sda1.partuuid, None);
    assert_eq!(sda1.partition_type, None);
    assert!(sda1.mountpoint.is_none());

    // sda2 - partição com alguns dados
    let sda2 = &sda.children[1];
    assert_eq!(sda2.filesystem, Some(FilesystemType::Ext4));
    assert_eq!(
        sda2.uuid,
        Some("11111111-2222-3333-4444-555555555555".to_string())
    );
    assert_eq!(sda2.partuuid, None); // ausente

    // loop0 - loop device
    let loop0 = &devices[1];
    assert_eq!(loop0.name, "loop0");
    assert_eq!(loop0.device_type, DeviceType::Loop);
    assert_eq!(loop0.filesystem, Some(FilesystemType::Unknown)); // squashfs não mapeado
    assert_eq!(loop0.read_only, Some(true));
    assert_eq!(
        loop0.mountpoint,
        Some(PathBuf::from("/run/archiso/bootmnt"))
    );
}

#[test]
fn test_parse_luks_encrypted() {
    let output = fixture_output("luks_encrypted.json");
    let devices = parse_lsblk_json(&output).unwrap();

    assert_eq!(devices.len(), 2);

    // nvme0n1
    let nvme = &devices[0];
    assert_eq!(nvme.name, "nvme0n1");
    assert_eq!(nvme.children.len(), 2);

    // p1 - ESP
    let p1 = &nvme.children[0];
    assert_eq!(p1.filesystem, Some(FilesystemType::Vfat));
    assert_eq!(p1.partition_type, Some(PartitionType::EfiSystem));

    // p2 - LUKS
    let p2 = &nvme.children[1];
    assert_eq!(p2.filesystem, Some(FilesystemType::Luks));
    assert_eq!(p2.label, Some("luks_root".to_string()));
    assert!(p2.uuid.as_ref().unwrap().starts_with("luks-uuid"));

    // dm-0 - dispositivo descriptografado
    let dm0 = &devices[1];
    assert_eq!(dm0.name, "dm-0");
    assert_eq!(dm0.device_type, DeviceType::Crypto);
    assert_eq!(dm0.filesystem, Some(FilesystemType::Btrfs));
    assert_eq!(dm0.mountpoint, Some(PathBuf::from("/")));
    assert_eq!(dm0.parent, Some("nvme0n1p2".to_string()));
}

#[test]
fn test_parse_missing_fixture_graceful() {
    let output = CommandOutput {
        exit_code: 0,
        stdout: r#"{"blockdevices":[]}"#.to_string(),
        stderr: String::new(),
        timed_out: false,
    };
    let devices = parse_lsblk_json(&output).unwrap();
    assert!(devices.is_empty());
}

#[test]
fn test_parse_empty_blockdevices() {
    let output = CommandOutput {
        exit_code: 0,
        stdout: r#"{"blockdevices":[]}"#.to_string(),
        stderr: String::new(),
        timed_out: false,
    };
    let devices = parse_lsblk_json(&output).unwrap();
    assert!(devices.is_empty());
}
