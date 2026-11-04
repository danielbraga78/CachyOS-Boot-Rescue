//! Modelos tipados para descoberta de hardware e instalações Linux.
//!
//! Define as estruturas de dados que representam o inventário somente leitura
//! de dispositivos, partições, filesystems, ESPs e instalações candidatas.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Identificador único de dispositivo de bloco.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct DeviceId(pub u64); // major:minor codificado

impl DeviceId {
    /// Cria um DeviceId a partir de major e minor.
    pub fn from_major_minor(major: u32, minor: u32) -> Self {
        Self(((major as u64) << 32) | (minor as u64))
    }

    /// Retorna o número major.
    pub fn major(&self) -> u32 {
        (self.0 >> 32) as u32
    }

    /// Retorna o número minor.
    pub fn minor(&self) -> u32 {
        self.0 as u32
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.major(), self.minor())
    }
}

/// Tipo de dispositivo de bloco.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    /// Disco físico (SATA, NVMe, etc).
    Disk,
    /// Partição.
    Partition,
    /// Loop device.
    Loop,
    /// Dispositivo LVM.
    Lvm,
    /// RAID.
    Raid,
    /// Criptografado (LUKS).
    Crypto,
    /// Desconhecido.
    Unknown,
}

/// Tipo de tabela de partições.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PartitionTableType {
    /// GPT (GUID Partition Table).
    Gpt,
    /// MBR (MSDOS).
    Mbr,
    /// Desconhecido ou não detectado.
    Unknown,
}

/// Tipo de partição (código GPT ou MBR).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartitionType {
    /// EFI System Partition (GPT: C12A7328-F81F-11D2-BA4B-00A0C93EC93B).
    EfiSystem,
    /// Linux filesystem (GPT: 0FC63DAF-8483-4772-8E79-3D69D8477DE4).
    LinuxFilesystem,
    /// Linux swap (GPT: 0657FD6D-A4AB-43C4-84E5-0933C84B4F4F).
    LinuxSwap,
    /// Linux RAID (GPT: A19D880F-05FC-4D3B-A006-743F0F84911E).
    LinuxRaid,
    /// Linux LVM (GPT: E6D6D379-F507-44C2-A23C-238F2A3DF928).
    LinuxLvm,
    /// Microsoft basic data (GPT: EBD0A0A2-B9E5-4433-87C0-68B6B72699C7).
    MicrosoftBasicData,
    /// Windows recovery (GPT: DE94BBA4-06D1-4D40-A16A-BFD50179D6AC).
    WindowsRecovery,
    /// Microsoft reserved (GPT: E3C9E316-0B5C-4DB8-817D-F92DF00215AE).
    MicrosoftReserved,
    /// BIOS boot partition (GPT: 21686148-6449-6E6F-744E-656564454649).
    BiosBoot,
    /// Outro tipo GPT conhecido.
    OtherGpt(String),
    /// Tipo MBR (valor hexadecimal).
    Mbr(u8),
    /// Desconhecido.
    Unknown,
}

/// Tipo de filesystem detectado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilesystemType {
    /// Btrfs.
    Btrfs,
    /// ext4.
    Ext4,
    /// ext3.
    Ext3,
    /// ext2.
    Ext2,
    /// XFS.
    Xfs,
    /// FAT32 / VFAT.
    Fat32,
    /// FAT16.
    Fat16,
    /// FAT12.
    Fat12,
    /// VFAT (genérico).
    Vfat,
    /// NTFS.
    Ntfs,
    /// Swap.
    Swap,
    /// LUKS (criptografado).
    Luks,
    /// LVM2 PV.
    Lvm2,
    /// ISO9660.
    Iso9660,
    /// UDF.
    Udf,
    /// Desconhecido.
    Unknown,
}

/// Estado do firmware UEFI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UefiState {
    /// Sistema iniciou em modo UEFI.
    Uefi,
    /// Sistema iniciou em modo BIOS/Legacy.
    BiosLegacy,
    /// Não foi possível determinar.
    Unknown,
}

