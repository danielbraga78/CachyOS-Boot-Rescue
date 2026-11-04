//! Plano de montagem explícito para instalação Linux.
//!
//! Define dispositivos, filesystems, mountpoints, opções, subvolumes
//! e dependências entre montagens.

use crate::discovery::types::{BlockDevice, FilesystemType};
use crate::storage::fstab::FstabEntrySafe;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Tipo de montagem no plano.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MountType {
    /// Montagem raiz (/)
    Root,
    /// Montagem de boot (/boot)
    Boot,
    /// Montagem de ESP (/boot/efi ou /efi)
    Esp,
    /// Montagem de home separado
    Home,
    /// Montagem de var separado
    Var,
    /// Montagem genérica (outros)
    Other,
}

/// Status de uma entrada no plano de montagem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MountEntryStatus {
    /// Montagem planejada e será executada
    Planned,
    /// Montagem ignorada (noauto)
    SkippedNoauto,
    /// Montagem ignorada (dispositivo não encontrado, nofail)
    SkippedNofailMissing,
    /// Montagem ignorada (dispositivo ambíguo)
    SkippedAmbiguous,
    /// Montagem já existe no sistema
    AlreadyMounted,
    /// Montagem falhou (será preenchido durante execução)
    Failed,
}

/// Entrada individual no plano de montagem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountPlanEntry {
    /// Tipo de montagem.
    pub mount_type: MountType,
    /// Caminho do dispositivo a montar.
    pub device_path: PathBuf,
    /// Ponto de montagem destino (caminho lógico dentro do sistema alvo, ex: /boot/efi).
    /// NÃO é o caminho absoluto no host - deve ser resolvido em relação ao root_mount.
    pub mount_point: PathBuf,
    /// Tipo de filesystem.
    pub fs_type: FilesystemType,
    /// Opções de montagem.
    pub options: String,
    /// Se é montagem Btrfs com subvolume.
    pub is_btrfs_subvolume: bool,
    /// Nome do subvolume (se aplicável).
    pub subvolume: Option<String>,
    /// Status da entrada no plano.
    pub status: MountEntryStatus,
    /// Dispositivo de origem (para rastreamento).
    pub source_device: Option<BlockDevice>,
    /// Entrada fstab original (se aplicável) - projeção segura sem raw_line.
    pub fstab_entry: Option<FstabEntrySafe>,
    /// Dependências: mount points que devem ser montados antes deste.
    pub dependencies: Vec<PathBuf>,
    /// Erro associado (se status == Failed).
    pub error: Option<String>,
}

impl MountPlanEntry {
    /// Retorna o ponto de montagem resolvido para o host (root_mount + mount_point).
    pub fn resolved_mount_point(&self, root_mount: &Path) -> PathBuf {
        // mount_point é absoluto (ex: /boot/efi), removemos a barra inicial
        let relative = self
            .mount_point
            .strip_prefix("/")
            .unwrap_or(&self.mount_point);
        root_mount.join(relative)
    }

    /// Verifica se a entrada deve ser montada (não ignorada, não já montada).
    pub fn should_mount(&self) -> bool {
        matches!(
            self.status,
            MountEntryStatus::Planned | MountEntryStatus::AlreadyMounted
        )
    }
}

/// Erro de validação do plano de montagem.
#[derive(Debug, thiserror::Error)]
pub enum MountPlanValidationError {
    #[error("Dependência ausente: {mount_point} requer {dependency} que não está no plano")]
    MissingDependency {
        mount_point: String,
        dependency: String,
    },
    #[error("Dependência cíclica detectada envolvendo: {0}")]
    CircularDependency(String),
    #[error("Ponto de montagem duplicado: {0}")]
    DuplicateMountPoint(String),
    #[error("Filesystem desconhecido: {0}")]
    UnknownFilesystem(String),
    #[error(
        "Entrada raiz do fstab ({fstab_device}) não corresponde ao dispositivo da instalação selecionada ({candidate_device})"
    )]
    RootFstabMismatch {
        fstab_device: String,
        candidate_device: String,
    },
    #[error("Ponto de montagem escapa da raiz: {0}")]
    MountPointEscapesRoot(String),
    #[error("Ponto de montagem inválido (relativo ou com ..): {0}")]
    InvalidMountPoint(String),
}

