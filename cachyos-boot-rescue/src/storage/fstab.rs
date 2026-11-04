//! Parser puro para /etc/fstab.
//!
//! Suporta UUID, PARTUUID, LABEL, caminhos de dispositivo,
//! opções `nofail`, `noauto`, `subvol` e outras opções de montagem relevantes.

use crate::discovery::types::FilesystemType;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Identificadores de filesystem que podem conter segredos ou informações sensíveis.
/// Estes NÃO devem ser serializados por padrão em logs, relatórios ou JSON.
mod sensitive {
    use super::*;

    /// Wrapper para marcar campos que não devem ser serializados.
    #[expect(dead_code)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Secret<T>(pub T);

    impl<T> Serialize for Secret<T>
    where
        T: Serialize,
    {
        fn serialize<S>(&self, _serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            // Não serializa - omite o campo completamente
            use serde::ser::SerializeStruct;
            let state = _serializer.serialize_struct("Secret", 0)?;
            state.end()
        }
    }

    impl<'de, T> Deserialize<'de> for Secret<T>
    where
        T: Deserialize<'de>,
    {
        fn deserialize<D>(_deserializer: D) -> std::result::Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            // Não pode desserializar - retorna erro
            Err(serde::de::Error::custom(
                "Secret fields cannot be deserialized",
            ))
        }
    }
}

/// Tipo de identificador de dispositivo no fstab.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FstabDeviceSpec {
    /// Caminho direto do dispositivo (ex: /dev/sda1)
    Path(PathBuf),
    /// UUID do filesystem (ex: UUID=xxxx-xxxx)
    Uuid(String),
    /// PARTUUID da partição (ex: PARTUUID=xxxx-xxxx)
    PartUuid(String),
    /// Label do filesystem (ex: LABEL=meu_label)
    Label(String),
}

impl FstabDeviceSpec {
    /// Retorna o UUID se for uma especificação UUID, None caso contrário.
    pub fn uuid(&self) -> Option<&str> {
        match self {
            FstabDeviceSpec::Uuid(u) => Some(u),
            _ => None,
        }
    }
}

/// Opções de montagem parseadas do fstab.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FstabOptions {
    /// Opções brutas como string (para reconstrução) - NÃO serializada (pode conter segredos).
    #[serde(skip_serializing)]
    pub raw: String,
    /// Se a opção `noauto` está presente.
    pub noauto: bool,
    /// Se a opção `nofail` está presente.
    pub nofail: bool,
    /// Se a opção `ro` (read-only) está presente.
    pub read_only: bool,
    /// Se a opção `rw` (read-write) está presente.
    pub read_write: bool,
    /// Subvolume Btrfs (ex: subvol=@ ou subvolid=123).
    pub subvol: Option<String>,
    /// Outras opções não tratadas especificamente.
    pub other: Vec<String>,
}

impl FstabOptions {
    /// Cria FstabOptions a partir de string parseada, sem armazenar raw.
    pub fn from_parsed(
        noauto: bool,
        nofail: bool,
        read_only: bool,
        read_write: bool,
        subvol: Option<String>,
        other: Vec<String>,
    ) -> Self {
        Self {
            raw: String::new(), // Não armazena raw por segurança
            noauto,
            nofail,
            read_only,
            read_write,
            subvol,
            other,
        }
    }

    /// Retorna opções seguras para uso em mount (sem raw).
    pub fn to_mount_options_safe(&self) -> String {
        let mut parts = Vec::new();

        if self.read_only {
            parts.push("ro".to_string());
        } else if self.read_write {
            parts.push("rw".to_string());
        }

        if self.noauto {
            parts.push("noauto".to_string());
        }
        if self.nofail {
            parts.push("nofail".to_string());
        }

        if let Some(ref subvol) = self.subvol {
            if let Some(id) = subvol.strip_prefix("id=") {
                parts.push(format!("subvolid={id}"));
            } else {
                parts.push(format!("subvol={}", subvol));
            }
        }

        parts.extend(self.other.clone());
        parts.join(",")
    }
}

