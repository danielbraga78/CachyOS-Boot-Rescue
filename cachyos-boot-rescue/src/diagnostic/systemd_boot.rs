//! Adaptador de diagnóstico do systemd-boot (somente leitura).
//!
//! Verifica presença de binário EFI, arquivos de entrada (.conf),
//! kernels e initramfs referenciados.

use crate::diagnostic::types::*;
use crate::discovery::types::LinuxInstallationCandidate;
use crate::error::Error;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Verifica a instalação do systemd-boot.
pub fn check_systemd_boot(
    environment: &BootEnvironment,
    _candidate: &LinuxInstallationCandidate,
    chroot_root: &Option<PathBuf>,
    _config: &DiagnosticConfig,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // systemd-boot requer UEFI
    if environment.firmware_mode != FirmwareMode::Uefi {
        findings.push(Finding {
            id: DiagnosticId::BootloaderNotDetected,
            severity: Severity::Info,
            title: "systemd-boot não compatível com BIOS Legacy".to_string(),
            description:
                "O systemd-boot só suporta boot UEFI. Sistema atual está em modo BIOS/Legacy."
                    .to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::DeviceMismatch,
                description: "Firmware não é UEFI".to_string(),
                path: None,
                expected: Some("UEFI".to_string()),
                observed: Some(format!("{:?}", environment.firmware_mode)),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
        return Ok(findings);
    }

    // Determina caminhos base
    let (_boot_path, efi_path) = resolve_systemd_boot_paths(environment, chroot_root);

    // 1. Verifica binário systemd-boot EFI
    findings.extend(check_systemd_boot_efi_binary(&efi_path)?);

    // 2. Verifica loader.conf
    findings.extend(check_loader_conf(&efi_path, chroot_root)?);

    // 3. Verifica entradas de boot (.conf em loader/entries/)
    findings.extend(check_boot_entries(&efi_path, environment, chroot_root)?);

    // 4. Verifica kernels e initramfs referenciados nas entradas
    findings.extend(check_systemd_boot_kernel_initramfs(
        &efi_path,
        environment,
        chroot_root,
    )?);

    Ok(findings)
}

/// Resolve caminhos para verificação do systemd-boot.
fn resolve_systemd_boot_paths(
    environment: &BootEnvironment,
    chroot_root: &Option<PathBuf>,
) -> (PathBuf, PathBuf) {
    let root = chroot_root.clone().unwrap_or_else(|| PathBuf::from("/"));

    // systemd-boot sempre usa a ESP
    let efi_path = if let Some(esp) = &environment.primary_esp {
        if let Some(mp) = &esp.mountpoint {
            root.join(mp.strip_prefix("/").unwrap_or(mp))
        } else {
            match environment.boot_layout {
                BootLayout::UnifiedBootOnEsp => root.join("boot"),
                BootLayout::EfiAtEfiBootSeparate => root.join("efi"),
                _ => root.join("boot/efi"),
            }
        }
    } else {
        match environment.boot_layout {
            BootLayout::UnifiedBootOnEsp => root.join("boot"),
            BootLayout::EfiAtEfiBootSeparate => root.join("efi"),
            _ => root.join("boot/efi"),
        }
    };

    let boot_path = match environment.boot_layout {
        BootLayout::UnifiedBootOnEsp => efi_path.clone(),
        _ => root.join("boot"),
    };

    (boot_path, efi_path)
}

/// Verifica presença do binário systemd-boot EFI.
fn check_systemd_boot_efi_binary(efi_path: &Path) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // systemd-boot instala como EFI/systemd/systemd-bootx64.efi
    // e também pode copiar para EFI/BOOT/BOOTX64.EFI
    let systemd_efi_paths = vec![
        efi_path.join("EFI/systemd/systemd-bootx64.efi"),
        efi_path.join("EFI/BOOT/BOOTX64.EFI"),
    ];

    let mut found = false;
    let mut found_path = None;

    for path in &systemd_efi_paths {
        if path.exists() {
            found = true;
            found_path = Some(path.clone());
            break;
        }
    }

    if !found {
        findings.push(Finding {
            id: DiagnosticId::BootloaderNotDetected,
            severity: Severity::Error,
            title: "Binário systemd-boot EFI não encontrado".to_string(),
            description: "Nenhum binário systemd-boot (systemd-bootx64.efi) foi encontrado nos caminhos esperados. O sistema UEFI não conseguirá carregar o systemd-boot.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::FileMissing,
                description: "Binário systemd-boot EFI ausente".to_string(),
                path: Some(efi_path.to_path_buf()),
                expected: Some("EFI/systemd/systemd-bootx64.efi".to_string()),
                observed: Some("Nenhum arquivo encontrado".to_string()),
                raw_data: HashMap::from([
                    ("checked_paths".to_string(), format!("{:?}", systemd_efi_paths)),
                ]),
            }],
            recommendations: vec![
                Recommendation {
                    id: "systemd_boot_install".to_string(),
                    description: "Instalar systemd-boot na ESP com bootctl.".to_string(),
                    action: "Executar bootctl install --path=<ESP>".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::High,
                    prerequisites: vec!["ESP montada".to_string(), "Chroot preparado".to_string(), "Backup da ESP".to_string(), "Pacote systemd instalado".to_string()],
                },
            ],
            verified_in_chroot: false,
            context: HashMap::new(),
        });
    } else {
        let info_path = found_path.clone().unwrap_or_else(|| efi_path.to_path_buf());
        findings.push(Finding {
            id: DiagnosticId::BootloaderNotDetected, // Info
            severity: Severity::Info,
            title: "Binário systemd-boot EFI encontrado".to_string(),
            description: format!(
                "Binário systemd-boot EFI presente em: {}",
                info_path.display()
            ),
            evidence: vec![Evidence {
                kind: EvidenceKind::FileExists,
                description: "Binário systemd-boot EFI encontrado".to_string(),
                path: Some(info_path),
                expected: Some("systemd-bootx64.efi".to_string()),
                observed: Some("Encontrado".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![],
            verified_in_chroot: false,
            context: HashMap::new(),
        });
    }

    Ok(findings)
}

/// Verifica loader.conf.
fn check_loader_conf(
    efi_path: &Path,
    chroot_root: &Option<PathBuf>,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    let loader_conf_path = efi_path.join("loader/loader.conf");

    if !loader_conf_path.exists() {
        findings.push(Finding {
            id: DiagnosticId::BootloaderNotDetected,
            severity: Severity::Warning,
            title: "loader.conf não encontrado".to_string(),
            description: "O arquivo loader/loader.conf não existe. O systemd-boot usará configurações padrão (timeout 10s, entrada padrão pela ordem).".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::FileMissing,
                description: "loader.conf ausente".to_string(),
                path: Some(loader_conf_path.clone()),
                expected: Some("loader/loader.conf".to_string()),
                observed: Some("Não encontrado".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![Recommendation {
                id: "create_loader_conf".to_string(),
                description: "Criar loader.conf com configurações desejadas.".to_string(),
                action: "Criar /efi/loader/loader.conf com timeout, default, editor".to_string(),
                is_mutating: true,
                risk: RiskLevel::Low,
                prerequisites: vec!["ESP montada".to_string()],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
        return Ok(findings);
    }

    // Lê e valida loader.conf
    let content = fs::read_to_string(&loader_conf_path).unwrap_or_default();
    let mut issues = Vec::new();

    // Verifica timeout
    if !content.lines().any(|l| l.trim().starts_with("timeout ")) {
        issues.push("timeout não definido".to_string());
    }

    // Verifica default
    if !content.lines().any(|l| l.trim().starts_with("default ")) {
        issues.push("default não definido".to_string());
    }

    if !issues.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::BootloaderNotDetected,
            severity: Severity::Info,
            title: "loader.conf encontrado com configurações parciais".to_string(),
            description: format!("loader.conf existe mas: {}", issues.join(", ")),
            evidence: vec![Evidence {
                kind: EvidenceKind::ContentMismatch,
                description: "Configurações opcionais ausentes".to_string(),
                path: Some(loader_conf_path),
                expected: Some("timeout e default definidos".to_string()),
                observed: Some(issues.join(", ")),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![Recommendation {
                id: "complete_loader_conf".to_string(),
                description: "Completar loader.conf com timeout e default.".to_string(),
                action: "Editar loader.conf adicionando timeout e default".to_string(),
                is_mutating: true,
                risk: RiskLevel::Low,
                prerequisites: vec!["ESP montada".to_string()],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    } else {
        findings.push(Finding {
            id: DiagnosticId::BootloaderNotDetected, // Info
            severity: Severity::Info,
            title: "loader.conf encontrado e completo".to_string(),
            description: "Arquivo loader.conf presente com timeout e default definidos."
                .to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::ContentMatches,
                description: "loader.conf válido".to_string(),
                path: Some(loader_conf_path),
                expected: Some("timeout e default".to_string()),
                observed: Some("OK".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    Ok(findings)
}

/// Verifica entradas de boot em loader/entries/.
fn check_boot_entries(
    efi_path: &Path,
    _environment: &BootEnvironment,
    chroot_root: &Option<PathBuf>,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    let entries_dir = efi_path.join("loader/entries");

    if !entries_dir.exists() {
        findings.push(Finding {
            id: DiagnosticId::BootloaderNotDetected,
            severity: Severity::Error,
            title: "Diretório loader/entries não encontrado".to_string(),
            description: "O diretório loader/entries não existe. Não há entradas de boot configuradas para o systemd-boot.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::DirectoryMissing,
                description: "loader/entries ausente".to_string(),
                path: Some(entries_dir.clone()),
                expected: Some("loader/entries/ com arquivos .conf".to_string()),
                observed: Some("Diretório não existe".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![Recommendation {
                id: "create_boot_entries".to_string(),
                description: "Criar arquivos de entrada em loader/entries/ para cada kernel.".to_string(),
                action: "Criar .conf files em loader/entries/ com title, linux, initrd, options".to_string(),
                is_mutating: true,
                risk: RiskLevel::Medium,
                prerequisites: vec!["Kernel instalado".to_string(), "Initramfs disponível".to_string(), "ESP montada".to_string()],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
        return Ok(findings);
    }

    // Lista arquivos .conf
    let entries: Vec<PathBuf> = fs::read_dir(&entries_dir)
        .map_err(|e| Error::internal(format!("Erro ao ler diretório: {e}")))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "conf"))
        .map(|e| e.path())
        .collect();

    if entries.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::BootloaderNotDetected,
            severity: Severity::Error,
            title: "Nenhuma entrada de boot (.conf) em loader/entries".to_string(),
            description: "O diretório loader/entries existe mas não contém arquivos .conf. Não há entradas de boot configuradas.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::FileMissing,
                description: "Arquivos .conf ausentes".to_string(),
                path: Some(entries_dir),
                expected: Some("Pelo menos um arquivo .conf".to_string()),
                observed: Some("Diretório vazio".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![Recommendation {
                id: "create_boot_entries".to_string(),
                description: "Criar arquivos de entrada em loader/entries/ para cada kernel.".to_string(),
                action: "Criar .conf files em loader/entries/ com title, linux, initrd, options".to_string(),
                is_mutating: true,
                risk: RiskLevel::Medium,
                prerequisites: vec!["Kernel instalado".to_string(), "Initramfs disponível".to_string(), "ESP montada".to_string()],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
        return Ok(findings);
    }

    findings.push(Finding {
        id: DiagnosticId::BootloaderNotDetected, // Info
        severity: Severity::Info,
        title: format!("{} entrada(s) de boot encontrada(s)", entries.len()),
        description: format!(
            "Arquivos de entrada em loader/entries/: {}",
            entries
                .iter()
                .map(|e| e.file_name().unwrap().to_string_lossy())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        evidence: vec![Evidence {
            kind: EvidenceKind::FileExists,
            description: "Entradas de boot presentes".to_string(),
            path: Some(entries_dir),
            expected: Some("Arquivos .conf".to_string()),
            observed: Some(format!("{} arquivos", entries.len())),
            raw_data: HashMap::new(),
        }],
        recommendations: vec![],
        verified_in_chroot: chroot_root.is_some(),
        context: HashMap::new(),
    });

    Ok(findings)
}

/// Verifica kernels e initramfs referenciados nas entradas.
fn check_systemd_boot_kernel_initramfs(
    efi_path: &Path,
    environment: &BootEnvironment,
    chroot_root: &Option<PathBuf>,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    let entries_dir = efi_path.join("loader/entries");
    if !entries_dir.exists() {
        return Ok(findings);
    }

    let root = chroot_root.clone().unwrap_or_else(|| PathBuf::from("/"));

    let entries: Vec<PathBuf> = fs::read_dir(&entries_dir)
        .map_err(|e| Error::internal(format!("Erro ao ler diretório: {e}")))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "conf"))
        .map(|e| e.path())
        .collect();

    let mut missing_kernels = Vec::new();
    let mut missing_initramfs = Vec::new();

    for entry_path in &entries {
        let content = fs::read_to_string(entry_path).unwrap_or_default();

        // Extrai linux e initrd
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("linux ") {
                let kernel_rel = trimmed.strip_prefix("linux ").unwrap().trim();
                let full_path =
                    resolve_systemd_boot_path(kernel_rel, &root, &environment.boot_layout);
                if !full_path.exists() {
                    missing_kernels.push((
                        entry_path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        kernel_rel.to_string(),
                        full_path.display().to_string(),
                    ));
                }
            } else if trimmed.starts_with("initrd ") {
                let initrd_rel = trimmed.strip_prefix("initrd ").unwrap().trim();
                let full_path =
                    resolve_systemd_boot_path(initrd_rel, &root, &environment.boot_layout);
                if !full_path.exists() {
                    missing_initramfs.push((
                        entry_path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        initrd_rel.to_string(),
                        full_path.display().to_string(),
                    ));
                }
            }
        }
    }

    if !missing_kernels.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::KernelNotFound,
            severity: Severity::Critical,
            title: "Kernel(s) referenciado(s) nas entradas systemd-boot não encontrado(s)"
                .to_string(),
            description: format!(
                "{} entrada(s) de kernel apontam para arquivos inexistentes.",
                missing_kernels.len()
            ),
            evidence: missing_kernels
                .iter()
                .map(|(entry, ref_path, full_path)| Evidence {
                    kind: EvidenceKind::FileMissing,
                    description: format!("Entrada '{}': kernel {}", entry, ref_path),
                    path: Some(PathBuf::from(full_path)),
                    expected: Some("Arquivo de kernel existente".to_string()),
                    observed: Some("Arquivo não encontrado".to_string()),
                    raw_data: HashMap::new(),
                })
                .collect(),
            recommendations: vec![Recommendation {
                id: "systemd_boot_fix_kernel".to_string(),
                description: "Corrigir caminhos do kernel nas entradas ou instalar kernel."
                    .to_string(),
                action: "Editar arquivos em loader/entries/ ou instalar kernel".to_string(),
                is_mutating: true,
                risk: RiskLevel::High,
                prerequisites: vec!["Kernel instalado".to_string()],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    if !missing_initramfs.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::InitramfsNotFound,
            severity: Severity::Critical,
            title: "Initramfs referenciado(s) nas entradas systemd-boot não encontrado(s)"
                .to_string(),
            description: format!(
                "{} entrada(s) de initramfs apontam para arquivos inexistentes.",
                missing_initramfs.len()
            ),
            evidence: missing_initramfs
                .iter()
                .map(|(entry, ref_path, full_path)| Evidence {
                    kind: EvidenceKind::FileMissing,
                    description: format!("Entrada '{}': initrd {}", entry, ref_path),
                    path: Some(PathBuf::from(full_path)),
                    expected: Some("Arquivo initramfs existente".to_string()),
                    observed: Some("Arquivo não encontrado".to_string()),
                    raw_data: HashMap::new(),
                })
                .collect(),
            recommendations: vec![Recommendation {
                id: "systemd_boot_fix_initrd".to_string(),
                description: "Corrigir caminhos do initramfs nas entradas ou regenerar initramfs."
                    .to_string(),
                action: "Editar arquivos em loader/entries/ ou executar mkinitcpio -P".to_string(),
                is_mutating: true,
                risk: RiskLevel::High,
                prerequisites: vec!["Kernel instalado".to_string()],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    if missing_kernels.is_empty() && missing_initramfs.is_empty() && !entries.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::KernelNotFound, // Info
            severity: Severity::Info,
            title: "Kernels e initramfs das entradas systemd-boot encontrados".to_string(),
            description:
                "Todos os kernels e initramfs referenciados nas entradas existem no filesystem."
                    .to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::ContentMatches,
                description: "Caminhos válidos em todas as entradas".to_string(),
                path: Some(entries_dir),
                expected: Some("Arquivos existentes".to_string()),
                observed: Some("OK".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    Ok(findings)
}

/// Resolve caminho do systemd-boot para caminho absoluto.
fn resolve_systemd_boot_path(rel_path: &str, root: &Path, boot_layout: &BootLayout) -> PathBuf {
    // systemd-boot paths são relativos à raiz da ESP
    if rel_path.starts_with('/') {
        match boot_layout {
            BootLayout::UnifiedBootOnEsp => root
                .join("boot")
                .join(rel_path.strip_prefix('/').unwrap_or(rel_path)),
            _ => root.join(rel_path.strip_prefix('/').unwrap_or(rel_path)),
        }
    } else {
        root.join("boot").join(rel_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_resolve_systemd_boot_path() {
        let root = PathBuf::from("/mnt");

        // Layout unificado
        assert_eq!(
            resolve_systemd_boot_path("/vmlinuz-linux", &root, &BootLayout::UnifiedBootOnEsp),
            PathBuf::from("/mnt/boot/vmlinuz-linux")
        );

        // Layout separado
        assert_eq!(
            resolve_systemd_boot_path("/vmlinuz-linux", &root, &BootLayout::SeparateBootEfi),
            PathBuf::from("/mnt/vmlinuz-linux")
        );

        // Relativo
        assert_eq!(
            resolve_systemd_boot_path("vmlinuz-linux", &root, &BootLayout::UnifiedBootOnEsp),
            PathBuf::from("/mnt/boot/vmlinuz-linux")
        );
    }
}