/// Plano completo de montagem para uma instalação.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountPlan {
    /// Entradas de montagem ordenadas por dependência (raiz primeiro).
    pub entries: Vec<MountPlanEntry>,
    /// Mapeamento de device_spec fstab para caminho resolvido.
    pub resolved_devices: HashMap<crate::storage::fstab::FstabDeviceSpec, PathBuf>,
    /// Especificações ambíguas (múltiplos dispositivos).
    pub ambiguous_devices: HashMap<crate::storage::fstab::FstabDeviceSpec, Vec<PathBuf>>,
    /// Especificações não encontradas.
    pub missing_devices: Vec<crate::storage::fstab::FstabDeviceSpec>,
    /// Avisos gerados durante a criação do plano.
    pub warnings: Vec<String>,
    /// Erros que impedem execução do plano.
    pub errors: Vec<String>,
}

impl MountPlan {
    /// Cria um plano vazio.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            resolved_devices: HashMap::new(),
            ambiguous_devices: HashMap::new(),
            missing_devices: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Adiciona uma entrada ao plano.
    pub fn add_entry(&mut self, entry: MountPlanEntry) {
        self.entries.push(entry);
    }

    /// Verifica se o plano tem erros que impedem execução.
    pub fn has_blocking_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Retorna entradas ordenadas para montagem (topológica por dependências).
    /// Falha se houver dependências ausentes ou ciclos.
    pub fn ordered_for_mount(&self) -> Result<Vec<&MountPlanEntry>, MountPlanValidationError> {
        let mut sorted: Vec<&MountPlanEntry> = Vec::new();
        let mut remaining: Vec<&MountPlanEntry> = self.entries.iter().collect();
        let mut processed = HashSet::new();

        // Validação prévia: detecta dependências ausentes
        let all_mount_points: HashSet<_> = self.entries.iter().map(|e| &e.mount_point).collect();
        for entry in &self.entries {
            for dep in &entry.dependencies {
                if !all_mount_points.contains(dep) && dep.as_os_str() != "/" {
                    return Err(MountPlanValidationError::MissingDependency {
                        mount_point: entry.mount_point.display().to_string(),
                        dependency: dep.display().to_string(),
                    });
                }
            }
        }

        // Validação: pontos de montagem duplicados
        let mut seen = HashSet::new();
        for entry in &self.entries {
            if !seen.insert(&entry.mount_point) {
                return Err(MountPlanValidationError::DuplicateMountPoint(
                    entry.mount_point.display().to_string(),
                ));
            }
        }

        while !remaining.is_empty() {
            let mut progress = false;
            let mut i = 0;
            while i < remaining.len() {
                let entry = remaining[i];
                let deps_satisfied = entry.dependencies.iter().all(|d| processed.contains(d));
                if deps_satisfied {
                    sorted.push(entry);
                    processed.insert(entry.mount_point.clone());
                    remaining.remove(i);
                    progress = true;
                } else {
                    i += 1;
                }
            }
            if !progress {
                // Ciclo detectado
                let cycle: Vec<String> = remaining
                    .iter()
                    .map(|e| e.mount_point.display().to_string())
                    .collect();
                return Err(MountPlanValidationError::CircularDependency(
                    cycle.join(" -> "),
                ));
            }
        }
        Ok(sorted)
    }

    /// Retorna entradas ordenadas para desmontagem (inverso da montagem).
    pub fn ordered_for_unmount(&self) -> Vec<&MountPlanEntry> {
        // Para desmontagem, usamos a ordem inversa da montagem bem-sucedida
        // Entradas com erro não precisam ser desmontadas
        self.entries
            .iter()
            .filter(|e| e.status != MountEntryStatus::Failed)
            .rev()
            .collect()
    }

    /// Adiciona um aviso.
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Adiciona um erro bloqueante.
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Gera resumo textual do plano para exibição ao usuário.
    pub fn format_summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push("=== Plano de Montagem ===".to_string());
        lines.push(format!("Entradas: {}", self.entries.len()));
        lines.push(format!("Avisos: {}", self.warnings.len()));
        lines.push(format!("Erros: {}", self.errors.len()));
        lines.push(String::new());

