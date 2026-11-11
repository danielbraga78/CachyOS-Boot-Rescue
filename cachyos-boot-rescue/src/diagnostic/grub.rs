//! Adaptador de diagnóstico do GRUB (somente leitura).
//!
//! Verifica presença de binário EFI, configuração grub.cfg, diretórios,
//! referências a UUIDs, kernels e initramfs.

use crate::diagnostic::types::*;
use crate::discovery::types::LinuxInstallationCandidate;
use crate::error::Error;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Verifica a instalação do GRUB.
pub fn check_grub(
    environment: &BootEnvironment,
    candidate: &LinuxInstallationCandidate,
    chroot_root: &Option<PathBuf>,
    _config: &DiagnosticConfig,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // Determina caminhos base
    let (boot_path, efi_path) = resolve_boot_efi_paths(environment, chroot_root);

    // 1. Verifica binário GRUB EFI
    findings.extend(check_grub_efi_binary(
        &efi_path,
        environment.firmware_mode,
        chroot_root,
    )?);

    // 2. Verifica grub.cfg
    findings.extend(check_grub_config(&boot_path, &efi_path, chroot_root)?);

    // 3. Verifica diretório de módulos GRUB
    findings.extend(check_grub_modules(&boot_path, &efi_path)?);

    // 4. Verifica referências UUID no grub.cfg
    findings.extend(check_grub_uuid_references(
        &boot_path,
        &efi_path,
        candidate,
        chroot_root,
    )?);

    // 5. Verifica kernels e initramfs referenciados
    findings.extend(check_grub_kernel_initramfs_refs(
        &boot_path,
        &efi_path,
        environment,
        chroot_root,
    )?);

    // 6. Verifica entrada padrão
    findings.extend(check_grub_default_entry(&boot_path, &efi_path)?);

    // 7. Verifica compatibilidade UEFI/BIOS
    findings.extend(check_grub_firmware_compatibility(
        &efi_path,
        environment.firmware_mode,
        chroot_root,
    )?);

    Ok(findings)
}

/// Resolve caminhos de /boot e /boot/efi (ou /efi) considerando chroot.
fn resolve_boot_efi_paths(
    environment: &BootEnvironment,
    chroot_root: &Option<PathBuf>,
) -> (PathBuf, PathBuf) {
    let root = chroot_root.clone().unwrap_or_else(|| PathBuf::from("/"));

    let boot_path = match environment.boot_layout {
        BootLayout::UnifiedBootOnEsp => root.join("boot"),
        BootLayout::SeparateBootEfi | BootLayout::EfiAtEfiBootSeparate => root.join("boot"),
        BootLayout::BiosLegacySeparateBoot => root.join("boot"),
        BootLayout::BiosLegacyBootOnRoot => root.join("boot"),
        BootLayout::Unknown => root.join("boot"),
    };

    let efi_path = if let Some(esp) = &environment.primary_esp {
        if let Some(mp) = &esp.mountpoint {
            root.join(mp.strip_prefix("/").unwrap_or(mp))
        } else {
            // Tenta caminhos padrão baseados no layout
            match environment.boot_layout {
                BootLayout::SeparateBootEfi => root.join("boot/efi"),
                BootLayout::EfiAtEfiBootSeparate => root.join("efi"),
                BootLayout::UnifiedBootOnEsp => root.join("boot"),
                _ => root.join("boot/efi"),
            }
        }
    } else {
        match environment.boot_layout {
            BootLayout::SeparateBootEfi => root.join("boot/efi"),
            BootLayout::EfiAtEfiBootSeparate => root.join("efi"),
            BootLayout::UnifiedBootOnEsp => root.join("boot"),
            _ => root.join("boot/efi"),
        }
    };

    (boot_path, efi_path)
}

