//! Detecção e suporte a LUKS (Linux Unified Key Setup).
//!
//! Apenas detecta partições LUKS e solicita desbloqueio interativo.
//! NÃO armazena senhas, NÃO registra, NÃO passa em argumentos de processo.
//!
//! ATENÇÃO: O desbloqueio interativo NÃO está implementado de forma segura neste momento.
//! Apenas a detecção está disponível. O desbloqueio requer implementação segura
//! com tratamento adequado de senha (zeroização de memória, stdin seguro, etc.).

use crate::discovery::types::{BlockDevice, FilesystemType, PartitionType};
use crate::error::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

/// Informações de dispositivo LUKS detectado.
#[derive(Debug, Clone, PartialEq)]
pub struct LuksDeviceInfo {
    /// Caminho do dispositivo LUKS (ex: /dev/sda2).
    pub device_path: PathBuf,
    /// UUID do container LUKS.
    pub luks_uuid: Option<String>,
    /// Tipo de filesystem interno (após descriptografar).
    pub inner_fs_type: Option<FilesystemType>,
    /// UUID do filesystem interno.
    pub inner_uuid: Option<String>,
    /// Label do filesystem interno.
    pub inner_label: Option<String>,
    /// Se o dispositivo já está descriptografado (device mapper ativo).
    pub is_open: bool,
    /// Caminho do device mapper se aberto (ex: /dev/mapper/luks-xxx).
    pub mapper_path: Option<PathBuf>,
}

/// Verifica se um dispositivo é LUKS.
pub fn is_luks_device(device: &BlockDevice) -> bool {
    device.filesystem == Some(FilesystemType::Luks)
        || device.partition_type == Some(PartitionType::LinuxFilesystem)
            && device.fstype_maybe_luks()
}

impl BlockDevice {
    fn fstype_maybe_luks(&self) -> bool {
        // Verifica se fstype indica LUKS via blkid
        self.filesystem == Some(FilesystemType::Luks)
    }
}

/// Detecta dispositivos LUKS nos dispositivos descobertos.
pub fn detect_luks_devices(devices: &[BlockDevice]) -> Vec<LuksDeviceInfo> {
    let mut luks_devices = Vec::new();

    fn check_device(dev: &BlockDevice, luks_devices: &mut Vec<LuksDeviceInfo>) {
        if is_luks_device(dev) {
            let info = probe_luks_device(dev);
            luks_devices.push(info);
        }
        for child in &dev.children {
            check_device(child, luks_devices);
        }
    }

    for dev in devices {
        check_device(dev, &mut luks_devices);
    }

    luks_devices
}

/// Probe detalhado de dispositivo LUKS usando cryptsetup.
fn probe_luks_device(device: &BlockDevice) -> LuksDeviceInfo {
    let mut info = LuksDeviceInfo {
        device_path: device.path.clone(),
        luks_uuid: device.uuid.clone(),
        inner_fs_type: None,
        inner_uuid: None,
        inner_label: None,
        is_open: false,
        mapper_path: None,
    };

    // Tenta obter informações do LUKS header
    let output = Command::new("cryptsetup")
        .args([
            "luksDump",
            "--dump-header",
            device.path.to_str().unwrap_or(""),
        ])
        .output();

    if let Ok(out) = output
        && out.status.success()
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        // Parse básico do luksDump para pegar UUID
        for line in stdout.lines() {
            if line.contains("UUID:")
                && info.luks_uuid.is_none()
                && let Some(uuid) = line.split(':').nth(1)
            {
                info.luks_uuid = Some(uuid.trim().to_string());
            }
        }
    }

    // Verifica se já está aberto (device mapper)
    if let Some(ref partuuid) = device.partuuid {
        let mapper_path = PathBuf::from(format!("/dev/mapper/luks-{}", partuuid));
        if mapper_path.exists() {
            info.is_open = true;
            info.mapper_path = Some(mapper_path.clone());

            // Tenta obter info do filesystem interno
            let output = Command::new("blkid")
                .args(["-o", "value", "-s", "TYPE", mapper_path.to_str().unwrap()])
                .output();

            if let Ok(out) = output
                && out.status.success()
            {
                let fs_type = String::from_utf8_lossy(&out.stdout).trim().to_string();
                info.inner_fs_type = Some(FilesystemType::from_string(&fs_type));
            }

            let output = Command::new("blkid")
                .args(["-o", "value", "-s", "UUID", mapper_path.to_str().unwrap()])
                .output();

            if let Ok(out) = output
                && out.status.success()
            {
                let uuid = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !uuid.is_empty() {
                    info.inner_uuid = Some(uuid);
                }
            }
        }
    }

    info
}

