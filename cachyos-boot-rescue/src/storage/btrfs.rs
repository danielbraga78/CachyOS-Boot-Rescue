//! Detecção e utilitários para Btrfs subvolumes.

use crate::discovery::types::{BlockDevice, FilesystemType};
use crate::error::{Error, Result};
use std::path::Path;
use std::process::Command;

/// Informações de subvolume Btrfs.
#[derive(Debug, Clone, PartialEq)]
pub struct BtrfsSubvolume {
    /// ID do subvolume.
    pub id: u64,
    /// ID do subvolume pai (0 para raiz).
    pub parent_id: u64,
    /// Caminho relativo ao topo do filesystem.
    pub path: String,
    /// Nome do subvolume.
    pub name: String,
}

/// Lista subvolumes Btrfs de um dispositivo montado.
pub fn list_btrfs_subvolumes(mount_point: &Path) -> Result<Vec<BtrfsSubvolume>> {
    let output = Command::new("btrfs")
        .args([
            "subvolume",
            "list",
            "-p",
            mount_point.to_str().unwrap_or(""),
        ])
        .output()
        .map_err(|e| Error::internal_with_source("Falha ao executar btrfs subvolume list", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::internal(format!(
            "btrfs subvolume list falhou: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut subvols = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("ID") {
            continue; // Header
        }

        // Formato: ID <id> gen <gen> top level <parent_id> path <path>
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 7 {
            continue;
        }

        let id = parts[1].parse::<u64>().unwrap_or(0);
        let parent_id = parts[5].parse::<u64>().unwrap_or(0);
        let path = parts[7..].join(" ");

        // Extrai nome do último componente do path
        let name = path.split('/').next_back().unwrap_or(&path).to_string();

        subvols.push(BtrfsSubvolume {
            id,
            parent_id,
            path,
            name,
        });
    }

    Ok(subvols)
}

/// Detecta subvolume raiz a partir de opções de montagem atuais.
pub fn detect_mounted_subvolume(mount_point: &Path) -> Option<String> {
    // Tenta ler /proc/self/mountinfo
    let content = std::fs::read_to_string("/proc/self/mountinfo").ok()?;
    let mount_str = mount_point.to_string_lossy();

    for line in content.lines() {
        if line.contains(&*mount_str) {
            // Procura por subvol= ou subvolid=
            if let Some(idx) = line.find("subvol=") {
                let subvol = &line[idx + 7..];
                let subvol = subvol.split_whitespace().next().unwrap_or("");
                if !subvol.is_empty() && subvol != "/" {
                    return Some(subvol.to_string());
                }
            }
            if let Some(idx) = line.find("subvolid=") {
                let subvol = &line[idx + 9..];
                let subvol = subvol.split_whitespace().next().unwrap_or("");
                if !subvol.is_empty() && subvol != "0" && subvol != "5" {
                    return Some(format!("id={}", subvol));
                }
            }
        }
    }

    None
}

/// Encontra subvolume raiz padrão (@ ou @root) em um filesystem Btrfs montado.
pub fn find_default_root_subvolume(mount_point: &Path) -> Option<String> {
    let subvols = list_btrfs_subvolumes(mount_point).ok()?;

    // Procura por @, @root, @_, ou subvolume com parent_id = 5 (top level)
    for subvol in &subvols {
        if subvol.name == "@" || subvol.name == "@root" || subvol.name == "@_" {
            return Some(subvol.name.clone());
        }
    }

    // Fallback: primeiro subvolume com parent_id = 5 (nível superior)
    for subvol in &subvols {
        if subvol.parent_id == 5 && subvol.id != 5 {
            return Some(subvol.name.clone());
        }
    }

    None
}

/// Verifica se um dispositivo é Btrfs e tem subvolumes.
pub fn has_btrfs_subvolumes(device: &BlockDevice) -> bool {
    device.filesystem == Some(FilesystemType::Btrfs)
}

/// Obtém caminho do subvolume para uso em opções de montagem.
pub fn get_subvol_mount_option(subvolume: &str) -> String {
    if let Some(stripped) = subvolume.strip_prefix("id=") {
        format!("subvolid={}", stripped)
    } else {
        format!("subvol={}", subvolume)
    }
}

/// Verifica se um caminho é um ponto de montagem Btrfs válido.
pub fn is_btrfs_mount(mount_point: &Path) -> bool {
    // Verifica via findmnt ou /proc/mounts
    let output = Command::new("findmnt")
        .args(["-n", "-o", "FSTYPE", mount_point.to_str().unwrap_or("")])
        .output()
        .ok();

    if let Some(out) = output
        && out.status.success()
    {
        let fs_type = String::from_utf8_lossy(&out.stdout).trim().to_string();
        return fs_type == "btrfs";
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::types::{
        BlockDevice, DeviceId, DeviceType, FilesystemType, PartitionType,
    };
    use std::path::PathBuf;

    fn create_btrfs_device() -> BlockDevice {
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
            uuid: Some("btrfs-uuid".to_string()),
            partuuid: Some("btrfs-partuuid".to_string()),
            label: Some("root".to_string()),
            partition_type: Some(PartitionType::LinuxFilesystem),
            filesystem: Some(FilesystemType::Btrfs),
            mountpoint: Some(PathBuf::from("/")),
            mountpoints: vec![],
            mount_options: vec![],
            parent: Some("sda".to_string()),
            sysfs_path: None,
            children: vec![],
        }
    }

    #[test]
    fn test_has_btrfs_subvolumes() {
        let btrfs_dev = create_btrfs_device();
        assert!(has_btrfs_subvolumes(&btrfs_dev));

        let mut ext4_dev = btrfs_dev.clone();
        ext4_dev.filesystem = Some(FilesystemType::Ext4);
        assert!(!has_btrfs_subvolumes(&ext4_dev));
    }

    #[test]
    fn test_get_subvol_mount_option() {
        assert_eq!(get_subvol_mount_option("@"), "subvol=@");
        assert_eq!(get_subvol_mount_option("@root"), "subvol=@root");
        assert_eq!(get_subvol_mount_option("id=256"), "subvolid=256");
    }

    #[test]
    fn test_btrfs_subvolume_struct() {
        let subvol = BtrfsSubvolume {
            id: 256,
            parent_id: 5,
            path: "@".to_string(),
            name: "@".to_string(),
        };
        assert_eq!(subvol.id, 256);
        assert_eq!(subvol.parent_id, 5);
        assert_eq!(subvol.name, "@");
    }
}