/// Entrada parseada do fstab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FstabEntry {
    /// Especificação do dispositivo (UUID, PARTUUID, LABEL, path)
    pub device_spec: FstabDeviceSpec,
    /// Ponto de montagem.
    pub mount_point: PathBuf,
    /// Tipo de filesystem (ex: ext4, btrfs, vfat, xfs).
    pub fs_type: FilesystemType,
    /// Opções de montagem parseadas.
    pub options: FstabOptions,
    /// Dump (geralmente 0).
    pub dump: u8,
    /// Pass (ordem de fsck, 0=não verificar, 1=raiz primeiro, 2=outros).
    pub pass: u8,
    /// Linha original para debug/auditoria - NÃO serializada (pode conter segredos).
    #[serde(skip_serializing)]
    pub raw_line: String,
    /// Número da linha no arquivo.
    pub line_number: usize,
}

impl FstabEntry {
    /// Retorna uma projeção segura para serialização (sem raw_line).
    pub fn to_safe_projection(&self) -> FstabEntrySafe {
        FstabEntrySafe {
            device_spec: self.device_spec.clone(),
            mount_point: self.mount_point.clone(),
            fs_type: self.fs_type,
            options: self.options.clone(),
            dump: self.dump,
            pass: self.pass,
            line_number: self.line_number,
        }
    }

    /// Converte device_spec para string para exibição/logging (sem segredos).
    pub fn device_spec_to_string(&self) -> String {
        match &self.device_spec {
            FstabDeviceSpec::Path(p) => p.display().to_string(),
            FstabDeviceSpec::Uuid(u) => format!("UUID={}", u),
            FstabDeviceSpec::PartUuid(p) => format!("PARTUUID={}", p),
            FstabDeviceSpec::Label(l) => format!("LABEL={}", l),
        }
    }
}

/// Projeção segura de FstabEntry para serialização (sem raw_line).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FstabEntrySafe {
    pub device_spec: FstabDeviceSpec,
    pub mount_point: PathBuf,
    pub fs_type: FilesystemType,
    pub options: FstabOptions,
    pub dump: u8,
    pub pass: u8,
    pub line_number: usize,
}

/// Resultado da resolução de identificadores do fstab contra dispositivos descobertos.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceResolution {
    /// Especificações resolvidas sem ambiguidade.
    pub resolved: HashMap<FstabDeviceSpec, PathBuf>,
    /// Especificações sem dispositivo correspondente.
    pub missing: Vec<FstabDeviceSpec>,
    /// Especificações que correspondem a mais de um dispositivo distinto.
    pub ambiguous: HashMap<FstabDeviceSpec, Vec<PathBuf>>,
}

impl FstabOptions {
    /// Parseia string de opções (ex: "rw,relatime,subvol=@") - não armazena raw por segurança.
    pub fn parse(options_str: &str) -> Self {
        let mut noauto = false;
        let mut nofail = false;
        let mut read_only = false;
        let mut read_write = false;
        let mut subvol = None;
        let mut other = Vec::new();

        for opt in options_str.split(',') {
            let opt = opt.trim();
            if opt.is_empty() {
                continue;
            }

            match opt {
                "noauto" => noauto = true,
                "nofail" => nofail = true,
                "ro" => read_only = true,
                "rw" => read_write = true,
                _ if opt.starts_with("subvol=") => {
                    subvol = Some(opt["subvol=".len()..].to_string());
                }
                _ if opt.starts_with("subvolid=") => {
                    subvol = Some(format!("id={}", &opt["subvolid=".len()..]));
                }
                _ => other.push(opt.to_string()),
            }
        }

        Self::from_parsed(noauto, nofail, read_only, read_write, subvol, other)
    }

    /// Reconstrói a string de opções para uso em mount(8) - versão segura sem raw.
    pub fn to_mount_options(&self) -> String {
        self.to_mount_options_safe()
    }
}

/// Erros específicos do parser fstab.
#[derive(Debug, thiserror::Error)]
pub enum FstabParseError {
    #[error("Linha {line}: número inválido de campos (esperado 6, encontrado {found})")]
    InvalidFieldCount { line: usize, found: usize },

    #[error("Linha {line}: ponto de montagem inválido '{mount_point}'")]
    InvalidMountPoint { line: usize, mount_point: String },

    #[error("Linha {line}: tipo de filesystem desconhecido '{fs_type}'")]
    UnknownFsType { line: usize, fs_type: String },

    #[error("Linha {line}: valor dump inválido '{dump}'")]
    InvalidDump { line: usize, dump: String },

    #[error("Linha {line}: valor pass inválido '{pass}'")]
    InvalidPass { line: usize, pass: String },

    #[error("Linha {line}: especificação de dispositivo vazia")]
    EmptyDeviceSpec { line: usize },