/// Dispositivo de bloco (disco ou partição).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockDevice {
    /// Caminho do dispositivo (ex: /dev/sda, /dev/nvme0n1p1).
    pub path: PathBuf,
    /// Nome do dispositivo (ex: sda, nvme0n1p1).
    pub name: String,
    /// ID do dispositivo (major:minor). None se não disponível.
    pub device_id: Option<DeviceId>,
    /// Tipo de dispositivo.
    pub device_type: DeviceType,
    /// Tamanho em bytes. None se não disponível.
    pub size: Option<u64>,
    /// Tamanho físico do setor em bytes. None se não disponível.
    pub sector_size: Option<u32>,
    /// Tamanho lógico do setor em bytes. None se não disponível.
    pub logical_sector_size: Option<u32>,
    /// Modelo do dispositivo (para discos).
    pub model: Option<String>,
    /// Serial do dispositivo (para discos). Não serializado em relatórios para privacidade.
    #[serde(skip_serializing)]
    pub serial: Option<String>,
    /// Vendor do dispositivo.
    pub vendor: Option<String>,
    /// Rotação (para HDDs: rpm, para SSDs: 0 ou None).
    pub rotation_rate: Option<u32>,
    /// Se é removível. None se não disponível/desconhecido.
    pub removable: Option<bool>,
    /// Se é somente leitura. None se não disponível/desconhecido.
    pub read_only: Option<bool>,
    /// Tipo de tabela de partições (apenas para discos).
    pub partition_table: Option<PartitionTableType>,
    /// UUID do dispositivo (para partições).
    pub uuid: Option<String>,
    /// PARTUUID da partição.
    pub partuuid: Option<String>,
    /// Label do filesystem.
    pub label: Option<String>,
    /// Tipo de partição.
    pub partition_type: Option<PartitionType>,
    /// Tipo de filesystem.
    pub filesystem: Option<FilesystemType>,
    /// Ponto de montagem atual (se montado).
    pub mountpoint: Option<PathBuf>,
    /// Pontos de montagem alternativos (lsblk MOUNTPOINTS).
    #[serde(default)]
    pub mountpoints: Vec<PathBuf>,
    /// Opções de montagem (se montado).
    pub mount_options: Vec<String>,
    /// Dispositivo pai (para partições).
    pub parent: Option<String>,
    /// Caminho no sysfs.
    pub sysfs_path: Option<PathBuf>,
    /// Dispositivos filhos (partições para discos).
    #[serde(default)]
    pub children: Vec<BlockDevice>,
}

/// Informações de filesystem complementares do blkid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilesystemInfo {
    /// Tipo de filesystem.
    pub fstype: FilesystemType,
    /// UUID do filesystem.
    pub uuid: Option<String>,
    /// Label do filesystem.
    pub label: Option<String>,
    /// PARTUUID (se partição).
    pub partuuid: Option<String>,
    /// Versão do filesystem (ex: 1.0 para ext4).
    pub version: Option<String>,
    /// UUID do subsistema (ex: LUKS).
    pub subsystems_uuid: Option<String>,
    /// Tipo do subsistema (ex: LUKS).
    pub subsystems_type: Option<String>,
}

/// Candidato a EFI System Partition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EspCandidate {
    /// Dispositivo da partição.
    pub device: BlockDevice,
    /// Informações de filesystem.
    pub filesystem: FilesystemInfo,
    /// Ponto de montagem atual (se montado).
    pub mountpoint: Option<PathBuf>,
    /// Se está montada em /boot/efi ou /efi.
    pub is_boot_efi_mount: bool,
    /// Se está montada em /boot.
    pub is_boot_mount: bool,
    /// Tamanho da partição em bytes.
    pub size: u64,
    /// Espaço livre estimado em bytes (se montado e disponível).
    pub free_space: Option<u64>,
    /// Nível de confiança (0-100).
    pub confidence: u8,
    /// Motivos da classificação.
    pub reasons: Vec<String>,
}