        for entry in &self.entries {
            let status_str = match entry.status {
                MountEntryStatus::Planned => " (planejado)",
                MountEntryStatus::SkippedNoauto => " (ignorado: noauto)",
                MountEntryStatus::SkippedNofailMissing => {
                    " (ignorado: nofail, dispositivo ausente)"
                }
                MountEntryStatus::SkippedAmbiguous => " (ignorado: dispositivo ambíguo)",
                MountEntryStatus::AlreadyMounted => " (já montado)",
                MountEntryStatus::Failed => " (FALHOU)",
            };
            let subvol = entry
                .subvolume
                .as_deref()
                .map_or(String::new(), |s| format!(" subvol={}", s));
            lines.push(format!(
                "  {} -> {} ({:?}){}{}",
                entry.device_path.display(),
                entry.mount_point.display(),
                entry.fs_type,
                subvol,
                status_str
            ));
            if let Some(ref err) = entry.error {
                lines.push(format!("    Erro: {}", err));
            }
        }

        if !self.warnings.is_empty() {
            lines.push(String::new());
            lines.push("Avisos:".to_string());
            for w in &self.warnings {
                lines.push(format!("  - {}", w));
            }
        }

        if !self.errors.is_empty() {
            lines.push(String::new());
            lines.push("Erros:".to_string());
            for e in &self.errors {
                lines.push(format!("  - {}", e));
            }
        }

        lines.join("\n")
    }

    /// Projeção segura para serialização (sem dados sensíveis).
    pub fn to_safe_projection(&self) -> MountPlanSafe {
        MountPlanSafe {
            entries: self
                .entries
                .iter()
                .map(|e| MountPlanEntrySafe {
                    mount_type: e.mount_type,
                    device_path: e.device_path.clone(),
                    mount_point: e.mount_point.clone(),
                    fs_type: e.fs_type,
                    options: e.options.clone(),
                    is_btrfs_subvolume: e.is_btrfs_subvolume,
                    subvolume: e.subvolume.clone(),
                    status: e.status,
                    fstab_entry: e.fstab_entry.clone(),
                    dependencies: e.dependencies.clone(),
                    error: e.error.clone(),
                })
                .collect(),
            resolved_devices: self
                .resolved_devices
                .iter()
                .map(|(k, v)| (spec_to_string(k), v.clone()))
                .collect(),
            ambiguous_devices: self
                .ambiguous_devices
                .iter()
                .map(|(k, v)| (spec_to_string(k), v.clone()))
                .collect(),
            missing_devices: self.missing_devices.iter().map(spec_to_string).collect(),
            warnings: self.warnings.clone(),
            errors: self.errors.clone(),
        }
    }
}

/// Projeção segura de MountPlanEntry para serialização.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountPlanEntrySafe {
    pub mount_type: MountType,
    pub device_path: PathBuf,
    pub mount_point: PathBuf,
    pub fs_type: FilesystemType,
    pub options: String,
    pub is_btrfs_subvolume: bool,
    pub subvolume: Option<String>,
    pub status: MountEntryStatus,
    pub fstab_entry: Option<FstabEntrySafe>,
    pub dependencies: Vec<PathBuf>,
    pub error: Option<String>,
}

/// Projeção segura de MountPlan para serialização.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountPlanSafe {
    pub entries: Vec<MountPlanEntrySafe>,
    // HashMap com chaves não-string não serializa bem em JSON - omitimos
    pub resolved_devices: Vec<(String, PathBuf)>,
    pub ambiguous_devices: Vec<(String, Vec<PathBuf>)>,
    pub missing_devices: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl Default for MountPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Constrói MountPlan a partir de candidato de instalação e dispositivos descobertos.