/// Verifica presença do binário GRUB EFI.
fn check_grub_efi_binary(
    efi_path: &Path,
    firmware_mode: FirmwareMode,
    chroot_root: &Option<PathBuf>,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // Caminhos típicos do GRUB EFI
    let grub_efi_paths = vec![
        efi_path.join("EFI/grub/grubx64.efi"),
        efi_path.join("EFI/GRUB/grubx64.efi"),
        efi_path.join("EFI/BOOT/BOOTX64.EFI"), // fallback genérico
        efi_path.join("EFI/cachyos/grubx64.efi"),
        efi_path.join("EFI/arch/grubx64.efi"),
    ];

    let mut found = false;
    let mut found_path = None;

    for path in &grub_efi_paths {
        if path.exists() {
            found = true;
            found_path = Some(path.clone());
            break;
        }
    }

    if !found && firmware_mode == FirmwareMode::Uefi {
        findings.push(Finding {
            id: DiagnosticId::GrubEfiBinaryMissing,
            severity: Severity::Error,
            title: "Binário GRUB EFI não encontrado".to_string(),
            description: "Nenhum binário GRUB EFI (grubx64.efi) foi encontrado nos caminhos esperados da ESP. O sistema UEFI não conseguirá carregar o GRUB.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::FileMissing,
                description: "Binário GRUB EFI ausente nos caminhos padrão".to_string(),
                path: Some(efi_path.to_path_buf()),
                expected: Some("EFI/grub/grubx64.efi ou similar".to_string()),
                observed: Some("Nenhum arquivo encontrado".to_string()),
                raw_data: HashMap::from([
                    ("checked_paths".to_string(), format!("{:?}", grub_efi_paths)),
                    ("firmware_mode".to_string(), format!("{:?}", firmware_mode)),
                ]),
            }],
            recommendations: vec![
                Recommendation {
                    id: "grub_install_efi".to_string(),
                    description: "Instalar o GRUB na ESP com grub-install para criar o binário EFI e registrar entrada NVRAM.".to_string(),
                    action: "Executar grub-install --target=x86_64-efi --efi-directory=<ESP> --bootloader-id=GRUB".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::High,
                    prerequisites: vec!["ESP montada".to_string(), "Chroot preparado".to_string(), "Backup da ESP".to_string()],
                },
                Recommendation {
                    id: "grub_copy_fallback".to_string(),
                    description: "Como alternativa, copiar grubx64.efi para EFI/BOOT/BOOTX64.EFI como fallback genérico UEFI.".to_string(),
                    action: "Copiar grubx64.efi para EFI/BOOT/BOOTX64.EFI".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::Medium,
                    prerequisites: vec!["ESP montada".to_string(), "Backup da ESP".to_string()],
                },
            ],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    } else if found {
        let info_path = found_path.clone().unwrap_or_else(|| efi_path.to_path_buf());
        findings.push(Finding {
            id: DiagnosticId::GrubEfiBinaryMissing, // Usa mesmo ID mas severidade Info
            severity: Severity::Info,
            title: "Binário GRUB EFI encontrado".to_string(),
            description: format!("Binário GRUB EFI presente em: {}", info_path.display()),
            evidence: vec![Evidence {
                kind: EvidenceKind::FileExists,
                description: "Binário GRUB EFI encontrado".to_string(),
                path: Some(info_path),
                expected: Some("grubx64.efi".to_string()),
                observed: Some("Encontrado".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    Ok(findings)
}

/// Verifica presença e validade do grub.cfg.
fn check_grub_config(
    boot_path: &Path,
    efi_path: &Path,
    chroot_root: &Option<PathBuf>,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // Caminhos típicos do grub.cfg
    let config_paths = vec![
        boot_path.join("grub/grub.cfg"),
        boot_path.join("grub2/grub.cfg"),
        efi_path.join("EFI/grub/grub.cfg"),
        efi_path.join("EFI/GRUB/grub.cfg"),
        efi_path.join("EFI/cachyos/grub.cfg"),
    ];

    let mut found_config = None;
    let mut config_content = String::new();

    for path in &config_paths {
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) if !content.trim().is_empty() => {
                    found_config = Some(path.clone());
                    config_content = content;
                    break;
                }
                Ok(_) => {
                    // Arquivo vazio
                    findings.push(Finding {
                        id: DiagnosticId::GrubConfigInvalid,
                        severity: Severity::Error,
                        title: "grub.cfg vazio".to_string(),
                        description: format!("O arquivo {} existe mas está vazio.", path.display()),
                        evidence: vec![Evidence {
                            kind: EvidenceKind::ContentMismatch,
                            description: "grub.cfg existe mas está vazio".to_string(),
                            path: Some(path.clone()),
                            expected: Some("Conteúdo de configuração GRUB".to_string()),
                            observed: Some("Arquivo vazio".to_string()),
                            raw_data: HashMap::new(),
                        }],
                        recommendations: vec![Recommendation {
                            id: "regenerate_grub_cfg".to_string(),
                            description: "Regenerar grub.cfg com grub-mkconfig.".to_string(),
                            action: "Executar grub-mkconfig -o /boot/grub/grub.cfg".to_string(),
                            is_mutating: true,
                            risk: RiskLevel::Medium,
                            prerequisites: vec!["Chroot preparado".to_string()],
                        }],
                        verified_in_chroot: chroot_root.is_some(),
                        context: HashMap::new(),
                    });
                    return Ok(findings);
                }
                Err(e) => {
                    findings.push(Finding {
                        id: DiagnosticId::GrubConfigInvalid,
                        severity: Severity::Warning,
                        title: "Erro ao ler grub.cfg".to_string(),
                        description: format!("Erro ao ler {}: {}", path.display(), e),
                        evidence: vec![Evidence {
                            kind: EvidenceKind::CommandFailed,
                            description: "Falha na leitura do arquivo".to_string(),
                            path: Some(path.clone()),
                            expected: Some("Leitura bem-sucedida".to_string()),
                            observed: Some(e.to_string()),
                            raw_data: HashMap::new(),
                        }],
                        recommendations: vec![],
                        verified_in_chroot: chroot_root.is_some(),
                        context: HashMap::new(),
                    });
                }
            }
        }
    }

    if found_config.is_none() {
        findings.push(Finding {
            id: DiagnosticId::GrubConfigMissing,
            severity: Severity::Error,
            title: "grub.cfg não encontrado".to_string(),
            description: "Nenhum arquivo grub.cfg foi encontrado nos caminhos esperados. O GRUB não terá entradas de boot para exibir.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::FileMissing,
                description: "grub.cfg ausente em todos os caminhos padrão".to_string(),
                path: Some(boot_path.join("grub/grub.cfg")),
                expected: Some("/boot/grub/grub.cfg ou similar".to_string()),
                observed: Some("Nenhum arquivo encontrado".to_string()),
                raw_data: HashMap::from([
                    ("checked_paths".to_string(), format!("{:?}", config_paths)),
                ]),
            }],
            recommendations: vec![Recommendation {
                id: "regenerate_grub_cfg".to_string(),
                description: "Gerar grub.cfg com grub-mkconfig.".to_string(),
                action: "Executar grub-mkconfig -o /boot/grub/grub.cfg".to_string(),
                is_mutating: true,
                risk: RiskLevel::Medium,
                prerequisites: vec!["Chroot preparado".to_string(), "Kernel instalado".to_string()],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
        return Ok(findings);
    }

    let config_path = found_config.unwrap();

    // Validação básica de sintaxe do grub.cfg
    let syntax_issues = validate_grub_config_syntax(&config_content);
    if !syntax_issues.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::GrubConfigInvalid,
            severity: Severity::Warning,
            title: "Possíveis problemas de sintaxe no grub.cfg".to_string(),
            description: format!(
                "Foram detectados {} problema(s) potencial(is) no grub.cfg: {}",
                syntax_issues.len(),
                syntax_issues.join("; ")
            ),
            evidence: vec![Evidence {
                kind: EvidenceKind::ContentMismatch,
                description: "Problemas de sintaxe detectados".to_string(),
                path: Some(config_path.clone()),
                expected: Some("Sintaxe válida".to_string()),
                observed: Some(syntax_issues.join("; ")),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![Recommendation {
                id: "review_grub_cfg".to_string(),
                description: "Revisar manualmente o grub.cfg e regenerar se necessário."
                    .to_string(),
                action: "Verificar grub.cfg e executar grub-mkconfig se necessário".to_string(),
                is_mutating: false,
                risk: RiskLevel::Low,
                prerequisites: vec![],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    } else {
        findings.push(Finding {
            id: DiagnosticId::GrubConfigMissing, // Info
            severity: Severity::Info,
            title: "grub.cfg encontrado e sintaticamente válido".to_string(),
            description: format!(
                "Arquivo de configuração GRUB presente em: {}",
                config_path.display()
            ),
            evidence: vec![Evidence {
                kind: EvidenceKind::ContentMatches,
                description: "grub.cfg encontrado e válido".to_string(),
                path: Some(config_path),
                expected: Some("Configuração válida".to_string()),
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

/// Validação básica de sintaxe do grub.cfg.
fn validate_grub_config_syntax(content: &str) -> Vec<String> {
    let mut issues = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut menuentry_count = 0;
    let mut has_default = false;
    let mut brace_depth = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Conta menuentry
        if trimmed.starts_with("menuentry ") || trimmed.starts_with("submenu ") {
            menuentry_count += 1;
        }

        // Verifica set default
        if trimmed.starts_with("set default=") || trimmed.contains("default=") {
            has_default = true;
        }

        // Verifica chaves balanceadas (básico)
        for ch in trimmed.chars() {
            if ch == '{' {
                brace_depth += 1;
            } else if ch == '}' {
                if brace_depth == 0 {
                    issues.push(format!("Chave '}}' não balanceada na linha {}", i + 1));
                } else {
                    brace_depth -= 1;
                }
            }
        }
    }

    if brace_depth != 0 {
        issues.push(format!(
            "Chaves não balanceadas: {} abertas não fechadas",
            brace_depth
        ));
    }

    if menuentry_count == 0 {
        issues.push("Nenhuma entrada 'menuentry' encontrada".to_string());
    }

    if !has_default {
        issues.push("Variável 'default' não definida".to_string());
    }

    issues
}

/// Verifica diretório de módulos GRUB.
fn check_grub_modules(boot_path: &Path, efi_path: &Path) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // Caminhos típicos de módulos
    let module_dirs = vec![
        boot_path.join("grub/x86_64-efi"),
        boot_path.join("grub2/x86_64-efi"),
        boot_path.join("grub/i386-pc"), // BIOS
        efi_path.join("EFI/grub/x86_64-efi"),
    ];

    let mut found_modules = false;
    let mut found_dir = None;

    for dir in &module_dirs {
        if dir.exists() && dir.is_dir() {
            // Verifica se tem módulos essenciais
            let essential_modules = [
                "normal.mod",
                "linux.mod",
                "initrd.mod",
                "efi_gop.mod",
                "efi_uga.mod",
            ];
            let mut missing = Vec::new();

            for module in essential_modules {
                if !dir.join(module).exists() {
                    missing.push(module);
                }
            }

            if missing.is_empty() {
                found_modules = true;
                found_dir = Some(dir.clone());
                break;
            } else {
                findings.push(Finding {
                    id: DiagnosticId::GrubModulesMissing,
                    severity: Severity::Warning,
                    title: "Módulos GRUB essenciais ausentes".to_string(),
                    description: format!(
                        "Diretório de módulos {} existe mas faltam: {}",
                        dir.display(),
                        missing.join(", ")
                    ),
                    evidence: vec![Evidence {
                        kind: EvidenceKind::FileMissing,
                        description: "Módulos GRUB essenciais não encontrados".to_string(),
                        path: Some(dir.clone()),
                        expected: Some(essential_modules.join(", ")),
                        observed: Some(format!("Faltando: {}", missing.join(", "))),
                        raw_data: HashMap::new(),
                    }],
                    recommendations: vec![Recommendation {
                        id: "reinstall_grub".to_string(),
                        description: "Reinstalar GRUB para restaurar módulos.".to_string(),
                        action: "Executar grub-install novamente".to_string(),
                        is_mutating: true,
                        risk: RiskLevel::Medium,
                        prerequisites: vec![
                            "Chroot preparado".to_string(),
                            "Backup da ESP".to_string(),
                        ],
                    }],
                    verified_in_chroot: false,
                    context: HashMap::new(),
                });
            }
        }
    }

    if !found_modules {
        findings.push(Finding {
            id: DiagnosticId::GrubModulesMissing,
            severity: Severity::Error,
            title: "Diretório de módulos GRUB não encontrado".to_string(),
            description: "Nenhum diretório de módulos GRUB (x86_64-efi ou i386-pc) foi encontrado. O GRUB não conseguirá carregar módulos essenciais.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::DirectoryMissing,
                description: "Diretório de módulos GRUB ausente".to_string(),
                path: Some(boot_path.join("grub")),
                expected: Some("/boot/grub/x86_64-efi/ ou similar".to_string()),
                observed: Some("Nenhum diretório encontrado".to_string()),
                raw_data: HashMap::from([
                    ("checked_dirs".to_string(), format!("{:?}", module_dirs)),
                ]),
            }],
            recommendations: vec![Recommendation {
                id: "grub_install_modules".to_string(),
                description: "Executar grub-install para criar diretório de módulos e instalar binário EFI.".to_string(),
                action: "grub-install --target=x86_64-efi --efi-directory=<ESP> --bootloader-id=GRUB".to_string(),
                is_mutating: true,
                risk: RiskLevel::High,
                prerequisites: vec!["ESP montada".to_string(), "Chroot preparado".to_string(), "Backup da ESP".to_string()],
            }],
            verified_in_chroot: false,
            context: HashMap::new(),
        });
    } else {
        let info_dir = found_dir.clone().unwrap_or_else(|| boot_path.to_path_buf());
        findings.push(Finding {
            id: DiagnosticId::GrubModulesMissing, // Info
            severity: Severity::Info,
            title: "Módulos GRUB encontrados".to_string(),
            description: format!(
                "Diretório de módulos GRUB presente em: {}",
                info_dir.display()
            ),
            evidence: vec![Evidence {
                kind: EvidenceKind::DirectoryExists,
                description: "Módulos GRUB essenciais presentes".to_string(),
                path: Some(info_dir),
                expected: Some(
                    "normal.mod, linux.mod, initrd.mod, efi_gop.mod, efi_uga.mod".to_string(),
                ),
                observed: Some("Todos presentes".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![],
            verified_in_chroot: false,
            context: HashMap::new(),
        });
    }

    Ok(findings)
}

/// Verifica referências UUID no grub.cfg comparando com dispositivos reais.
fn check_grub_uuid_references(
    boot_path: &Path,
    efi_path: &Path,
    candidate: &LinuxInstallationCandidate,
    chroot_root: &Option<PathBuf>,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // Busca grub.cfg
    let config_paths = vec![
        boot_path.join("grub/grub.cfg"),
        boot_path.join("grub2/grub.cfg"),
        efi_path.join("EFI/grub/grub.cfg"),
        efi_path.join("EFI/GRUB/grub.cfg"),
    ];

    let mut config_content = String::new();
    for path in &config_paths {
        if let Ok(content) = fs::read_to_string(path) {
            if !content.trim().is_empty() {
                config_content = content;
                break;
            }
        }
    }

    if config_content.is_empty() {
        return Ok(findings); // Já reportado em check_grub_config
    }

    // Extrai UUIDs do grub.cfg (padrão search --fs-uuid --set=root XXXX)
    let uuid_pattern =
        regex::Regex::new(r#"search\s+.*--fs-uuid\s+--set=root\s+([a-fA-F0-9\-]{36})"#).unwrap();
    let uuid_pattern2 = regex::Regex::new(r#"search\s+.*--fs-uuid\s+([a-fA-F0-9\-]{36})"#).unwrap();
    let uuid_pattern3 = regex::Regex::new(r#"\$root\s*=\s*['"]([a-fA-F0-9\-]{36})['"]"#).unwrap();

    let mut grub_uuids = Vec::new();
    for cap in uuid_pattern.captures_iter(&config_content) {
        if let Some(uuid) = cap.get(1) {
            grub_uuids.push(uuid.as_str().to_lowercase());
        }
    }
    for cap in uuid_pattern2.captures_iter(&config_content) {
        if let Some(uuid) = cap.get(1) {
            grub_uuids.push(uuid.as_str().to_lowercase());
        }
    }
    for cap in uuid_pattern3.captures_iter(&config_content) {
        if let Some(uuid) = cap.get(1) {
            grub_uuids.push(uuid.as_str().to_lowercase());
        }
    }

    // Deduplica
    grub_uuids.sort();
    grub_uuids.dedup();

    if grub_uuids.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::GrubUuidsMismatch,
            severity: Severity::Warning,
            title: "Nenhuma referência UUID encontrada no grub.cfg".to_string(),
            description: "O grub.cfg não contém referências UUID explícitas (search --fs-uuid). Isso pode funcionar mas é menos robusto que usar UUIDs.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::ContentMismatch,
                description: "Ausência de search --fs-uuid no grub.cfg".to_string(),
                path: Some(boot_path.join("grub/grub.cfg")),
                expected: Some("search --fs-uuid --set=root <UUID>".to_string()),
                observed: Some("Não encontrado".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![Recommendation {
                id: "add_uuid_to_grub".to_string(),
                description: "Regenerar grub.cfg para incluir referências UUID automáticas.".to_string(),
                action: "Executar grub-mkconfig -o /boot/grub/grub.cfg".to_string(),
                is_mutating: true,
                risk: RiskLevel::Low,
                prerequisites: vec!["Chroot preparado".to_string()],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
        return Ok(findings);
    }

    // Compara com UUIDs reais
    let root_uuid = candidate
        .root_filesystem
        .uuid
        .as_ref()
        .map(|u| u.to_lowercase());
    let boot_uuids: Vec<String> = candidate
        .boot_partitions
        .iter()
        .filter_map(|b| b.filesystem.uuid.as_ref().map(|u| u.to_lowercase()))
        .collect();

    let mut mismatched = Vec::new();
    for grub_uuid in &grub_uuids {
        let matches_root = root_uuid.as_ref().is_some_and(|u| u == grub_uuid);
        let matches_boot = boot_uuids.iter().any(|u| u == grub_uuid);

        if !matches_root && !matches_boot {
            mismatched.push(grub_uuid.clone());
        }
    }

    if !mismatched.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::GrubUuidsMismatch,
            severity: Severity::Error,
            title: "UUIDs no grub.cfg não correspondem aos dispositivos".to_string(),
            description: format!("O grub.cfg referencia UUID(s) que não correspondem à partição raiz nem às partições de boot: {}", mismatched.join(", ")),
            evidence: vec![Evidence {
                kind: EvidenceKind::ConfigValueMismatch,
                description: "UUIDs divergentes".to_string(),
                path: Some(boot_path.join("grub/grub.cfg")),
                expected: Some(format!("Root UUID: {:?}, Boot UUIDs: {:?}", root_uuid, boot_uuids)),
                observed: Some(format!("GRUB UUIDs: {:?}", grub_uuids)),
                raw_data: HashMap::from([
                    ("grub_uuids".to_string(), grub_uuids.join(", ")),
                    ("root_uuid".to_string(), root_uuid.unwrap_or_default()),
                    ("boot_uuids".to_string(), boot_uuids.join(", ")),
                ]),
            }],
            recommendations: vec![
                Recommendation {
                    id: "regenerate_grub_cfg_uuid".to_string(),
                    description: "Regenerar grub.cfg para atualizar referências UUID.".to_string(),
                    action: "Executar grub-mkconfig -o /boot/grub/grub.cfg".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::Medium,
                    prerequisites: vec!["Chroot preparado".to_string(), "Partições montadas corretamente".to_string()],
                },
                Recommendation {
                    id: "manual_edit_grub_uuid".to_string(),
                    description: "Editar manualmente grub.cfg para corrigir UUIDs (não recomendado).".to_string(),
                    action: "Editar /boot/grub/grub.cfg e corrigir UUIDs".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::High,
                    prerequisites: vec!["Backup do grub.cfg".to_string()],
                },
            ],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    } else {
        findings.push(Finding {
            id: DiagnosticId::GrubUuidsMismatch, // Info
            severity: Severity::Info,
            title: "UUIDs no grub.cfg correspondem aos dispositivos".to_string(),
            description: format!(
                "Todas as {} referência(s) UUID no grub.cfg correspondem às partições detectadas.",
                grub_uuids.len()
            ),
            evidence: vec![Evidence {
                kind: EvidenceKind::ConfigValueMatches,
                description: "UUIDs consistentes".to_string(),
                path: Some(boot_path.join("grub/grub.cfg")),
                expected: Some("Correspondência com root/boot UUIDs".to_string()),
                observed: Some("OK".to_string()),
                raw_data: HashMap::from([("grub_uuids".to_string(), grub_uuids.join(", "))]),
            }],
            recommendations: vec![],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    Ok(findings)
}

/// Verifica kernels e initramfs referenciados no grub.cfg.
fn check_grub_kernel_initramfs_refs(
    boot_path: &Path,
    efi_path: &Path,
    _environment: &BootEnvironment,
    chroot_root: &Option<PathBuf>,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    let config_paths = vec![
        boot_path.join("grub/grub.cfg"),
        boot_path.join("grub2/grub.cfg"),
        efi_path.join("EFI/grub/grub.cfg"),
    ];

    let mut config_content = String::new();
    for path in &config_paths {
        if let Ok(content) = fs::read_to_string(path) {
            if !content.trim().is_empty() {
                config_content = content;
                break;
            }
        }
    }

    if config_content.is_empty() {
        return Ok(findings);
    }

    // Extrai linhas linux e initrd
    let linux_pattern = regex::Regex::new(r#"linux\s+(/?[^\s]+)"#).unwrap();
    let initrd_pattern = regex::Regex::new(r#"initrd\s+(/?[^\s]+)"#).unwrap();

    let mut missing_kernels = Vec::new();
    let mut missing_initramfs = Vec::new();

    for cap in linux_pattern.captures_iter(&config_content) {
        if let Some(path_match) = cap.get(1) {
            let kernel_path = path_match.as_str();
            let full_path = resolve_grub_path(kernel_path, boot_path, chroot_root);
            if !full_path.exists() {
                missing_kernels.push((kernel_path.to_string(), full_path.display().to_string()));
            }
        }
    }

    for cap in initrd_pattern.captures_iter(&config_content) {
        if let Some(path_match) = cap.get(1) {
            let initrd_path = path_match.as_str();
            let full_path = resolve_grub_path(initrd_path, boot_path, chroot_root);
            if !full_path.exists() {
                missing_initramfs.push((initrd_path.to_string(), full_path.display().to_string()));
            }
        }
    }

    if !missing_kernels.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::GrubKernelNotFound,
            severity: Severity::Critical,
            title: "Kernel(s) referenciado(s) no grub.cfg não encontrado(s)".to_string(),
            description: format!("{} entrada(s) de kernel no grub.cfg apontam para arquivos inexistentes.", missing_kernels.len()),
            evidence: missing_kernels.iter().map(|(ref_path, full_path)| Evidence {
                kind: EvidenceKind::FileMissing,
                description: format!("Kernel referenciado: {}", ref_path),
                path: Some(PathBuf::from(full_path)),
                expected: Some("Arquivo de kernel existente".to_string()),
                observed: Some("Arquivo não encontrado".to_string()),
                raw_data: HashMap::new(),
            }).collect(),
            recommendations: vec![
                Recommendation {
                    id: "install_kernel".to_string(),
                    description: "Instalar kernel correspondente ou regenerar grub.cfg para detectar kernels instalados.".to_string(),
                    action: "Instalar kernel (pacman -S linux) e regenerar grub.cfg".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::High,
                    prerequisites: vec!["Chroot preparado".to_string(), "Conexão de rede".to_string()],
                },
                Recommendation {
                    id: "regenerate_grub_kernel".to_string(),
                    description: "Regenerar grub.cfg para detectar kernels automaticamente.".to_string(),
                    action: "grub-mkconfig -o /boot/grub/grub.cfg".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::Medium,
                    prerequisites: vec!["Chroot preparado".to_string()],
                },
            ],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    if !missing_initramfs.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::GrubInitramfsNotFound,
            severity: Severity::Critical,
            title: "Initramfs referenciado(s) no grub.cfg não encontrado(s)".to_string(),
            description: format!(
                "{} entrada(s) de initramfs no grub.cfg apontam para arquivos inexistentes.",
                missing_initramfs.len()
            ),
            evidence: missing_initramfs
                .iter()
                .map(|(ref_path, full_path)| Evidence {
                    kind: EvidenceKind::FileMissing,
                    description: format!("Initramfs referenciado: {}", ref_path),
                    path: Some(PathBuf::from(full_path)),
                    expected: Some("Arquivo initramfs existente".to_string()),
                    observed: Some("Arquivo não encontrado".to_string()),
                    raw_data: HashMap::new(),
                })
                .collect(),
            recommendations: vec![
                Recommendation {
                    id: "regenerate_initramfs".to_string(),
                    description: "Regenerar initramfs com mkinitcpio.".to_string(),
                    action: "mkinitcpio -P".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::High,
                    prerequisites: vec![
                        "Chroot preparado".to_string(),
                        "Kernel instalado".to_string(),
                    ],
                },
                Recommendation {
                    id: "regenerate_grub_initramfs".to_string(),
                    description: "Regenerar grub.cfg após regenerar initramfs.".to_string(),
                    action: "grub-mkconfig -o /boot/grub/grub.cfg".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::Medium,
                    prerequisites: vec![
                        "Initramfs regenerado".to_string(),
                        "Chroot preparado".to_string(),
                    ],
                },
            ],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    if missing_kernels.is_empty() && missing_initramfs.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::GrubKernelNotFound, // Info
            severity: Severity::Info,
            title: "Kernels e initramfs referenciados no grub.cfg encontrados".to_string(),
            description:
                "Todos os kernels e initramfs referenciados no grub.cfg existem no filesystem."
                    .to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::ContentMatches,
                description: "Kernels e initramfs válidos".to_string(),
                path: Some(boot_path.join("grub/grub.cfg")),
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

/// Resolve caminho relativo do GRUB para caminho absoluto no filesystem.
fn resolve_grub_path(grub_path: &str, boot_path: &Path, chroot_root: &Option<PathBuf>) -> PathBuf {
    let root = chroot_root.clone().unwrap_or_else(|| PathBuf::from("/"));

    if grub_path.starts_with('/') {
        // Caminho absoluto do ponto de vista do GRUB (ex: /vmlinuz-linux)
        // Mapeia para /boot no filesystem
        root.join("boot")
            .join(grub_path.strip_prefix('/').unwrap_or(grub_path))
    } else {
        // Caminho relativo (ex: vmlinuz-linux)
        boot_path.join(grub_path)
    }
}

/// Verifica entrada padrão do GRUB.
fn check_grub_default_entry(boot_path: &Path, efi_path: &Path) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    let config_paths = vec![
        boot_path.join("grub/grub.cfg"),
        boot_path.join("grub2/grub.cfg"),
        efi_path.join("EFI/grub/grub.cfg"),
    ];

    let mut has_default = false;
    for path in &config_paths {
        if let Ok(content) = fs::read_to_string(path) {
            if content.contains("set default=") || content.contains("default=") {
                has_default = true;
                break;
            }
        }
    }

    if !has_default {
        findings.push(Finding {
            id: DiagnosticId::GrubDefaultEntryMissing,
            severity: Severity::Warning,
            title: "Entrada padrão (default) não definida no grub.cfg".to_string(),
            description: "A variável 'default' não está definida no grub.cfg. O GRUB usará a primeira entrada (índice 0) como padrão, o que pode não ser o kernel desejado.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::ConfigValueMismatch,
                description: "Variável 'default' ausente".to_string(),
                path: Some(boot_path.join("grub/grub.cfg")),
                expected: Some("set default=0 ou similar".to_string()),
                observed: Some("Não definido".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![Recommendation {
                id: "set_grub_default".to_string(),
                description: "Definir entrada padrão no GRUB via /etc/default/grub e regenerar.".to_string(),
                action: "Editar /etc/default/grub (GRUB_DEFAULT=0) e executar grub-mkconfig".to_string(),
                is_mutating: true,
                risk: RiskLevel::Low,
                prerequisites: vec!["Chroot preparado".to_string()],
            }],
            verified_in_chroot: false,
            context: HashMap::new(),
        });
    }

    Ok(findings)
}

/// Verifica compatibilidade GRUB com firmware (UEFI vs BIOS).
fn check_grub_firmware_compatibility(
    efi_path: &Path,
    firmware_mode: FirmwareMode,
    chroot_root: &Option<PathBuf>,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // Verifica se tem binário EFI
    let has_efi_binary = efi_path.join("EFI/grub/grubx64.efi").exists()
        || efi_path.join("EFI/GRUB/grubx64.efi").exists()
        || efi_path.join("EFI/BOOT/BOOTX64.EFI").exists();

    // Verifica se tem diretório BIOS (i386-pc)
    let has_bios_modules =
        efi_path.join("grub/i386-pc").exists() || efi_path.join("grub2/i386-pc").exists();

    match (firmware_mode, has_efi_binary, has_bios_modules) {
        (FirmwareMode::Uefi, false, true) => {
            findings.push(Finding {
                id: DiagnosticId::GrubBiosOnUefi,
                severity: Severity::Error,
                title: "GRUB BIOS instalado em sistema UEFI".to_string(),
                description: "Foram encontrados módulos GRUB para BIOS (i386-pc) mas nenhum binário EFI. O sistema UEFI não conseguirá carregar este GRUB.".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::DeviceMismatch,
                    description: "Módulos BIOS presentes, binário EFI ausente".to_string(),
                    path: Some(efi_path.to_path_buf()),
                    expected: Some("grubx64.efi".to_string()),
                    observed: Some("i386-pc modules only".to_string()),
                    raw_data: HashMap::new(),
                }],
                recommendations: vec![Recommendation {
                    id: "grub_install_efi_mode".to_string(),
                    description: "Reinstalar GRUB no modo UEFI (x86_64-efi).".to_string(),
                    action: "grub-install --target=x86_64-efi --efi-directory=<ESP> --bootloader-id=GRUB".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::High,
                    prerequisites: vec!["ESP montada".to_string(), "Chroot preparado".to_string(), "Backup da ESP".to_string()],
                }],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
        }
        (FirmwareMode::BiosLegacy, true, false) => {
            findings.push(Finding {
                id: DiagnosticId::GrubUefiOnBios,
                severity: Severity::Warning,
                title: "Binário GRUB EFI presente em sistema BIOS".to_string(),
                description: "Foi encontrado binário GRUB EFI mas o sistema iniciou em modo BIOS/Legacy. O binário EFI não será usado, mas módulos BIOS podem estar ausentes.".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::DeviceMismatch,
                    description: "Binário EFI presente, verificar módulos BIOS".to_string(),
                    path: Some(efi_path.to_path_buf()),
                    expected: Some("i386-pc modules".to_string()),
                    observed: Some("grubx64.efi found".to_string()),
                    raw_data: HashMap::new(),
                }],
                recommendations: vec![Recommendation {
                    id: "verify_bios_modules".to_string(),
                    description: "Verificar se módulos BIOS (i386-pc) estão instalados.".to_string(),
                    action: "Verificar /boot/grub/i386-pc/ e reinstalar GRUB se necessário".to_string(),
                    is_mutating: false,
                    risk: RiskLevel::Low,
                    prerequisites: vec![],
                }],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
        }
        _ => {}
    }

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_validate_grub_config_syntax() {
        // Config válida básica
        let valid = r#"
set default=0
menuentry "Linux" {
    linux /vmlinuz-linux root=UUID=xxx
    initrd /initramfs-linux.img
}
"#;
        assert!(validate_grub_config_syntax(valid).is_empty());

        // Sem menuentry
        let no_entry = "set default=0\n";
        let issues = validate_grub_config_syntax(no_entry);
        assert!(issues.iter().any(|i| i.contains("menuentry")));

        // Chaves não balanceadas
        let unbalanced = "menuentry \"Test\" {\n  linux /vmlinuz\n";
        let issues = validate_grub_config_syntax(unbalanced);
        assert!(issues.iter().any(|i| i.contains("balancead")));

        // Sem default
        let no_default = "menuentry \"Test\" {\n  linux /vmlinuz\n}\n";
        let issues = validate_grub_config_syntax(no_default);
        assert!(issues.iter().any(|i| i.contains("default")));
    }

    #[test]
    fn test_resolve_grub_path() {
        let boot_path = PathBuf::from("/boot");
        let chroot_root = Some(PathBuf::from("/mnt"));

        // Caminho absoluto do GRUB
        assert_eq!(
            resolve_grub_path("/vmlinuz-linux", &boot_path, &chroot_root),
            PathBuf::from("/mnt/boot/vmlinuz-linux")
        );

        // Caminho relativo
        assert_eq!(
            resolve_grub_path("vmlinuz-linux", &boot_path, &chroot_root),
            PathBuf::from("/boot/vmlinuz-linux")
        );

        // Sem chroot
        let no_chroot = None;
        assert_eq!(
            resolve_grub_path("/vmlinuz-linux", &boot_path, &no_chroot),
            PathBuf::from("/boot/vmlinuz-linux")
        );
    }
}