    #[error("Linha {line}: formato de UUID inválido '{spec}'")]
    InvalidUuidFormat { line: usize, spec: String },
}

impl FstabParseError {
    /// Retorna o número da linha do erro.
    pub fn line(&self) -> usize {
        match self {
            Self::InvalidFieldCount { line, .. } => *line,
            Self::InvalidMountPoint { line, .. } => *line,
            Self::UnknownFsType { line, .. } => *line,
            Self::InvalidDump { line, .. } => *line,
            Self::InvalidPass { line, .. } => *line,
            Self::EmptyDeviceSpec { line } => *line,
            Self::InvalidUuidFormat { line, .. } => *line,
        }
    }
}

/// Parseia o conteúdo de /etc/fstab.
///
/// Retorna vetor de entradas válidas e coleta erros de linhas problemáticas
/// sem falhar o parse completo (linhas comentadas e vazias são ignoradas).
pub fn parse_fstab(content: &str) -> Result<(Vec<FstabEntry>, Vec<FstabParseError>)> {
    let mut entries = Vec::new();
    let mut errors = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;
        let trimmed = line.trim();

        // Ignora linhas vazias e comentários
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        match parse_fstab_line(trimmed, line_num) {
            Ok(entry) => entries.push(entry),
            Err(e) => errors.push(e),
        }
    }

    Ok((entries, errors))
}

/// Parseia uma única linha do fstab.
fn parse_fstab_line(
    line: &str,
    line_num: usize,
) -> std::result::Result<FstabEntry, FstabParseError> {
    let fields: Vec<&str> = line
        .split_whitespace()
        .take_while(|field| !field.starts_with('#'))
        .collect();

    if fields.len() != 6 {
        return Err(FstabParseError::InvalidFieldCount {
            line: line_num,
            found: fields.len(),
        });
    }

    let device_spec = parse_device_spec(&decode_fstab_field(fields[0]), line_num)?;
    let mount_point = PathBuf::from(decode_fstab_field(fields[1]));
    if !mount_point.is_absolute()
        && !(mount_point.as_path() == std::path::Path::new("none")
            && FilesystemType::from_string(fields[2]) == FilesystemType::Swap)
    {
        return Err(FstabParseError::InvalidMountPoint {
            line: line_num,
            mount_point: mount_point.display().to_string(),
        });
    }
    let fs_type = parse_fs_type(fields[2]);
    let options = FstabOptions::parse(&decode_fstab_field(fields[3]));
    let dump = fields[4]
        .parse::<u8>()
        .map_err(|_| FstabParseError::InvalidDump {
            line: line_num,
            dump: fields[4].to_string(),
        })?;
    let pass = fields[5]
        .parse::<u8>()
        .map_err(|_| FstabParseError::InvalidPass {
            line: line_num,
            pass: fields[5].to_string(),
        })?;

    Ok(FstabEntry {
        device_spec,
        mount_point,
        fs_type,
        options,
        dump,
        pass,
        raw_line: line.to_string(),
        line_number: line_num,
    })
}

fn decode_fstab_field(field: &str) -> String {
    let mut decoded = String::with_capacity(field.len());
    let mut chars = field.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let mut octal = String::new();
            for _ in 0..3 {
                if let Some(next) = chars.peek().copied()
                    && next.is_ascii_digit()
                {
                    octal.push(next);
                    chars.next();
                } else {
                    break;
                }
            }

            match octal.as_str() {
                "040" => decoded.push(' '),
                "011" => decoded.push('\t'),
                "012" => decoded.push('\n'),
                "134" => decoded.push('\\'),
                "043" => decoded.push('#'),
                _ => {
                    decoded.push('\\');
                    decoded.push_str(&octal);
                }
            }
        } else {
            decoded.push(ch);
        }
    }

    decoded
}

