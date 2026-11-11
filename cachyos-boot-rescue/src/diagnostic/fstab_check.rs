//! Verificação de fstab e montagem (somente leitura).
//!
//! Compara fstab com dispositivos reais, verifica UUIDs, pontos de montagem
//! e consistência com a instalação selecionada.

use crate::diagnostic::types::*;
use crate::discovery::types::FilesystemType;
use crate::discovery::types::LinuxInstallationCandidate;
use crate::error::Error;
use crate::storage::fstab::{FstabDeviceSpec, FstabEntry, parse_fstab};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Verifica fstab e montagem.
pub fn check_fstab_mount(
    environment: &BootEnvironment,
    candidate: &LinuxInstallationCandidate,
    chroot_root: &Option<PathBuf>,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    let root = chroot_root.clone().unwrap_or_else(|| PathBuf::from("/"));

    // Lê fstab do sistema alvo
    let fstab_path = root.join("etc/fstab");
    let fstab_content = match fs::read_to_string(&fstab_path) {
        Ok(c) => c,
        Err(_) => {
            findings.push(Finding {
                id: DiagnosticId::FstabUuidMismatch,
                severity: Severity::Error,
                title: "/etc/fstab não encontrado ou inacessível".to_string(),
                description: "Não foi possível ler /etc/fstab da instalação alvo. Verificações de UUID e montagem não podem ser realizadas.".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::FileMissing,
                    description: "/etc/fstab não encontrado".to_string(),
                    path: Some(fstab_path),
                    expected: Some("/etc/fstab legível".to_string()),
                    observed: Some("Erro de leitura".to_string()),
                    raw_data: HashMap::new(),
                }],
                recommendations: vec![Recommendation {
                    id: "restore_fstab".to_string(),
                    description: "Restaurar ou recriar /etc/fstab com UUIDs corretos.".to_string(),
                    action: "Gerar fstab com genfstab -U /mnt >> /mnt/etc/fstab".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::High,
                    prerequisites: vec!["Partições montadas corretamente".to_string(), "Chroot preparado".to_string()],
                }],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
            return Ok(findings);
        }
    };

    // Parse do fstab
    let (fstab_entries, _parse_errors) =
        parse_fstab(&fstab_content).map_err(|e| Error::internal(e.to_string()))?;

    if fstab_entries.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::FstabUuidMismatch,
            severity: Severity::Warning,
            title: "/etc/fstab vazio ou sem entradas válidas".to_string(),
            description: "O arquivo /etc/fstab existe mas não contém entradas de montagem válidas."
                .to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::ContentMismatch,
                description: "fstab vazio".to_string(),
                path: Some(fstab_path),
                expected: Some("Entradas de montagem".to_string()),
                observed: Some("Vazio".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
        return Ok(findings);
    }

    // 1. Verifica entrada raiz (/)
    findings.extend(check_fstab_root_entry(&fstab_entries, candidate, &root)?);

    // 2. Verifica entradas de boot (/boot, /boot/efi, /efi)
    findings.extend(check_fstab_boot_entries(
        &fstab_entries,
        candidate,
        environment,
        &root,
    )?);

    // 3. Verifica UUIDs de todas as entradas
    findings.extend(check_fstab_uuids(&fstab_entries, candidate, &root)?);

    // 4. Verifica subvolumes Btrfs
    findings.extend(check_fstab_btrfs_subvolumes(
        &fstab_entries,
        candidate,
        &root,
    )?);

    // 5. Verifica montagens atuais vs fstab
    findings.extend(check_current_mounts_vs_fstab(&fstab_entries, &root)?);

    // Se nenhum problema encontrado
    if findings.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::FstabUuidMismatch, // Info
            severity: Severity::Info,
            title: "fstab válido e consistente com dispositivos".to_string(),
            description: format!("Todas as {} entradas do fstab têm UUIDs correspondentes aos dispositivos detectados.", fstab_entries.len()),
            evidence: vec![Evidence {
                kind: EvidenceKind::ConfigValueMatches,
                description: "fstab consistente".to_string(),
                path: Some(fstab_path),
                expected: Some("UUIDs correspondentes".to_string()),
                observed: Some("OK".to_string()),
                raw_data: HashMap::from([
                    ("entries_count".to_string(), fstab_entries.len().to_string()),
                ]),
            }],
            recommendations: vec![],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    Ok(findings)
}

