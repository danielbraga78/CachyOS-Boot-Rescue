//! Testes de integração da descoberta completa com fixtures.

use cachyos_boot_rescue::discovery::enrich_with_blkid;
use cachyos_boot_rescue::discovery::types::DiscoveryReport;
use cachyos_boot_rescue::discovery::{CommandOutput, parse_lsblk_json};
use cachyos_boot_rescue::discovery::{identify_esp_candidates, identify_linux_installations};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn load_fixture(name: &str) -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_path = manifest_dir.join("fixtures").join("lsblk").join(name);
    fs::read_to_string(fixture_path).unwrap_or_else(|_| panic!("Fixture não encontrada: {}", name))
}

fn lsblk_output(name: &str) -> CommandOutput {
    CommandOutput {
        exit_code: 0,
        stdout: load_fixture(name),
        stderr: String::new(),
        timed_out: false,
    }
}

#[test]
fn test_full_discovery_sata_single_disk() {
    // Parse lsblk
    let devices = parse_lsblk_json(&lsblk_output("sata_single_disk.json")).unwrap();

    // Identifica ESPs
    let esps = identify_esp_candidates(&devices);
    assert_eq!(esps.len(), 1);
    assert_eq!(esps[0].device.path, PathBuf::from("/dev/sda1"));
    assert!(esps[0].is_boot_efi_mount);

    // Identifica instalações Linux
    let linux_candidates = identify_linux_installations(&devices, &esps);
    assert_eq!(linux_candidates.len(), 1);
    assert_eq!(
        linux_candidates[0].root_device.path,
        PathBuf::from("/dev/sda2")
    );
    assert_eq!(linux_candidates[0].boot_partitions.len(), 1);
}

#[test]
fn test_full_discovery_nvme_btrfs() {
    let devices = parse_lsblk_json(&lsblk_output("nvme_single_disk.json")).unwrap();

    let esps = identify_esp_candidates(&devices);
    assert_eq!(esps.len(), 1);
    assert_eq!(esps[0].device.path, PathBuf::from("/dev/nvme0n1p1"));
    assert_eq!(esps[0].mountpoint, Some(PathBuf::from("/efi")));

    let linux_candidates = identify_linux_installations(&devices, &esps);
    assert_eq!(linux_candidates.len(), 1);
    let candidate = &linux_candidates[0];
    assert_eq!(candidate.root_device.path, PathBuf::from("/dev/nvme0n1p2"));
    assert_eq!(
        candidate.root_filesystem.fstype,
        cachyos_boot_rescue::discovery::types::FilesystemType::Btrfs
    );
    assert!(!candidate.boot_partitions.is_empty());
}

#[test]
fn test_full_discovery_multiple_esps() {
    let devices = parse_lsblk_json(&lsblk_output("multiple_esps.json")).unwrap();

    let esps = identify_esp_candidates(&devices);
    // Deve encontrar 4 ESPs: sda1, sda3, sdb1 (sda2 e sda4 não são ESP, sdb2 não é ESP)
    assert_eq!(esps.len(), 3);

    // Verifica que todas são do tipo EFI
    for esp in &esps {
        assert_eq!(
            esp.device.partition_type,
            Some(cachyos_boot_rescue::discovery::types::PartitionType::EfiSystem)
        );
        assert_eq!(
            esp.device.filesystem,
            Some(cachyos_boot_rescue::discovery::types::FilesystemType::Vfat)
        );
    }

    // Ordenação por confiança: sda1 (montada em /boot/efi) > sda3 (label EFI_WINDOWS) > sdb1 (não montada)
    assert!(esps[0].confidence >= esps[1].confidence);
    assert!(esps[1].confidence >= esps[2].confidence);
}

#[test]
fn test_full_discovery_removable_disks() {
    let devices = parse_lsblk_json(&lsblk_output("removable_disks.json")).unwrap();

    let esps = identify_esp_candidates(&devices);
    // sda1 (interno), sdb1 (USB), sdc1 (SD card - MBR tipo 0x0c, mas label "RPI_BOOT" + FAT + tamanho)
    assert_eq!(esps.len(), 3);

    // USB e SD card são removíveis
    let removable_esps: Vec<_> = esps
        .iter()
        .filter(|e| e.device.removable == Some(true))
        .collect();
    assert_eq!(removable_esps.len(), 2); // sdb1 (USB) e sdc1 (SD card)
}