/// Parseia a especificação do dispositivo (UUID=, PARTUUID=, LABEL=, ou path).
fn parse_device_spec(
    spec: &str,
    line_num: usize,
) -> std::result::Result<FstabDeviceSpec, FstabParseError> {
    if spec.is_empty() {
        return Err(FstabParseError::EmptyDeviceSpec { line: line_num });
    }

    if let Some(uuid) = spec.strip_prefix("UUID=") {
        if uuid.is_empty() {
            return Err(FstabParseError::InvalidUuidFormat {
                line: line_num,
                spec: spec.to_string(),
            });
        }
        Ok(FstabDeviceSpec::Uuid(uuid.to_string()))
    } else if let Some(partuuid) = spec.strip_prefix("PARTUUID=") {
        if partuuid.is_empty() {
            return Err(FstabParseError::InvalidUuidFormat {
                line: line_num,
                spec: spec.to_string(),
            });
        }
        Ok(FstabDeviceSpec::PartUuid(partuuid.to_string()))
    } else if let Some(label) = spec.strip_prefix("LABEL=") {
        if label.is_empty() {
            return Err(FstabParseError::EmptyDeviceSpec { line: line_num });
        }
        Ok(FstabDeviceSpec::Label(label.to_string()))
    } else {
        // Caminho de dispositivo direto
        Ok(FstabDeviceSpec::Path(PathBuf::from(spec)))
    }
}

/// Parseia tipo de filesystem.
fn parse_fs_type(fs_type: &str) -> FilesystemType {
    FilesystemType::from_string(fs_type)
}

/// Resolve especificações de dispositivo do fstab para caminhos reais usando dispositivos descobertos.
///
/// Retorna mapeamento de FstabDeviceSpec para caminho do dispositivo (PathBuf).
/// Dispositivos não encontrados ficam de fora do mapa.
pub fn resolve_device_specs(
    entries: &[FstabEntry],
    discovered_devices: &[crate::discovery::types::BlockDevice],
) -> HashMap<FstabDeviceSpec, PathBuf> {
    resolve_device_specs_detailed(entries, discovered_devices).resolved
}

/// Resolve identificadores sem selecionar arbitrariamente em caso de duplicidade.
pub fn resolve_device_specs_detailed(
    entries: &[FstabEntry],
    discovered_devices: &[crate::discovery::types::BlockDevice],
) -> DeviceResolution {
    let mut result = DeviceResolution::default();
    let mut all_devices = Vec::new();
    fn collect_devices<'a>(
        dev: &'a crate::discovery::types::BlockDevice,
        out: &mut Vec<&'a crate::discovery::types::BlockDevice>,
    ) {
        out.push(dev);
        for child in &dev.children {
            collect_devices(child, out);
        }
    }
    for dev in discovered_devices {
        collect_devices(dev, &mut all_devices);
    }

    let mut seen_specs = std::collections::HashSet::new();
    for entry in entries {
        let spec = &entry.device_spec;
        if !seen_specs.insert(spec.clone()) {
            continue;
        }

        let mut paths = std::collections::BTreeSet::new();
        for device in &all_devices {
            let matches = match spec {
                FstabDeviceSpec::Uuid(uuid) => device.uuid.as_deref() == Some(uuid),
                FstabDeviceSpec::PartUuid(partuuid) => device.partuuid.as_deref() == Some(partuuid),
                FstabDeviceSpec::Label(label) => device.label.as_deref() == Some(label),
                FstabDeviceSpec::Path(path) => device.path == *path,
            };
            if matches {
                paths.insert(device.path.clone());
            }
        }

        match paths.len() {
            0 => result.missing.push(spec.clone()),
            1 => {
                if let Some(path) = paths.into_iter().next() {
                    result.resolved.insert(spec.clone(), path);
                }
            }
            _ => {
                result
                    .ambiguous
                    .insert(spec.clone(), paths.into_iter().collect());
            }
        }
    }

    result
}