/// Candidato a instalação Linux.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinuxInstallationCandidate {
    /// Dispositivo raiz (partição /).
    pub root_device: BlockDevice,
    /// Informações de filesystem da raiz.
    pub root_filesystem: FilesystemInfo,
    /// Partições de boot associadas (/boot, /boot/efi, /efi).
    pub boot_partitions: Vec<EspCandidate>,
    /// Partições adicionais detectadas (home, var, etc).
    pub additional_partitions: Vec<BlockDevice>,
    /// Distribuição detectada (via /etc/os-release).
    pub distribution: Option<String>,
    /// Versão da distribuição.
    pub version: Option<String>,
    /// ID da distribuição (ex: arch, cachyos, fedora).
    pub distribution_id: Option<String>,
    /// Caminho do kernel detectado (se /boot montado).
    pub kernel_paths: Vec<PathBuf>,
    /// Caminho do initramfs detectado (se /boot montado).
    pub initramfs_paths: Vec<PathBuf>,
    /// Se /etc/fstab está acessível.
    pub has_fstab: bool,
    /// Conteúdo do fstab (se acessível).
    /// Não serializado em relatórios para evitar exposição de informações sensíveis
    /// (UUIDs, caminhos de montagem, opções de criptografia, etc.).
    /// O parser de fstab (Fase 4) deve acessar este campo diretamente quando necessário.
    #[serde(skip_serializing)]
    pub fstab_content: Option<String>,
    /// Se /etc/os-release está acessível.
    pub has_os_release: bool,
    /// Se está em Btrfs com subvolumes.
    pub is_btrfs_subvolume: bool,
    /// Subvolume raiz (se Btrfs).
    pub btrfs_subvolume: Option<String>,
    /// Nível de confiança (0-100).
    pub confidence: u8,
    /// Motivos da classificação.
    pub reasons: Vec<String>,
}

/// Relatório completo de descoberta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryReport {
    /// Timestamp da descoberta.
    pub timestamp: DateTime<Utc>,
    /// ID da sessão de logging.
    pub session_id: String,
    /// Estado do firmware UEFI.
    pub uefi_state: UefiState,
    /// Todos os dispositivos de bloco detectados.
    pub devices: Vec<BlockDevice>,
    /// Candidatos a ESP.
    pub esp_candidates: Vec<EspCandidate>,
    /// Candidatos a instalação Linux.
    pub linux_candidates: Vec<LinuxInstallationCandidate>,
    /// Erros ou avisos durante a descoberta.
    pub diagnostics: Vec<DiscoveryDiagnostic>,
    /// Versão do lsblk usado.
    pub lsblk_version: Option<String>,
    /// Versão do blkid usado.
    pub blkid_version: Option<String>,
}

/// Diagnóstico de descoberta (erro, aviso, info).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryDiagnostic {
    /// Severidade.
    pub level: DiagnosticLevel,
    /// Componente que gerou o diagnóstico.
    pub component: String,
    /// Mensagem.
    pub message: String,
    /// Detalhes adicionais.
    pub details: Option<String>,
}

/// Nível de severidade do diagnóstico.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticLevel {
    /// Erro que impede descoberta completa.
    Error,
    /// Aviso que pode afetar resultados.
    Warning,
    /// Informação adicional.
    Info,
}

/// Resultado da execução de comando externo.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandOutput {
    /// Código de saída.
    pub exit_code: i32,
    /// Saída padrão.
    pub stdout: String,
    /// Saída de erro.
    pub stderr: String,
    /// Se houve timeout.
    pub timed_out: bool,
}

/// Configuração para execução de comandos.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandConfig {
    /// Timeout em segundos.
    pub timeout_secs: u64,
    /// Limite máximo de saída (bytes).
    pub max_output_bytes: usize,
    /// Argumentos adicionais de ambiente.
    pub env_vars: HashMap<String, String>,
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_output_bytes: 1024 * 1024, // 1 MB
            env_vars: HashMap::new(),
        }
    }
}