pub fn build_mount_plan(
    candidate: &crate::discovery::types::LinuxInstallationCandidate,
    all_devices: &[crate::discovery::types::BlockDevice],
    fstab_content: Option<&str>,
    root_mount: &Path,
) -> MountPlan {
    let mut plan = MountPlan::new();

    // Validação: root_mount deve ser absoluto
    if !root_mount.is_absolute() {
        plan.add_error("root_mount deve ser um caminho absoluto".to_string());
        return plan;
    }

    // Parse fstab se disponível
    let (fstab_entries, fstab_errors) = if let Some(content) = fstab_content {
        match crate::storage::fstab::parse_fstab(content) {
            Ok((entries, errors)) => (entries, errors),
            Err(e) => {
                plan.add_error(format!("Falha ao parsear fstab: {}", e));
                (Vec::new(), Vec::new())
            }
        }
    } else {
        (Vec::new(), Vec::new())
    };

    for err in fstab_errors {
        plan.add_warning(format!("fstab linha {}: {}", err.line(), err));
    }

    // Resolve device specs com detalhes (ambíguo/faltando)
    let resolution =
        crate::storage::fstab::resolve_device_specs_detailed(&fstab_entries, all_devices);
    plan.resolved_devices = resolution.resolved.clone();
    plan.ambiguous_devices = resolution.ambiguous.clone();
    plan.missing_devices = resolution.missing.clone();

    // Verifica ambiguidades e marca entradas como ignoradas
    for (spec, paths) in &resolution.ambiguous {
        plan.add_warning(format!(
            "Dispositivo ambíguo para '{}': {} candidatos ({})",
            spec_to_string(spec),
            paths.len(),
            paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Verifica dispositivos faltando
    for spec in &resolution.missing {
        plan.add_warning(format!(
            "Dispositivo não encontrado para '{}'",
            spec_to_string(spec)
        ));
    }

    // Verifica se raiz está resolvida
    let root_device_path = &candidate.root_device.path;
    let root_device_uuid = candidate.root_device.uuid.clone();
    let root_fs_type = candidate
        .root_device
        .filesystem
        .unwrap_or(FilesystemType::Unknown);

    // Validação: se fstab tem entrada para /, deve corresponder ao dispositivo do candidato
    if let Some(root_fstab_entry) = fstab_entries
        .iter()
        .find(|e| e.mount_point.as_os_str() == "/")
    {
        let fstab_device = resolution.resolved.get(&root_fstab_entry.device_spec);
        if let Some(resolved_path) = fstab_device {
            if resolved_path != root_device_path {
                plan.add_error(format!(
                    "Entrada raiz do fstab ({}) não corresponde ao dispositivo da instalação selecionada ({})",
                    resolved_path.display(),
                    root_device_path.display()
                ));
            } else if let (Some(root_fstab_uuid), Some(candidate_uuid)) =
                (root_fstab_entry.device_spec.uuid(), &root_device_uuid)
                && root_fstab_uuid != *candidate_uuid
            {
                plan.add_warning(format!(
                    "UUID da entrada raiz do fstab ({}) difere do UUID do candidato ({})",
                    root_fstab_uuid, candidate_uuid
                ));
            }
        } else if resolution
            .ambiguous
            .contains_key(&root_fstab_entry.device_spec)
        {
            plan.add_error("Entrada raiz do fstab tem dispositivo ambíguo".to_string());
        } else if resolution.missing.contains(&root_fstab_entry.device_spec) {
            plan.add_error(
                "Entrada raiz do fstab refere-se a dispositivo não encontrado".to_string(),
            );
        }
    }

    // Entrada raiz (sempre primeira)
    let root_entry = MountPlanEntry {
        mount_type: MountType::Root,
        device_path: root_device_path.clone(),
        mount_point: PathBuf::from("/"),
        fs_type: root_fs_type,
        options: "rw,relatime".to_string(),
        is_btrfs_subvolume: candidate.is_btrfs_subvolume,
        subvolume: candidate.btrfs_subvolume.clone(),
        status: if candidate.root_device.mountpoint.is_some() {
            MountEntryStatus::AlreadyMounted
        } else {
            MountEntryStatus::Planned
        },
        source_device: Some(candidate.root_device.clone()),
        fstab_entry: None,
        dependencies: vec![],
        error: None,
    };
    plan.add_entry(root_entry);

    // Processa entradas fstab para outros mount points
    for fstab_entry in &fstab_entries {
        // Pula entrada raiz (já adicionada)
        if fstab_entry.mount_point.as_os_str() == "/" {
            continue;
        }

        // Pula swap
        if fstab_entry.fs_type == FilesystemType::Swap {
            continue;
        }

        // Validação: ponto de montagem deve ser absoluto e não escapar da raiz
        if !fstab_entry.mount_point.is_absolute() {
            plan.add_error(format!(
                "Ponto de montagem não é absoluto: {}",
                fstab_entry.mount_point.display()
            ));
            continue;
        }
        // Verifica se tem componentes ".." que poderiam escapar
        if fstab_entry
            .mount_point
            .components()
            .any(|c| c.as_os_str() == "..")
        {
            plan.add_error(format!(
                "Ponto de montagem contém '..' (escapa da raiz): {}",
                fstab_entry.mount_point.display()
            ));
            continue;
        }

        // Validação: filesystem conhecido
        if fstab_entry.fs_type == FilesystemType::Unknown {
            plan.add_error(format!(
                "Filesystem desconhecido para {}: {:?}",
                fstab_entry.mount_point.display(),
                fstab_entry.fs_type
            ));
            continue;
        }

        // Verifica noauto PRIMEIRO - se noauto, não monta independentemente do dispositivo
        if fstab_entry.options.noauto {
            let entry = MountPlanEntry {
                mount_type: mount_type_from_path(&fstab_entry.mount_point),
                device_path: PathBuf::new(),
                mount_point: fstab_entry.mount_point.clone(),
                fs_type: fstab_entry.fs_type,
                options: String::new(),
                is_btrfs_subvolume: false,
                subvolume: None,
                status: MountEntryStatus::SkippedNoauto,
                source_device: None,
                fstab_entry: Some(fstab_entry.to_safe_projection()),
                dependencies: vec![PathBuf::from("/")],
                error: Some("noauto - não será montado automaticamente".to_string()),
            };
            plan.add_entry(entry);
            continue;
        }

        // Resolve dispositivo
        let (device_path, status) = match resolution.resolved.get(&fstab_entry.device_spec) {
            Some(p) => (p.clone(), MountEntryStatus::Planned),
            None => {
                if resolution.ambiguous.contains_key(&fstab_entry.device_spec) {
                    (PathBuf::new(), MountEntryStatus::SkippedAmbiguous)
                } else if fstab_entry.options.nofail {
                    plan.add_warning(format!(
                        "Dispositivo não encontrado para '{}' (mount point: {}), ignorado devido a nofail",
                        fstab_entry.device_spec_to_string(),
                        fstab_entry.mount_point.display()
                    ));
                    (PathBuf::new(), MountEntryStatus::SkippedNofailMissing)
                } else {
                    plan.add_warning(format!(
                        "Dispositivo não encontrado para '{}' (mount point: {})",
                        fstab_entry.device_spec_to_string(),
                        fstab_entry.mount_point.display()
                    ));
                    (PathBuf::new(), MountEntryStatus::SkippedNofailMissing)
                }
            }
        };

        // Se dispositivo não resolvido (ambíguo ou faltando), adiciona entrada com status apropriado
        if device_path.as_os_str().is_empty() {
            let entry = MountPlanEntry {
                mount_type: mount_type_from_path(&fstab_entry.mount_point),
                device_path: PathBuf::new(),
                mount_point: fstab_entry.mount_point.clone(),
                fs_type: fstab_entry.fs_type,
                options: String::new(),
                is_btrfs_subvolume: false,
                subvolume: None,
                status,
                source_device: None,
                fstab_entry: Some(fstab_entry.to_safe_projection()),
                dependencies: vec![PathBuf::from("/")],
                error: match status {
                    MountEntryStatus::SkippedAmbiguous => {
                        Some("Dispositivo ambíguo - não montado por segurança".to_string())
                    }
                    MountEntryStatus::SkippedNofailMissing => {
                        Some("Dispositivo não encontrado (nofail)".to_string())
                    }
                    _ => None,
                },
            };
            plan.add_entry(entry);
            continue;
        }

        // Determina tipo de montagem
        let mount_type = mount_type_from_path(&fstab_entry.mount_point);

        // Verifica se já montado
        let already_mounted = all_devices
            .iter()
            .any(|d| d.mountpoint.as_ref() == Some(&fstab_entry.mount_point));

        let final_status = if already_mounted {
            MountEntryStatus::AlreadyMounted
        } else {
            status
        };

        // Encontra dispositivo de origem
        let source_device = all_devices.iter().find(|d| d.path == device_path).cloned();

        let entry = MountPlanEntry {
            mount_type,
            device_path,
            mount_point: fstab_entry.mount_point.clone(),
            fs_type: fstab_entry.fs_type,
            options: fstab_entry.options.to_mount_options(),
            is_btrfs_subvolume: false, // Será detectado se Btrfs
            subvolume: fstab_entry.options.subvol.clone(),
            status: final_status,
            source_device,
            fstab_entry: Some(fstab_entry.to_safe_projection()),
            dependencies: vec![PathBuf::from("/")], // Depende da raiz
            error: None,
        };
        plan.add_entry(entry);
    }

    // Adiciona partições de boot/ESP do candidato se não estiverem no fstab
    for esp in &candidate.boot_partitions {
        let mount_point = match esp.mountpoint.as_ref() {
            Some(mp) => mp.clone(),
            None => continue, // Não montada, não planejamos montar ESP não referenciada
        };

        // Validação: ponto de montagem não escapa da raiz
        if !mount_point.is_absolute() || mount_point.components().any(|c| c.as_os_str() == "..") {
            plan.add_warning(format!(
                "ESP com ponto de montagem inválido ignorado: {}",
                mount_point.display()
            ));
            continue;
        }

        // Verifica se já está no plano
        if plan.entries.iter().any(|e| e.mount_point == mount_point) {
            continue;
        }

        let mount_type = mount_type_from_path(&mount_point);

        let already_mounted = esp.device.mountpoint.is_some();
        let entry = MountPlanEntry {
            mount_type,
            device_path: esp.device.path.clone(),
            mount_point,
            fs_type: esp.filesystem.fstype,
            options: "rw,relatime,fmask=0022,dmask=0022,codepage=437,iocharset=ascii,shortname=mixed,utf8,errors=remount-ro".to_string(),
            is_btrfs_subvolume: false,
            subvolume: None,
            status: if already_mounted {
                MountEntryStatus::AlreadyMounted
            } else {
                MountEntryStatus::Planned
            },
            source_device: Some(esp.device.clone()),
            fstab_entry: None,
            dependencies: vec![PathBuf::from("/")],
            error: None,
        };
        plan.add_entry(entry);
    }

    // Detecta Btrfs subvolumes em outras partições
    for entry in &mut plan.entries {
        if entry.fs_type == FilesystemType::Btrfs
            && !entry.is_btrfs_subvolume
            && let Some(ref dev) = entry.source_device
            && candidate.is_btrfs_subvolume
            && dev.path == candidate.root_device.path
        {
            entry.is_btrfs_subvolume = true;
            if entry.subvolume.is_none() {
                entry.subvolume = candidate.btrfs_subvolume.clone();
            }
        }
    }

    // Validação final: ordenação topológica para detectar ciclos/dependências ausentes
    if let Err(e) = plan.ordered_for_mount() {
        plan.add_error(format!("Validação de dependências falhou: {}", e));
    }

    plan
}

fn mount_type_from_path(path: &Path) -> MountType {
    match path.to_string_lossy().as_ref() {
        "/boot" => MountType::Boot,
        "/boot/efi" | "/efi" => MountType::Esp,
        "/home" => MountType::Home,
        "/var" => MountType::Var,
        _ => MountType::Other,
    }
}

fn spec_to_string(spec: &crate::storage::fstab::FstabDeviceSpec) -> String {
    crate::storage::fstab::spec_to_string(spec)
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
    fn test_build_mount_plan_basic() {
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
        assert_eq!(plan.entries.len(), 2); // root + /boot/efi
        assert_eq!(plan.entries[0].mount_type, MountType::Root);
        assert_eq!(plan.entries[0].mount_point, PathBuf::from("/"));
        assert_eq!(plan.entries[1].mount_type, MountType::Esp);
        assert_eq!(plan.entries[1].mount_point, PathBuf::from("/boot/efi"));

        // Verifica mount point resolvido
        assert_eq!(
            plan.entries[1].resolved_mount_point(&root_mount),
            PathBuf::from("/mnt/target/boot/efi")
        );
    }

    #[test]
    fn test_mount_plan_noauto_skipped() {
        let candidate = create_test_candidate();
        let devices = create_test_devices();
        let root_mount = PathBuf::from("/mnt/target");
        let fstab_content =
            "UUID=root-uuid / ext4 defaults 0 1\nUUID=esp-uuid /boot/efi vfat noauto 0 2\n";
        let plan = build_mount_plan(&candidate, &devices, Some(fstab_content), &root_mount);

        assert!(!plan.has_blocking_errors());
        // /boot/efi deve estar no plano mas com status SkippedNoauto
        let esp_entry = plan
            .entries
            .iter()
            .find(|e| e.mount_point.as_os_str() == "/boot/efi");
        assert!(esp_entry.is_some());
        assert_eq!(esp_entry.unwrap().status, MountEntryStatus::SkippedNoauto);
    }

    #[test]
    fn test_mount_plan_nofail_missing_device() {
        let candidate = create_test_candidate();
        let devices = create_test_devices();
        let root_mount = PathBuf::from("/mnt/target");
        let fstab_content =
            "UUID=root-uuid / ext4 defaults 0 1\nUUID=missing-uuid /home ext4 nofail 0 2\n";
        let plan = build_mount_plan(&candidate, &devices, Some(fstab_content), &root_mount);

        assert!(!plan.has_blocking_errors());
        let home_entry = plan
            .entries
            .iter()
            .find(|e| e.mount_point.as_os_str() == "/home");
        assert!(home_entry.is_some());
        assert_eq!(
            home_entry.unwrap().status,
            MountEntryStatus::SkippedNofailMissing
        );
    }

    #[test]
    fn test_mount_plan_ambiguous_device_blocked() {
        let candidate = create_test_candidate();
        let mut devices = create_test_devices();
        // Adiciona segundo dispositivo com mesmo UUID
        devices[0].children.push(BlockDevice {
            path: PathBuf::from("/dev/sdb1"),
            name: "sdb1".to_string(),
            device_id: Some(DeviceId::from_major_minor(8, 17)),
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
            uuid: Some("esp-uuid".to_string()), // Mesmo UUID!
            partuuid: Some("esp-partuuid-2".to_string()),
            label: Some("EFI2".to_string()),
            partition_type: Some(PartitionType::EfiSystem),
            filesystem: Some(FilesystemType::Vfat),
            mountpoint: None,
            mountpoints: vec![],
            mount_options: vec![],
            parent: Some("sdb".to_string()),
            sysfs_path: None,
            children: vec![],
        });

        let root_mount = PathBuf::from("/mnt/target");
        let plan = build_mount_plan(
            &candidate,
            &devices,
            candidate.fstab_content.as_deref(),
            &root_mount,
        );

        assert!(
            plan.has_blocking_errors()
                || plan
                    .entries
                    .iter()
                    .any(|e| e.status == MountEntryStatus::SkippedAmbiguous)
        );
        // Verifica que ESP está marcada como ambígua
        let esp_entry = plan
            .entries
            .iter()
            .find(|e| e.mount_point.as_os_str() == "/boot/efi");
        assert!(esp_entry.is_some());
        assert_eq!(
            esp_entry.unwrap().status,
            MountEntryStatus::SkippedAmbiguous
        );
    }

    #[test]
    fn test_mount_plan_root_fstab_mismatch() {
        let mut candidate = create_test_candidate();
        // Muda o UUID do root device do candidato
        candidate.root_device.uuid = Some("different-uuid".to_string());

        let devices = create_test_devices();
        let root_mount = PathBuf::from("/mnt/target");
        let fstab_content = "UUID=root-uuid / ext4 defaults 0 1\n"; // fstab tem root-uuid
        let plan = build_mount_plan(&candidate, &devices, Some(fstab_content), &root_mount);

        // UUID mismatch gera warning, não erro (path é o mesmo)
        assert!(!plan.has_blocking_errors());
        assert!(plan.warnings.iter().any(|w| w.contains("difere do UUID")));
    }

    #[test]
    fn test_mount_plan_unknown_filesystem_rejected() {
        let candidate = create_test_candidate();
        let devices = create_test_devices();
        let root_mount = PathBuf::from("/mnt/target");
        let fstab_content = "UUID=root-uuid / ext4 defaults 0 1\nUUID=esp-uuid /mnt/unknown unknownfs defaults 0 2\n";
        let plan = build_mount_plan(&candidate, &devices, Some(fstab_content), &root_mount);

        assert!(plan.has_blocking_errors());
        assert!(
            plan.errors
                .iter()
                .any(|e| e.contains("Filesystem desconhecido"))
        );
    }

    #[test]
    fn test_mount_plan_duplicate_mountpoint_rejected() {
        let candidate = create_test_candidate();
        let devices = create_test_devices();
        let root_mount = PathBuf::from("/mnt/target");
        let fstab_content = "UUID=root-uuid / ext4 defaults 0 1\nUUID=esp-uuid /home ext4 defaults 0 2\nLABEL=other /home ext4 defaults 0 2\n";
        let plan = build_mount_plan(&candidate, &devices, Some(fstab_content), &root_mount);

        assert!(plan.has_blocking_errors());
        assert!(plan.errors.iter().any(|e| e.contains("duplicado")));
    }

    #[test]
    fn test_mount_plan_circular_dependency_detected() {
        let mut plan = MountPlan::new();
        plan.add_entry(MountPlanEntry {
            mount_type: MountType::Root,
            device_path: PathBuf::from("/dev/sda2"),
            mount_point: PathBuf::from("/"),
            fs_type: FilesystemType::Ext4,
            options: "rw".to_string(),
            is_btrfs_subvolume: false,
            subvolume: None,
            status: MountEntryStatus::Planned,
            source_device: None,
            fstab_entry: None,
            dependencies: vec![PathBuf::from("/home")], // Raiz depende de home (ciclo!)
            error: None,
        });
        plan.add_entry(MountPlanEntry {
            mount_type: MountType::Home,
            device_path: PathBuf::from("/dev/sda3"),
            mount_point: PathBuf::from("/home"),
            fs_type: FilesystemType::Ext4,
            options: "rw".to_string(),
            is_btrfs_subvolume: false,
            subvolume: None,
            status: MountEntryStatus::Planned,
            source_device: None,
            fstab_entry: None,
            dependencies: vec![PathBuf::from("/")],
            error: None,
        });

        let result = plan.ordered_for_mount();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MountPlanValidationError::CircularDependency(_)
        ));
    }

    #[test]
    fn test_mount_plan_missing_dependency_detected() {
        let mut plan = MountPlan::new();
        plan.add_entry(MountPlanEntry {
            mount_type: MountType::Root,
            device_path: PathBuf::from("/dev/sda2"),
            mount_point: PathBuf::from("/"),
            fs_type: FilesystemType::Ext4,
            options: "rw".to_string(),
            is_btrfs_subvolume: false,
            subvolume: None,
            status: MountEntryStatus::Planned,
            source_device: None,
            fstab_entry: None,
            dependencies: vec![],
            error: None,
        });
        plan.add_entry(MountPlanEntry {
            mount_type: MountType::Home,
            device_path: PathBuf::from("/dev/sda3"),
            mount_point: PathBuf::from("/home"),
            fs_type: FilesystemType::Ext4,
            options: "rw".to_string(),
            is_btrfs_subvolume: false,
            subvolume: None,
            status: MountEntryStatus::Planned,
            source_device: None,
            fstab_entry: None,
            dependencies: vec![PathBuf::from("/nonexistent")], // Dependência não existe no plano
            error: None,
        });

        let result = plan.ordered_for_mount();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MountPlanValidationError::MissingDependency { .. }
        ));
    }

    #[test]
    fn test_mount_plan_mount_point_escapes_root_rejected() {
        let candidate = create_test_candidate();
        let devices = create_test_devices();
        let root_mount = PathBuf::from("/mnt/target");
        let fstab_content =
            "UUID=root-uuid / ext4 defaults 0 1\nUUID=esp-uuid /../etc ext4 defaults 0 2\n";
        let plan = build_mount_plan(&candidate, &devices, Some(fstab_content), &root_mount);

        assert!(plan.has_blocking_errors());
        assert!(
            plan.errors
                .iter()
                .any(|e| e.contains("escapa da raiz") || e.contains(".."))
        );
    }

    #[test]
    fn test_mount_plan_relative_mountpoint_rejected() {
        let candidate = create_test_candidate();
        let devices = create_test_devices();
        let root_mount = PathBuf::from("/mnt/target");
        // fstab parser rejects relative paths, so this tests that parse error becomes warning
        let fstab_content =
            "UUID=root-uuid / ext4 defaults 0 1\nUUID=esp-uuid relative/path ext4 defaults 0 2\n";
        let plan = build_mount_plan(&candidate, &devices, Some(fstab_content), &root_mount);

        // Parser rejects relative mountpoint, so it becomes a warning
        assert!(!plan.has_blocking_errors());
        assert!(
            plan.warnings
                .iter()
                .any(|w| w.contains("ponto de montagem inválido"))
        );
    }

    #[test]
    fn test_mount_plan_safe_projection_excludes_sensitive() {
        let candidate = create_test_candidate();
        let devices = create_test_devices();
        let root_mount = PathBuf::from("/mnt/target");
        let plan = build_mount_plan(
            &candidate,
            &devices,
            candidate.fstab_content.as_deref(),
            &root_mount,
        );

        let safe = plan.to_safe_projection();
        let json = serde_json::to_string(&safe).unwrap();

        // Verifica que raw_line não está presente
        assert!(!json.contains("raw_line"));
        assert!(!json.contains("raw"));
    }

    #[test]
    fn test_resolved_mount_point() {
        let entry = MountPlanEntry {
            mount_type: MountType::Esp,
            device_path: PathBuf::from("/dev/sda1"),
            mount_point: PathBuf::from("/boot/efi"),
            fs_type: FilesystemType::Vfat,
            options: "rw".to_string(),
            is_btrfs_subvolume: false,
            subvolume: None,
            status: MountEntryStatus::Planned,
            source_device: None,
            fstab_entry: None,
            dependencies: vec![],
            error: None,
        };

        let root_mount = PathBuf::from("/mnt/target");
        assert_eq!(
            entry.resolved_mount_point(&root_mount),
            PathBuf::from("/mnt/target/boot/efi")
        );
    }
}