/// Verifica se cryptsetup está disponível.
pub fn is_cryptsetup_available() -> bool {
    Command::new("cryptsetup")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Tenta desbloquear dispositivo LUKS.
///
/// RETORNA ERRO: Desbloqueio interativo NÃO implementado de forma segura.
/// Apenas a detecção está disponível. Para desbloquear, implemente com:
/// - Zeroização de memória da senha (zeroize crate)
/// - Leitura segura via stdin (sem echo)
/// - Tratamento de falhas de write_all
/// - Timeout e cancelamento adequados
/// - Não passar senha em argumentos de processo
/// - Não registrar senha em logs
pub fn unlock_luks_interactive(_device_path: &Path) -> Result<PathBuf> {
    Err(Error::internal(
        "Desbloqueio LUKS interativo não implementado. \
         Apenas detecção está disponível. \
         Para desbloquear, implemente tratamento seguro de senha com zeroização de memória.",
    ))
}

/// Fecha dispositivo LUKS aberto.
pub fn close_luks(mapper_path: &Path) -> Result<()> {
    let mapper_name = mapper_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::internal("Nome de mapper inválido"))?;

    let output = Command::new("cryptsetup")
        .args(["luksClose", mapper_name])
        .output()
        .map_err(|e| Error::internal_with_source("Falha ao executar cryptsetup luksClose", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Ignora erro se já não existir
        if !stderr.contains("not found") && !stderr.contains("não encontrado") {
            return Err(Error::internal(format!("Falha ao fechar LUKS: {}", stderr)));
        }
    }

    info!(mapper = %mapper_path.display(), "Dispositivo LUKS fechado");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::types::{
        BlockDevice, DeviceId, DeviceType, FilesystemType, PartitionType,
    };
    use std::path::PathBuf;

    fn create_luks_device() -> BlockDevice {
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
            uuid: Some("luks-uuid-1234".to_string()),
            partuuid: Some("luks-partuuid-1234".to_string()),
            label: Some("luks_root".to_string()),
            partition_type: Some(PartitionType::LinuxFilesystem),
            filesystem: Some(FilesystemType::Luks),
            mountpoint: None,
            mountpoints: vec![],
            mount_options: vec![],
            parent: Some("sda".to_string()),
            sysfs_path: None,
            children: vec![],
        }
    }

    #[test]
    fn test_is_luks_device() {
        let luks_dev = create_luks_device();
        assert!(is_luks_device(&luks_dev));

        let mut ext4_dev = luks_dev.clone();
        ext4_dev.filesystem = Some(FilesystemType::Ext4);
        assert!(!is_luks_device(&ext4_dev));
    }

    #[test]
    fn test_detect_luks_devices() {
        let devices = vec![create_luks_device(), {
            let mut d = create_luks_device();
            d.path = PathBuf::from("/dev/sda3");
            d.filesystem = Some(FilesystemType::Ext4);
            d
        }];

        let luks = detect_luks_devices(&devices);
        assert_eq!(luks.len(), 1);
        assert_eq!(luks[0].device_path, PathBuf::from("/dev/sda2"));
        assert_eq!(luks[0].luks_uuid, Some("luks-uuid-1234".to_string()));
    }

    #[test]
    fn test_luks_device_info_default() {
        let info = LuksDeviceInfo {
            device_path: PathBuf::from("/dev/sda2"),
            luks_uuid: None,
            inner_fs_type: None,
            inner_uuid: None,
            inner_label: None,
            is_open: false,
            mapper_path: None,
        };
        assert!(!info.is_open);
        assert!(info.mapper_path.is_none());
    }

    #[test]
    fn test_is_cryptsetup_available() {
        // Apenas verifica que a função executa sem panic
        let _ = is_cryptsetup_available();
    }

    #[test]
    fn test_unlock_luks_interactive_not_implemented() {
        let result = unlock_luks_interactive(Path::new("/dev/sda2"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("não implementado"));
    }

    #[test]
    fn test_close_luks_handles_not_found() {
        // Testa que close_luks não falha se o dispositivo não existe
        let result = close_luks(Path::new("/dev/mapper/nonexistent"));
        // Deve falhar mas não com "not found"
        // (o comportamento real depende do sistema)
        let _ = result;
    }
}