impl FilesystemType {
    /// Converte string do lsblk/blkid para FilesystemType.
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "btrfs" => Self::Btrfs,
            "ext4" => Self::Ext4,
            "ext3" => Self::Ext3,
            "ext2" => Self::Ext2,
            "xfs" => Self::Xfs,
            "vfat" | "fat32" => Self::Vfat,
            "fat16" => Self::Fat16,
            "fat12" => Self::Fat12,
            "ntfs" => Self::Ntfs,
            "swap" => Self::Swap,
            "crypto_luks" | "luks" => Self::Luks,
            "lvm2_member" | "lvm2" => Self::Lvm2,
            "iso9660" => Self::Iso9660,
            "udf" => Self::Udf,
            _ => Self::Unknown,
        }
    }

    /// Retorna se é um filesystem Linux nativo.
    pub fn is_linux_native(&self) -> bool {
        matches!(
            self,
            Self::Btrfs | Self::Ext4 | Self::Ext3 | Self::Ext2 | Self::Xfs
        )
    }

    /// Retorna se é FAT/VFAT.
    pub fn is_fat(&self) -> bool {
        matches!(self, Self::Vfat | Self::Fat32 | Self::Fat16 | Self::Fat12)
    }

    /// Retorna se é um tipo de criptografia.
    pub fn is_crypto(&self) -> bool {
        matches!(self, Self::Luks)
    }
}

impl PartitionType {
    /// Converte string do lsblk para PartitionType.
    pub fn from_string(s: &str) -> Self {
        let lower = s.to_lowercase();
        // GPT type GUIDs
        match lower.as_str() {
            "c12a7328-f81f-11d2-ba4b-00a0c93ec93b" => Self::EfiSystem,
            "0fc63daf-8483-4772-8e79-3d69d8477de4" => Self::LinuxFilesystem,
            "0657fd6d-a4ab-43c4-84e5-0933c84b4f4f" => Self::LinuxSwap,
            "a19d880f-05fc-4d3b-a006-743f0f84911e" => Self::LinuxRaid,
            "e6d6d379-f507-44c2-a23c-238f2a3df928" => Self::LinuxLvm,
            "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7" => Self::MicrosoftBasicData,
            "de94bba4-06d1-4d40-a16a-bfd50179d6ac" => Self::WindowsRecovery,
            "e3c9e316-0b5c-4db8-817d-f92df00215ae" => Self::MicrosoftReserved,
            "21686148-6449-6e6f-744e-656564454649" => Self::BiosBoot,
            _ if lower.starts_with("0x") => {
                // MBR type hex
                if let Ok(val) = u8::from_str_radix(&lower[2..], 16) {
                    Self::Mbr(val)
                } else {
                    Self::Unknown
                }
            }
            _ => Self::OtherGpt(s.to_string()),
        }
    }

    /// Retorna se é ESP (EFI System Partition).
    pub fn is_esp(&self) -> bool {
        matches!(self, Self::EfiSystem)
    }

    /// Retorna se é partição Linux (filesystem, RAID ou LVM).
    /// Swap NÃO é considerado partição de instalação Linux candidata.
    pub fn is_linux_installation(&self) -> bool {
        matches!(
            self,
            Self::LinuxFilesystem | Self::LinuxRaid | Self::LinuxLvm
        )
    }

    /// Retorna se é qualquer tipo de partição Linux (incluindo swap).
    pub fn is_linux_any(&self) -> bool {
        matches!(
            self,
            Self::LinuxFilesystem | Self::LinuxSwap | Self::LinuxRaid | Self::LinuxLvm
        )
    }
}