#[test]
fn test_full_discovery_incomplete_data() {
    let devices = parse_lsblk_json(&lsblk_output("incomplete_data.json")).unwrap();

    let esps = identify_esp_candidates(&devices);
    // sda1 tem tipo EFI mas sem filesystem - ainda deve ser candidato com confiança baixa
    assert_eq!(esps.len(), 1);
    assert!(esps[0].confidence > 0);
    assert!(esps[0].confidence < 50); // Apenas tipo de partição

    let linux_candidates = identify_linux_installations(&devices, &esps);
    // sda2 tem ext4 e está montada em /
    assert_eq!(linux_candidates.len(), 1);
    assert_eq!(
        linux_candidates[0].root_device.path,
        PathBuf::from("/dev/sda2")
    );
}

#[test]
fn test_full_discovery_luks_encrypted() {
    let devices = parse_lsblk_json(&lsblk_output("luks_encrypted.json")).unwrap();

    let esps = identify_esp_candidates(&devices);
    assert_eq!(esps.len(), 1);
    assert_eq!(esps[0].device.path, PathBuf::from("/dev/nvme0n1p1"));

    let linux_candidates = identify_linux_installations(&devices, &esps);
    // dm-0 é o dispositivo descriptografado montado em /
    assert_eq!(linux_candidates.len(), 1);
    assert_eq!(
        linux_candidates[0].root_device.path,
        PathBuf::from("/dev/mapper/luks_root")
    );
    assert_eq!(
        linux_candidates[0].root_filesystem.fstype,
        cachyos_boot_rescue::discovery::types::FilesystemType::Btrfs
    );
}

#[test]
fn test_blkid_enrichment() {
    let mut devices = parse_lsblk_json(&lsblk_output("sata_single_disk.json")).unwrap();

    // Simula dados do blkid
    let mut blkid_data = HashMap::new();
    blkid_data.insert(
        "/dev/sda1".to_string(),
        cachyos_boot_rescue::discovery::types::FilesystemInfo {
            fstype: cachyos_boot_rescue::discovery::types::FilesystemType::Vfat,
            uuid: Some("BLKID-UUID-1".to_string()),
            label: Some("BLKID-LABEL".to_string()),
            partuuid: Some("blkid-partuuid".to_string()),
            version: None,
            subsystems_uuid: None,
            subsystems_type: None,
        },
    );

    enrich_with_blkid(&mut devices, &blkid_data);

    // Verifica que enriqueceu a partição
    let sda = &devices[0];
    let sda1 = &sda.children[0];
    // lsblk já tinha UUID, então não deve sobrescrever
    assert_eq!(sda1.uuid, Some("ABCD-1234".to_string()));
    assert_eq!(sda1.label, Some("EFI".to_string()));
}

#[test]
fn test_blkid_enrichment_missing_in_lsblk() {
    let mut devices = parse_lsblk_json(&lsblk_output("incomplete_data.json")).unwrap();

    let mut blkid_data = HashMap::new();
    blkid_data.insert(
        "/dev/sda1".to_string(),
        cachyos_boot_rescue::discovery::types::FilesystemInfo {
            fstype: cachyos_boot_rescue::discovery::types::FilesystemType::Vfat,
            uuid: Some("BLKID-UUID".to_string()),
            label: Some("BLKID-LABEL".to_string()),
            partuuid: Some("blkid-partuuid".to_string()),
            version: None,
            subsystems_uuid: None,
            subsystems_type: None,
        },
    );

    enrich_with_blkid(&mut devices, &blkid_data);

    let sda = &devices[0];
    let sda1 = &sda.children[0];
    // lsblk não tinha UUID/label, deve usar blkid
    assert_eq!(sda1.uuid, Some("BLKID-UUID".to_string()));
    assert_eq!(sda1.label, Some("BLKID-LABEL".to_string()));
    assert_eq!(
        sda1.filesystem,
        Some(cachyos_boot_rescue::discovery::types::FilesystemType::Vfat)
    );
}

#[test]
fn test_discovery_report_structure() {
    let devices = parse_lsblk_json(&lsblk_output("sata_single_disk.json")).unwrap();
    let esps = identify_esp_candidates(&devices);
    let linux = identify_linux_installations(&devices, &esps);

    let report = DiscoveryReport {
        timestamp: chrono::Utc::now(),
        session_id: "test-session".to_string(),
        uefi_state: cachyos_boot_rescue::discovery::types::UefiState::Uefi,
        devices,
        esp_candidates: esps,
        linux_candidates: linux,
        diagnostics: vec![],
        lsblk_version: Some("2.38".to_string()),
        blkid_version: None,
    };

    // Verifica serialização
    let json = serde_json::to_string_pretty(&report).unwrap();
    assert!(json.contains("test-session"));
    assert!(json.contains("uefi")); // lowercase due to serde rename_all
    assert!(json.contains("sda1"));
    assert!(json.contains("sda2"));
}