/// Verifica entrada raiz no fstab.
fn check_fstab_root_entry(
    entries: &[FstabEntry],
    candidate: &LinuxInstallationCandidate,
    root: &Path,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    let root_path = PathBuf::from("/");
    let root_entries: Vec<_> = entries
        .iter()
        .filter(|e| e.mount_point == root_path)
        .collect();

    if root_entries.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::FstabUuidMismatch,
            severity: Severity::Error,
            title: "Entrada raiz (/) ausente no fstab".to_string(),
            description: "Não há entrada para o filesystem raiz no fstab. O sistema pode não montar a raiz corretamente no boot.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::ConfigValueMismatch,
                description: "Entrada / ausente".to_string(),
                path: Some(root.join("etc/fstab")),
                expected: Some("UUID=xxx / <fs> defaults 0 1".to_string()),
                observed: Some("Não encontrado".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![Recommendation {
                id: "add_root_fstab".to_string(),
                description: "Adicionar entrada raiz ao fstab com UUID correto.".to_string(),
                action: "Adicionar linha: UUID=<root_uuid> / <fs_type> defaults 0 1".to_string(),
                is_mutating: true,
                risk: RiskLevel::High,
                prerequisites: vec!["UUID da partição raiz conhecido".to_string()],
            }],
            verified_in_chroot: false,
            context: HashMap::new(),
        });
        return Ok(findings);
    }

    if root_entries.len() > 1 {
        findings.push(Finding {
            id: DiagnosticId::FstabUuidMismatch,
            severity: Severity::Warning,
            title: "Múltiplas entradas raiz (/) no fstab".to_string(),
            description: format!(
                "Foram encontradas {} entradas para / no fstab. Apenas uma deve existir.",
                root_entries.len()
            ),
            evidence: vec![Evidence {
                kind: EvidenceKind::ConfigValueMismatch,
                description: "Entradas duplicadas para /".to_string(),
                path: Some(root.join("etc/fstab")),
                expected: Some("Uma única entrada".to_string()),
                observed: Some(format!("{} entradas", root_entries.len())),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![Recommendation {
                id: "fix_duplicate_root".to_string(),
                description: "Remover entradas duplicadas, manter apenas a correta.".to_string(),
                action: "Editar /etc/fstab e remover duplicatas".to_string(),
                is_mutating: true,
                risk: RiskLevel::Medium,
                prerequisites: vec!["Identificar entrada correta".to_string()],
            }],
            verified_in_chroot: false,
            context: HashMap::new(),
        });
    }

    // Verifica UUID da entrada raiz
    let root_entry = root_entries[0];
    if let Some(uuid_spec) = extract_uuid_from_spec(&root_entry.device_spec) {
        let candidate_uuid = candidate
            .root_filesystem
            .uuid
            .as_ref()
            .map(|u| u.to_lowercase());
        if let Some(c_uuid) = candidate_uuid {
            if uuid_spec.to_lowercase() != c_uuid {
                findings.push(Finding {
                    id: DiagnosticId::FstabUuidMismatch,
                    severity: Severity::Error,
                    title: "UUID da raiz no fstab diverge do dispositivo".to_string(),
                    description: format!("fstab especifica UUID {} mas a partição raiz detectada tem UUID {}. Isso impedirá a montagem da raiz no boot.", uuid_spec, c_uuid),
                    evidence: vec![Evidence {
                        kind: EvidenceKind::ConfigValueMismatch,
                        description: "UUID raiz divergente".to_string(),
                        path: Some(root.join("etc/fstab")),
                        expected: Some(c_uuid.clone()),
                        observed: Some(uuid_spec.clone()),
                        raw_data: HashMap::from([
                            ("fstab_uuid".to_string(), uuid_spec.clone()),
                            ("device_uuid".to_string(), c_uuid.clone()),
                            ("device_path".to_string(), candidate.root_device.path.display().to_string()),
                        ]),
                    }],
                    recommendations: vec![
                        Recommendation {
                            id: "fix_root_uuid".to_string(),
                            description: "Corrigir UUID no fstab para corresponder ao dispositivo real.".to_string(),
                            action: "Editar /etc/fstab: substituir UUID incorreto pelo correto".to_string(),
                            is_mutating: true,
                            risk: RiskLevel::High,
                            prerequisites: vec!["Backup do fstab".to_string(), "Confirmar UUID correto com blkid".to_string()],
                        },
                        Recommendation {
                            id: "regenerate_fstab".to_string(),
                            description: "Regerar fstab completo com genfstab.".to_string(),
                            action: "genfstab -U /mnt >> /mnt/etc/fstab (após montar partições)".to_string(),
                            is_mutating: true,
                            risk: RiskLevel::Medium,
                            prerequisites: vec!["Partições montadas em /mnt".to_string()],
                        },
                    ],
                    verified_in_chroot: false,
                    context: HashMap::new(),
                });
            } else {
                findings.push(Finding {
                    id: DiagnosticId::FstabUuidMismatch, // Info
                    severity: Severity::Info,
                    title: "UUID da raiz no fstab corresponde ao dispositivo".to_string(),
                    description: format!("Entrada raiz no fstab tem UUID correto: {}", uuid_spec),
                    evidence: vec![Evidence {
                        kind: EvidenceKind::ConfigValueMatches,
                        description: "UUID raiz consistente".to_string(),
                        path: Some(root.join("etc/fstab")),
                        expected: Some(c_uuid.clone()),
                        observed: Some(uuid_spec.clone()),
                        raw_data: HashMap::new(),
                    }],
                    recommendations: vec![],
                    verified_in_chroot: false,
                    context: HashMap::new(),
                });
            }
        }
    }

    Ok(findings)
}

/// Verifica entradas de boot no fstab.
fn check_fstab_boot_entries(
    entries: &[FstabEntry],
    candidate: &LinuxInstallationCandidate,
    environment: &BootEnvironment,
    root: &Path,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // Verifica /boot
    let boot_path = PathBuf::from("/boot");
    let boot_entries: Vec<_> = entries
        .iter()
        .filter(|e| e.mount_point == boot_path)
        .collect();
    let has_separate_boot = candidate.additional_partitions.iter().any(|p| {
        p.mountpoint
            .as_ref()
            .is_some_and(|m| m == Path::new("/boot"))
    });

    if has_separate_boot && boot_entries.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::FstabMissingBootEntry,
            severity: Severity::Error,
            title: "Entrada /boot ausente no fstab (partição separada detectada)".to_string(),
            description: "Foi detectada partição separada para /boot mas não há entrada correspondente no fstab. O /boot não será montado no boot.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::ConfigValueMismatch,
                description: "Partição /boot detectada mas sem entrada fstab".to_string(),
                path: Some(root.join("etc/fstab")),
                expected: Some("UUID=xxx /boot <fs> defaults 0 2".to_string()),
                observed: Some("Ausente".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![Recommendation {
                id: "add_boot_fstab".to_string(),
                description: "Adicionar entrada /boot ao fstab com UUID da partição de boot.".to_string(),
                action: "Adicionar linha: UUID=<boot_uuid> /boot <fs_type> defaults 0 2".to_string(),
                is_mutating: true,
                risk: RiskLevel::High,
                prerequisites: vec!["UUID da partição /boot conhecido".to_string()],
            }],
            verified_in_chroot: false,
            context: HashMap::new(),
        });
    } else if !boot_entries.is_empty() {
        // Verifica UUID
        for entry in &boot_entries {
            if let Some(uuid_spec) = extract_uuid_from_spec(&entry.device_spec) {
                let boot_uuid = candidate
                    .boot_partitions
                    .iter()
                    .find(|b| {
                        b.mountpoint
                            .as_ref()
                            .is_some_and(|m| m == Path::new("/boot"))
                    })
                    .and_then(|b| b.filesystem.uuid.as_ref().map(|u| u.to_lowercase()));

                if let Some(b_uuid) = boot_uuid
                    && uuid_spec.to_lowercase() != b_uuid
                {
                    findings.push(Finding {
                        id: DiagnosticId::FstabUuidMismatch,
                        severity: Severity::Error,
                        title: "UUID de /boot no fstab diverge do dispositivo".to_string(),
                        description: format!("fstab: {}, dispositivo: {}", uuid_spec, b_uuid),
                        evidence: vec![Evidence {
                            kind: EvidenceKind::ConfigValueMismatch,
                            description: "UUID /boot divergente".to_string(),
                            path: Some(root.join("etc/fstab")),
                            expected: Some(b_uuid.clone()),
                            observed: Some(uuid_spec.clone()),
                            raw_data: HashMap::new(),
                        }],
                        recommendations: vec![Recommendation {
                            id: "fix_boot_uuid".to_string(),
                            description: "Corrigir UUID de /boot no fstab.".to_string(),
                            action: "Editar /etc/fstab com UUID correto da partição /boot"
                                .to_string(),
                            is_mutating: true,
                            risk: RiskLevel::High,
                            prerequisites: vec!["Backup do fstab".to_string()],
                        }],
                        verified_in_chroot: false,
                        context: HashMap::new(),
                    });
                }
            }
        }
    }

    // Verifica /boot/efi ou /efi
    let efi_mounts = ["/boot/efi", "/efi"];
    for efi_mp in &efi_mounts {
        let efi_mp_path = PathBuf::from(*efi_mp);
        let efi_entries: Vec<_> = entries
            .iter()
            .filter(|e| *e.mount_point == efi_mp_path)
            .collect();

        let esp_mounted_here = environment.primary_esp.as_ref().is_some_and(|esp| {
            esp.mountpoint
                .as_ref()
                .is_some_and(|mp| mp == Path::new(*efi_mp))
        });

        if esp_mounted_here && efi_entries.is_empty() {
            findings.push(Finding {
                id: DiagnosticId::FstabMissingEspEntry,
                severity: Severity::Error,
                title: format!("Entrada {} ausente no fstab (ESP detectada)", efi_mp),
                description: format!("ESP montada em {} mas não há entrada no fstab. A ESP não será montada automaticamente no boot.", efi_mp),
                evidence: vec![Evidence {
                    kind: EvidenceKind::ConfigValueMismatch,
                    description: "ESP detectada mas sem entrada fstab".to_string(),
                    path: Some(root.join("etc/fstab")),
                    expected: Some(format!("UUID=xxx {} vfat defaults 0 2", efi_mp)),
                    observed: Some("Ausente".to_string()),
                    raw_data: HashMap::new(),
                }],
                recommendations: vec![Recommendation {
                    id: "add_esp_fstab".to_string(),
                    description: format!("Adicionar entrada {} ao fstab com UUID da ESP.", efi_mp),
                    action: format!("Adicionar linha: UUID=<esp_uuid> {} vfat defaults 0 2", efi_mp),
                    is_mutating: true,
                    risk: RiskLevel::High,
                    prerequisites: vec!["UUID da ESP conhecido".to_string()],
                }],
                verified_in_chroot: false,
                context: HashMap::new(),
            });
        } else if !efi_entries.is_empty() {
            for entry in &efi_entries {
                if let Some(uuid_spec) = extract_uuid_from_spec(&entry.device_spec) {
                    let esp_uuid = environment
                        .primary_esp
                        .as_ref()
                        .and_then(|esp| esp.uuid.as_ref().map(|u| u.to_lowercase()));

                    if let Some(e_uuid) = esp_uuid {
                        if uuid_spec.to_lowercase() != e_uuid {
                            findings.push(Finding {
                                id: DiagnosticId::FstabUuidMismatch,
                                severity: Severity::Error,
                                title: format!("UUID de {} no fstab diverge da ESP", efi_mp),
                                description: format!("fstab: {}, ESP: {}", uuid_spec, e_uuid),
                                evidence: vec![Evidence {
                                    kind: EvidenceKind::ConfigValueMismatch,
                                    description: format!("UUID {} divergente", efi_mp),
                                    path: Some(root.join("etc/fstab")),
                                    expected: Some(e_uuid.clone()),
                                    observed: Some(uuid_spec.clone()),
                                    raw_data: HashMap::new(),
                                }],
                                recommendations: vec![Recommendation {
                                    id: "fix_esp_uuid".to_string(),
                                    description: format!("Corrigir UUID de {} no fstab.", efi_mp),
                                    action: "Editar /etc/fstab com UUID correto da ESP".to_string(),
                                    is_mutating: true,
                                    risk: RiskLevel::High,
                                    prerequisites: vec!["Backup do fstab".to_string()],
                                }],
                                verified_in_chroot: false,
                                context: HashMap::new(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(findings)
}

/// Verifica UUIDs de todas as entradas do fstab.
fn check_fstab_uuids(
    entries: &[FstabEntry],
    candidate: &LinuxInstallationCandidate,
    root: &Path,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // Coleta todos os UUIDs conhecidos do candidato
    let mut known_uuids = HashMap::new();
    if let Some(uuid) = &candidate.root_filesystem.uuid {
        known_uuids.insert(uuid.to_lowercase(), "root".to_string());
    }
    for boot in &candidate.boot_partitions {
        if let Some(uuid) = &boot.filesystem.uuid {
            known_uuids.insert(
                uuid.to_lowercase(),
                format!(
                    "boot ({})",
                    boot.mountpoint
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "?".to_string())
                ),
            );
        }
    }
    for part in &candidate.additional_partitions {
        if let Some(uuid) = &part.uuid {
            known_uuids.insert(
                uuid.to_lowercase(),
                format!(
                    "part ({})",
                    part.mountpoint
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "?".to_string())
                ),
            );
        }
    }

    for entry in entries {
        if let Some(uuid_spec) = extract_uuid_from_spec(&entry.device_spec) {
            let uuid_lower = uuid_spec.to_lowercase();
            if !known_uuids.contains_key(&uuid_lower) {
                findings.push(Finding {
                    id: DiagnosticId::FstabUuidMismatch,
                    severity: Severity::Warning,
                    title: format!("UUID no fstab não corresponde a nenhum dispositivo detectado: {}", uuid_spec),
                    description: format!("A entrada para {} referencia UUID {} que não foi encontrado nos dispositivos da instalação candidata.", entry.mount_point.display(), uuid_spec),
                    evidence: vec![Evidence {
                        kind: EvidenceKind::ConfigValueMismatch,
                        description: "UUID desconhecido".to_string(),
                        path: Some(root.join("etc/fstab")),
                        expected: Some("UUID de dispositivo conhecido".to_string()),
                        observed: Some(uuid_spec.clone()),
                        raw_data: HashMap::from([
                            ("mount_point".to_string(), entry.mount_point.display().to_string()),
                            ("known_uuids".to_string(), known_uuids.keys().cloned().collect::<Vec<_>>().join(", ")),
                        ]),
                    }],
                    recommendations: vec![Recommendation {
                        id: "verify_unknown_uuid".to_string(),
                        description: "Verificar se o UUID corresponde a outro dispositivo (ex: swap, home separado) ou se é erro.".to_string(),
                        action: "blkid | grep <UUID> para identificar dispositivo".to_string(),
                        is_mutating: false,
                        risk: RiskLevel::Low,
                        prerequisites: vec![],
                    }],
                    verified_in_chroot: false,
                    context: HashMap::new(),
                });
            }
        }
    }

    Ok(findings)
}

/// Verifica subvolumes Btrfs no fstab.
fn check_fstab_btrfs_subvolumes(
    entries: &[FstabEntry],
    candidate: &LinuxInstallationCandidate,
    root: &Path,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    if !candidate.is_btrfs_subvolume {
        return Ok(findings);
    }

    let expected_subvol = candidate.btrfs_subvolume.as_deref().unwrap_or("@");

    for entry in entries {
        if entry.fs_type == FilesystemType::Btrfs
            && (*entry.mount_point == PathBuf::from("/") || entry.mount_point.starts_with("/home"))
        {
            // Extrai opção subvol
            let subvol_opt = entry
                .options
                .other
                .iter()
                .find(|o| o.starts_with("subvol=") || o.starts_with("subvolid="));

            if let Some(subvol) = subvol_opt {
                let subvol_value = subvol.split('=').nth(1).unwrap_or("");
                if *entry.mount_point == PathBuf::from("/")
                    && subvol_value != expected_subvol
                    && subvol_value != "@"
                {
                    findings.push(Finding {
                        id: DiagnosticId::FstabInvalidSubvolume,
                        severity: Severity::Warning,
                        title: format!("Subvolume Btrfs em {} pode ser incorreto: {}", entry.mount_point.display(), subvol_value),
                        description: format!("Entrada fstab para {} usa subvol={} mas o subvolume raiz esperado é {}. Verifique se corresponde à estrutura real.", entry.mount_point.display(), subvol_value, expected_subvol),
                        evidence: vec![Evidence {
                            kind: EvidenceKind::ConfigValueMismatch,
                            description: "Subvolume potencialmente incorreto".to_string(),
                            path: Some(root.join("etc/fstab")),
                            expected: Some(format!("subvol={}", expected_subvol)),
                            observed: Some(format!("subvol={}", subvol_value)),
                            raw_data: HashMap::new(),
                        }],
                        recommendations: vec![Recommendation {
                            id: "verify_btrfs_subvol".to_string(),
                            description: "Verificar subvolume real com btrfs subvolume list e corrigir fstab se necessário.".to_string(),
                            action: "btrfs subvolume list /mnt && editar /etc/fstab".to_string(),
                            is_mutating: true,
                            risk: RiskLevel::Medium,
                            prerequisites: vec!["Partição Btrfs montada".to_string()],
                        }],
                        verified_in_chroot: false,
                        context: HashMap::new(),
                    });
                }
            } else if *entry.mount_point == PathBuf::from("/") {
                findings.push(Finding {
                    id: DiagnosticId::FstabInvalidSubvolume,
                    severity: Severity::Warning,
                    title: "Entrada Btrfs raiz sem opção subvol no fstab".to_string(),
                    description: "Sistema usa Btrfs com subvolumes mas a entrada raiz no fstab não especifica subvol=. Isso pode montar o subvolume errado.".to_string(),
                    evidence: vec![Evidence {
                        kind: EvidenceKind::ConfigValueMismatch,
                        description: "subvol= ausente em entrada Btrfs raiz".to_string(),
                        path: Some(root.join("etc/fstab")),
                        expected: Some(format!("subvol={}", expected_subvol)),
                        observed: Some("Ausente".to_string()),
                        raw_data: HashMap::new(),
                    }],
                    recommendations: vec![Recommendation {
                        id: "add_btrfs_subvol".to_string(),
                        description: "Adicionar opção subvol= à entrada Btrfs raiz no fstab.".to_string(),
                        action: format!("Adicionar subvol={} às opções da entrada /", expected_subvol),
                        is_mutating: true,
                        risk: RiskLevel::Medium,
                        prerequisites: vec!["Confirmar subvolume raiz correto".to_string()],
                    }],
                    verified_in_chroot: false,
                    context: HashMap::new(),
                });
            }
        }
    }

    Ok(findings)
}

/// Verifica montagens atuais vs fstab.
fn check_current_mounts_vs_fstab(
    entries: &[FstabEntry],
    root: &Path,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // Lê /proc/mounts
    let mounts_content = fs::read_to_string("/proc/mounts").unwrap_or_default();

    for entry in entries {
        if *entry.mount_point == PathBuf::from("/")
            || *entry.mount_point == PathBuf::from("/boot")
            || *entry.mount_point == PathBuf::from("/boot/efi")
            || *entry.mount_point == PathBuf::from("/efi")
        {
            let mp = &entry.mount_point;

            // Verifica se está montado
            let is_mounted = mounts_content.lines().any(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                parts.len() >= 2 && Path::new(parts[1]) == mp.as_path()
            });

            if !is_mounted
                && !entry.options.other.contains(&"noauto".to_string())
                && !entry.options.noauto
            {
                findings.push(Finding {
                    id: DiagnosticId::MountpointMismatch,
                    severity: Severity::Warning,
                    title: format!("Ponto de montagem {} não está montado (fstab sem noauto)", mp.display()),
                    description: format!("O fstab especifica montagem em {} mas não está atualmente montado. Isso pode indicar problema se a montagem falhar no boot.", mp.display()),
                    evidence: vec![Evidence {
                        kind: EvidenceKind::MountMissing,
                        description: "Montagem fstab não ativa".to_string(),
                        path: Some(root.join(mp.strip_prefix("/").unwrap_or(mp))),
                        expected: Some("Montado".to_string()),
                        observed: Some("Não montado".to_string()),
                        raw_data: HashMap::from([
                            ("mount_point".to_string(), mp.display().to_string()),
                            ("fstab_options".to_string(), entry.options.other.join(",")),
                        ]),
                    }],
                    recommendations: vec![Recommendation {
                        id: "mount_missing_fstab".to_string(),
                        description: "Montar o filesystem ou investigar por que não montou.".to_string(),
                        action: format!("mount {} (ou verificar logs de boot)", mp.display()),
                        is_mutating: true,
                        risk: RiskLevel::Low,
                        prerequisites: vec!["Dispositivo disponível".to_string()],
                    }],
                    verified_in_chroot: false,
                    context: HashMap::new(),
                });
            }
        }
    }

    Ok(findings)
}

/// Extrai UUID de especificação de dispositivo (UUID, PARTUUID, LABEL, Path).
fn extract_uuid_from_spec(spec: &FstabDeviceSpec) -> Option<String> {
    match spec {
        FstabDeviceSpec::Uuid(u) => Some(u.clone()),
        FstabDeviceSpec::PartUuid(u) => Some(u.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_uuid_from_spec() {
        assert_eq!(
            extract_uuid_from_spec(&FstabDeviceSpec::Uuid("1234-5678".to_string())),
            Some("1234-5678".to_string())
        );
        assert_eq!(
            extract_uuid_from_spec(&FstabDeviceSpec::PartUuid(
                "12345678-1234-1234-1234-123456789abc".to_string()
            )),
            Some("12345678-1234-1234-1234-123456789abc".to_string())
        );
        assert_eq!(
            extract_uuid_from_spec(&FstabDeviceSpec::Path(PathBuf::from("/dev/sda1"))),
            None
        );
        assert_eq!(
            extract_uuid_from_spec(&FstabDeviceSpec::Label("boot".to_string())),
            None
        );
    }

    #[test]
    fn test_version_cmp() {
        use crate::diagnostic::kernel::version_cmp;
        assert_eq!(version_cmp("6.12.1", "6.11.5"), std::cmp::Ordering::Greater);
        assert_eq!(version_cmp("6.11.5", "6.12.1"), std::cmp::Ordering::Less);
        assert_eq!(
            version_cmp("6.12.1-arch1", "6.12.1-arch2"),
            std::cmp::Ordering::Less
        );
    }
}