impl DeviceType {
    /// Converte string do lsblk para DeviceType.
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "disk" => Self::Disk,
            "part" => Self::Partition,
            "loop" => Self::Loop,
            "lvm" => Self::Lvm,
            "raid" => Self::Raid,
            "crypt" => Self::Crypto,
            _ => Self::Unknown,
        }
    }
}

impl PartitionTableType {
    /// Converte string do lsblk para PartitionTableType.
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "gpt" => Self::Gpt,
            "dos" | "mbr" => Self::Mbr,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::types::{
        BlockDevice, DeviceId, DeviceType, FilesystemInfo, FilesystemType,
        LinuxInstallationCandidate, PartitionType,
    };
    use std::path::PathBuf;

    #[test]
    fn test_linux_installation_candidate_fstab_content_not_serialized() {
        // Testa que fstab_content não é serializado em relatórios
        let candidate = LinuxInstallationCandidate {
            root_device: BlockDevice {
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
            root_filesystem: FilesystemInfo {
                fstype: FilesystemType::Ext4,
                uuid: Some("root-uuid".to_string()),
                label: Some("root".to_string()),
                partuuid: Some("root-partuuid".to_string()),
                version: None,
                subsystems_uuid: None,
                subsystems_type: None,
            },
            boot_partitions: vec![],
            additional_partitions: vec![],
            distribution: Some("CachyOS".to_string()),
            version: Some("2026.11".to_string()),
            distribution_id: Some("cachyos".to_string()),
            kernel_paths: vec![],
            initramfs_paths: vec![],
            has_fstab: true,
            fstab_content: Some("UUID=secret-luks-uuid / ext4 defaults 0 1\nUUID=esp-uuid /boot/efi vfat defaults 0 2\n".to_string()),
            has_os_release: true,
            is_btrfs_subvolume: false,
            btrfs_subvolume: None,
            confidence: 80,
            reasons: vec!["test".to_string()],
        };

        let json = serde_json::to_string(&candidate).unwrap();

        // Verifica que campos sensíveis NÃO estão no JSON serializado
        assert!(!json.contains("fstab_content"));
        assert!(!json.contains("secret-luks-uuid"));
        assert!(!json.contains("esp-uuid"));

        // Verifica que outros campos estão presentes
        assert!(json.contains("has_fstab"));
        assert!(json.contains("CachyOS"));
        assert!(json.contains("sda2"));
    }

    #[test]
    fn test_discovery_report_serialization_excludes_fstab() {
        // Testa que o DiscoveryReport completo não vaza fstab_content
        let candidate = LinuxInstallationCandidate {
            root_device: BlockDevice {
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
            root_filesystem: FilesystemInfo {
                fstype: FilesystemType::Ext4,
                uuid: Some("root-uuid".to_string()),
                label: Some("root".to_string()),
                partuuid: Some("root-partuuid".to_string()),
                version: None,
                subsystems_uuid: None,
                subsystems_type: None,
            },
            boot_partitions: vec![],
            additional_partitions: vec![],
            distribution: Some("CachyOS".to_string()),
            version: Some("2026.11".to_string()),
            distribution_id: Some("cachyos".to_string()),
            kernel_paths: vec![],
            initramfs_paths: vec![],
            has_fstab: true,
            fstab_content: Some("UUID=secret-luks / ext4 defaults 0 1\n".to_string()),
            has_os_release: true,
            is_btrfs_subvolume: false,
            btrfs_subvolume: None,
            confidence: 80,
            reasons: vec!["test".to_string()],
        };

        let report = DiscoveryReport {
            timestamp: chrono::Utc::now(),
            session_id: "test-session".to_string(),
            uefi_state: UefiState::Uefi,
            devices: vec![],
            esp_candidates: vec![],
            linux_candidates: vec![candidate],
            diagnostics: vec![],
            lsblk_version: None,
            blkid_version: None,
        };

        let json = serde_json::to_string(&report).unwrap();

        // Verifica que fstab_content não vaza no relatório completo
        assert!(!json.contains("fstab_content"));
        assert!(!json.contains("secret-luks"));
    }
}