/// Converte FstabDeviceSpec para string (para exibição/logging).
pub fn spec_to_string(spec: &FstabDeviceSpec) -> String {
    match spec {
        FstabDeviceSpec::Path(p) => p.display().to_string(),
        FstabDeviceSpec::Uuid(u) => format!("UUID={}", u),
        FstabDeviceSpec::PartUuid(p) => format!("PARTUUID={}", p),
        FstabDeviceSpec::Label(l) => format!("LABEL={}", l),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::types::{
        BlockDevice, DeviceId, DeviceType, FilesystemType, PartitionType,
    };
    use std::path::PathBuf;

    fn create_test_device(
        path: &str,
        uuid: Option<&str>,
        partuuid: Option<&str>,
        label: Option<&str>,
        fs: FilesystemType,
    ) -> BlockDevice {
        BlockDevice {
            path: PathBuf::from(path),
            name: path.trim_start_matches("/dev/").to_string(),
            device_id: Some(DeviceId::from_major_minor(8, 1)),
            device_type: DeviceType::Partition,
            size: Some(1024 * 1024 * 1024),
            sector_size: Some(512),
            logical_sector_size: Some(512),
            model: None,
            serial: None,
            vendor: None,
            rotation_rate: None,
            removable: Some(false),
            read_only: Some(false),
            partition_table: None,
            uuid: uuid.map(String::from),
            partuuid: partuuid.map(String::from),
            label: label.map(String::from),
            partition_type: Some(PartitionType::LinuxFilesystem),
            filesystem: Some(fs),
            mountpoint: None,
            mountpoints: vec![],
            mount_options: vec![],
            parent: Some("sda".to_string()),
            sysfs_path: None,
            children: vec![],
        }
    }

    #[test]
    fn test_parse_fstab_basic() {
        let content = r#"
# Comment line
UUID=1234-5678  /  ext4  defaults  0  1
PARTUUID=abcd-ef00  /boot  ext4  rw,relatime  0  2
LABEL=HOME  /home  btrfs  rw,subvol=@home  0  2
/dev/sda3  /mnt/data  xfs  nofail,noauto  0  0
"#;

        let (entries, errors) = parse_fstab(content).unwrap();
        assert!(errors.is_empty());
        assert_eq!(entries.len(), 4);

        // UUID entry
        assert_eq!(
            entries[0].device_spec,
            FstabDeviceSpec::Uuid("1234-5678".to_string())
        );
        assert_eq!(entries[0].mount_point, PathBuf::from("/"));
        assert_eq!(entries[0].fs_type, FilesystemType::Ext4);
        assert!(!entries[0].options.noauto);
        assert!(!entries[0].options.nofail);

        // PARTUUID entry
        assert_eq!(
            entries[1].device_spec,
            FstabDeviceSpec::PartUuid("abcd-ef00".to_string())
        );
        assert_eq!(entries[1].mount_point, PathBuf::from("/boot"));

        // LABEL entry
        assert_eq!(
            entries[2].device_spec,
            FstabDeviceSpec::Label("HOME".to_string())
        );
        assert_eq!(entries[2].mount_point, PathBuf::from("/home"));
        assert_eq!(entries[2].options.subvol, Some("@home".to_string()));

        // Path entry
        assert_eq!(
            entries[3].device_spec,
            FstabDeviceSpec::Path(PathBuf::from("/dev/sda3"))
        );
        assert_eq!(entries[3].mount_point, PathBuf::from("/mnt/data"));
        assert!(entries[3].options.nofail);
        assert!(entries[3].options.noauto);
    }

    #[test]
    fn test_parse_fstab_subvolid() {
        let content = "UUID=xxxx  /  btrfs  rw,subvolid=256  0  1\n";
        let (entries, _) = parse_fstab(content).unwrap();
        assert_eq!(entries[0].options.subvol, Some("id=256".to_string()));
    }

    #[test]
    fn test_parse_fstab_ignores_comments_and_empty() {
        let content = r#"

# This is a comment
UUID=1234  /  ext4  defaults  0  1

# Another comment

PARTUUID=abcd  /boot  ext4  defaults  0  2
"#;
        let (entries, _) = parse_fstab(content).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_parse_fstab_errors_collected() {
        let content = "UUID=  /  ext4  defaults  0  1\n"; // UUID vazio
        let (entries, errors) = parse_fstab(content).unwrap();
        assert!(entries.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            FstabParseError::InvalidUuidFormat { .. }
        ));
    }

    #[test]
    fn test_fstab_options_to_mount_options() {
        let opts = FstabOptions {
            raw: "rw,relatime,subvol=@".to_string(),
            read_write: true,
            subvol: Some("@".to_string()),
            other: vec!["relatime".to_string()],
            ..Default::default()
        };
        let mount_opts = opts.to_mount_options();
        assert!(mount_opts.contains("rw"));
        assert!(mount_opts.contains("subvol=@"));
        assert!(mount_opts.contains("relatime"));
    }

    #[test]
    fn test_resolve_device_specs() {
        let devices = vec![
            create_test_device(
                "/dev/sda1",
                Some("root-uuid"),
                Some("root-partuuid"),
                Some("ROOT"),
                FilesystemType::Ext4,
            ),
            create_test_device(
                "/dev/sda2",
                Some("home-uuid"),
                Some("home-partuuid"),
                Some("HOME"),
                FilesystemType::Btrfs,
            ),
            create_test_device(
                "/dev/sda3",
                None,
                Some("boot-partuuid"),
                None,
                FilesystemType::Vfat,
            ),
        ];

        let entries = vec![
            FstabEntry {
                device_spec: FstabDeviceSpec::Uuid("root-uuid".to_string()),
                mount_point: PathBuf::from("/"),
                fs_type: FilesystemType::Ext4,
                options: FstabOptions::default(),
                dump: 0,
                pass: 1,
                raw_line: "".to_string(),
                line_number: 1,
            },
            FstabEntry {
                device_spec: FstabDeviceSpec::Label("HOME".to_string()),
                mount_point: PathBuf::from("/home"),
                fs_type: FilesystemType::Btrfs,
                options: FstabOptions::default(),
                dump: 0,
                pass: 2,
                raw_line: "".to_string(),
                line_number: 2,
            },
            FstabEntry {
                device_spec: FstabDeviceSpec::PartUuid("boot-partuuid".to_string()),
                mount_point: PathBuf::from("/boot"),
                fs_type: FilesystemType::Vfat,
                options: FstabOptions::default(),
                dump: 0,
                pass: 2,
                raw_line: "".to_string(),
                line_number: 3,
            },
            FstabEntry {
                device_spec: FstabDeviceSpec::Path(PathBuf::from("/dev/nonexistent")),
                mount_point: PathBuf::from("/mnt"),
                fs_type: FilesystemType::Ext4,
                options: FstabOptions::default(),
                dump: 0,
                pass: 0,
                raw_line: "".to_string(),
                line_number: 4,
            },
        ];

        let resolved = resolve_device_specs(&entries, &devices);
        assert_eq!(resolved.len(), 3);
        assert_eq!(
            resolved.get(&FstabDeviceSpec::Uuid("root-uuid".to_string())),
            Some(&PathBuf::from("/dev/sda1"))
        );
        assert_eq!(
            resolved.get(&FstabDeviceSpec::Label("HOME".to_string())),
            Some(&PathBuf::from("/dev/sda2"))
        );
        assert_eq!(
            resolved.get(&FstabDeviceSpec::PartUuid("boot-partuuid".to_string())),
            Some(&PathBuf::from("/dev/sda3"))
        );
        assert!(!resolved.contains_key(&FstabDeviceSpec::Path(PathBuf::from("/dev/nonexistent"))));
    }

    #[test]
    fn test_fstab_entry_safe_projection_excludes_raw_line() {
        let entry = FstabEntry {
            device_spec: FstabDeviceSpec::Uuid("test-uuid".to_string()),
            mount_point: PathBuf::from("/"),
            fs_type: FilesystemType::Ext4,
            options: FstabOptions::parse("rw,relatime"),
            dump: 0,
            pass: 1,
            raw_line: "UUID=test-uuid / ext4 rw,relatime 0 1".to_string(),
            line_number: 1,
        };

        let safe = entry.to_safe_projection();
        assert_eq!(
            safe.device_spec,
            FstabDeviceSpec::Uuid("test-uuid".to_string())
        );
        assert_eq!(safe.mount_point, PathBuf::from("/"));
        assert_eq!(safe.fs_type, FilesystemType::Ext4);
        assert_eq!(safe.line_number, 1);

        // Serialização não deve incluir raw_line
        let json = serde_json::to_string(&safe).unwrap();
        assert!(!json.contains("raw_line"));
        assert!(!json.contains("test-uuid / ext4"));
    }

    #[test]
    fn test_fstab_options_no_raw_in_serialization() {
        let opts = FstabOptions::parse("rw,relatime,subvol=@,nofail");
        let json = serde_json::to_string(&opts).unwrap();
        // raw não deve ser serializado
        assert!(!json.contains("\"raw\""));
        assert!(json.contains("nofail"));
        assert!(json.contains("subvol"));
    }

    #[test]
    fn test_device_spec_to_string() {
        let entry = FstabEntry {
            device_spec: FstabDeviceSpec::Uuid("abc-123".to_string()),
            mount_point: PathBuf::from("/"),
            fs_type: FilesystemType::Ext4,
            options: FstabOptions::default(),
            dump: 0,
            pass: 1,
            raw_line: "".to_string(),
            line_number: 1,
        };
        assert_eq!(entry.device_spec_to_string(), "UUID=abc-123");

        let entry2 = FstabEntry {
            device_spec: FstabDeviceSpec::Path(PathBuf::from("/dev/sda1")),
            ..entry
        };
        assert_eq!(entry2.device_spec_to_string(), "/dev/sda1");
    }
}
