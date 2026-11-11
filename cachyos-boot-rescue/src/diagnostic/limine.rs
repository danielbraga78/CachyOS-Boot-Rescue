//! Adaptador de diagnóstico do Limine (somente leitura).
//!
//! Verifica evidências de configuração, binário EFI e caminhos de kernel/initramfs.

use crate::diagnostic::types::*;
use crate::discovery::types::LinuxInstallationCandidate;
use crate::error::Error;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Verifica a instalação do Limine.
pub fn check_limine(
    environment: &BootEnvironment,
    _candidate: &LinuxInstallationCandidate,
    chroot_root: &Option<PathBuf>,
    _config: &DiagnosticConfig,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // Limine requer UEFI
    if environment.firmware_mode != FirmwareMode::Uefi {
        findings.push(Finding {
            id: DiagnosticId::LimineEspLayoutMismatch,
            severity: Severity::Info,
            title: "Limine não compatível com BIOS Legacy".to_string(),
            description: "O Limine só suporta boot UEFI. Sistema atual está em modo BIOS/Legacy."
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
    let (boot_path, efi_path) = resolve_limine_paths(environment, chroot_root);

    // 1. Verifica binário Limine EFI
    findings.extend(check_limine_efi_binary(&efi_path)?);

    // 2. Verifica limine.conf
    findings.extend(check_limine_config(&boot_path, &efi_path, chroot_root)?);

    // 3. Verifica caminhos de kernel/initramfs no limine.conf
    findings.extend(check_limine_kernel_initramfs_paths(
        &boot_path,
        &efi_path,
        environment,
        chroot_root,
    )?);

    // 4. Verifica layout ESP compatível
    findings.extend(check_limine_esp_layout(&environment)?);

    Ok(findings)
}

/// Resolve caminhos para verificação do Limine.
fn resolve_limine_paths(
    environment: &BootEnvironment,
    chroot_root: &Option<PathBuf>,
) -> (PathBuf, PathBuf) {
    let root = chroot_root.clone().unwrap_or_else(|| PathBuf::from("/"));

    // Limine geralmente usa a ESP diretamente
    let efi_path = if let Some(esp) = &environment.primary_esp {
        if let Some(mp) = &esp.mountpoint {
            root.join(mp.strip_prefix("/").unwrap_or(mp))
        } else {
            root.join("boot") // Limine costuma usar /boot como ESP
        }
    } else {
        root.join("boot")
    };

    // /boot pode ser o mesmo que ESP ou separado
    let boot_path = match environment.boot_layout {
        BootLayout::UnifiedBootOnEsp => efi_path.clone(),
        _ => root.join("boot"),
    };

    (boot_path, efi_path)
}

/// Verifica presença do binário Limine EFI.
fn check_limine_efi_binary(efi_path: &Path) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // Caminhos típicos do Limine EFI
    let limine_efi_paths = vec![
        efi_path.join("EFI/BOOT/BOOTX64.EFI"), // Limine instala aqui por padrão
        efi_path.join("EFI/limine/liminex64.efi"),
        efi_path.join("liminex64.efi"), // Se /boot é a ESP
    ];

    let mut found = false;
    let mut found_path = None;

    for path in &limine_efi_paths {
        if path.exists() {
            found = true;
            found_path = Some(path.clone());
            break;
        }
    }

    if !found {
        findings.push(Finding {
            id: DiagnosticId::LimineEfiBinaryMissing,
            severity: Severity::Error,
            title: "Binário Limine EFI não encontrado".to_string(),
            description: "Nenhum binário Limine EFI (liminex64.efi ou BOOTX64.EFI) foi encontrado nos caminhos esperados. O sistema UEFI não conseguirá carregar o Limine.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::FileMissing,
                description: "Binário Limine EFI ausente".to_string(),
                path: Some(efi_path.to_path_buf()),
                expected: Some("EFI/BOOT/BOOTX64.EFI ou EFI/limine/liminex64.efi".to_string()),
                observed: Some("Nenhum arquivo encontrado".to_string()),
                raw_data: HashMap::from([
                    ("checked_paths".to_string(), format!("{:?}", limine_efi_paths)),
                ]),
            }],
            recommendations: vec![
                Recommendation {
                    id: "limine_install".to_string(),
                    description: "Instalar Limine na ESP com limine-install.".to_string(),
                    action: "Executar limine-install <device> (ex: limine-install /dev/nvme0n1)".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::High,
                    prerequisites: vec!["ESP montada".to_string(), "Chroot preparado".to_string(), "Backup da ESP".to_string(), "Pacote limine instalado".to_string()],
                },
            ],
            verified_in_chroot: false,
            context: HashMap::new(),
        });
    } else {
        let info_path = found_path.clone().unwrap_or_else(|| efi_path.to_path_buf());
        findings.push(Finding {
            id: DiagnosticId::LimineEfiBinaryMissing, // Info
            severity: Severity::Info,
            title: "Binário Limine EFI encontrado".to_string(),
            description: format!("Binário Limine EFI presente em: {}", info_path.display()),
            evidence: vec![Evidence {
                kind: EvidenceKind::FileExists,
                description: "Binário Limine EFI encontrado".to_string(),
                path: Some(info_path),
                expected: Some("liminex64.efi ou BOOTX64.EFI".to_string()),
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

/// Verifica presença e validade do limine.conf.
fn check_limine_config(
    boot_path: &Path,
    efi_path: &Path,
    chroot_root: &Option<PathBuf>,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // Caminhos típicos do limine.conf
    let config_paths = vec![
        boot_path.join("limine.conf"),
        efi_path.join("limine.conf"),
        efi_path.join("EFI/limine/limine.conf"),
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
                    findings.push(Finding {
                        id: DiagnosticId::LimineConfigInvalid,
                        severity: Severity::Error,
                        title: "limine.conf vazio".to_string(),
                        description: format!("O arquivo {} existe mas está vazio.", path.display()),
                        evidence: vec![Evidence {
                            kind: EvidenceKind::ContentMismatch,
                            description: "limine.conf existe mas está vazio".to_string(),
                            path: Some(path.clone()),
                            expected: Some("Conteúdo de configuração Limine".to_string()),
                            observed: Some("Arquivo vazio".to_string()),
                            raw_data: HashMap::new(),
                        }],
                        recommendations: vec![Recommendation {
                            id: "create_limine_conf".to_string(),
                            description: "Criar limine.conf com entradas de boot válidas."
                                .to_string(),
                            action:
                                "Criar /boot/limine.conf ou /efi/limine.conf com entradas válidas"
                                    .to_string(),
                            is_mutating: true,
                            risk: RiskLevel::Medium,
                            prerequisites: vec!["Kernel instalado".to_string()],
                        }],
                        verified_in_chroot: chroot_root.is_some(),
                        context: HashMap::new(),
                    });
                    return Ok(findings);
                }
                Err(e) => {
                    findings.push(Finding {
                        id: DiagnosticId::LimineConfigInvalid,
                        severity: Severity::Warning,
                        title: "Erro ao ler limine.conf".to_string(),
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
            id: DiagnosticId::LimineConfigMissing,
            severity: Severity::Error,
            title: "limine.conf não encontrado".to_string(),
            description: "Nenhum arquivo limine.conf foi encontrado nos caminhos esperados. O Limine não terá entradas de boot para exibir.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::FileMissing,
                description: "limine.conf ausente em todos os caminhos padrão".to_string(),
                path: Some(boot_path.join("limine.conf")),
                expected: Some("/boot/limine.conf ou /efi/limine.conf".to_string()),
                observed: Some("Nenhum arquivo encontrado".to_string()),
                raw_data: HashMap::from([
                    ("checked_paths".to_string(), format!("{:?}", config_paths)),
                ]),
            }],
            recommendations: vec![Recommendation {
                id: "create_limine_conf".to_string(),
                description: "Criar limine.conf com entradas de boot válidas.".to_string(),
                action: "Criar limine.conf baseado em template ou documentação do Limine".to_string(),
                is_mutating: true,
                risk: RiskLevel::Medium,
                prerequisites: vec!["Kernel instalado".to_string(), "Initramfs disponível".to_string()],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
        return Ok(findings);
    }

    let config_path = found_config.unwrap();

    // Validação básica de sintaxe do limine.conf
    let syntax_issues = validate_limine_config_syntax(&config_content);
    if !syntax_issues.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::LimineConfigInvalid,
            severity: Severity::Warning,
            title: "Possíveis problemas de sintaxe no limine.conf".to_string(),
            description: format!(
                "Foram detectados {} problema(s) potencial(is) no limine.conf: {}",
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
                id: "review_limine_conf".to_string(),
                description: "Revisar manualmente o limine.conf.".to_string(),
                action: "Verificar limine.conf e corrigir conforme documentação do Limine"
                    .to_string(),
                is_mutating: false,
                risk: RiskLevel::Low,
                prerequisites: vec![],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    } else {
        findings.push(Finding {
            id: DiagnosticId::LimineConfigMissing, // Info
            severity: Severity::Info,
            title: "limine.conf encontrado e sintaticamente válido".to_string(),
            description: format!(
                "Arquivo de configuração Limine presente em: {}",
                config_path.display()
            ),
            evidence: vec![Evidence {
                kind: EvidenceKind::ContentMatches,
                description: "limine.conf encontrado e válido".to_string(),
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

/// Validação básica de sintaxe do limine.conf.
fn validate_limine_config_syntax(content: &str) -> Vec<String> {
    let mut issues = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut has_entry = false;
    let mut in_entry = false;
    let mut entry_has_kernel = false;
    let mut entry_has_initrd = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Ignora comentários
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        // Detecta início de entrada
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Fecha entrada anterior
            if in_entry {
                if !entry_has_kernel {
                    issues.push(format!(
                        "Entrada '{}' sem kernel",
                        lines[i - 1].trim().trim_matches(|c| c == '[' || c == ']')
                    ));
                }
                if !entry_has_initrd {
                    issues.push(format!(
                        "Entrada '{}' sem initrd",
                        lines[i - 1].trim().trim_matches(|c| c == '[' || c == ']')
                    ));
                }
            }
            in_entry = true;
            entry_has_kernel = false;
            entry_has_initrd = false;
            has_entry = true;
            continue;
        }

        if in_entry {
            if trimmed.starts_with("kernel:") || trimmed.starts_with("kernel ") {
                entry_has_kernel = true;
            }
            if trimmed.starts_with("initrd:") || trimmed.starts_with("initrd ") {
                entry_has_initrd = true;
            }
        }
    }

    // Verifica última entrada
    if in_entry {
        if !entry_has_kernel {
            issues.push("Última entrada sem kernel".to_string());
        }
        if !entry_has_initrd {
            issues.push("Última entrada sem initrd".to_string());
        }
    }

    if !has_entry {
        issues.push("Nenhuma entrada de boot (seção [...]) encontrada".to_string());
    }

    // Verifica timeout
    if !content.lines().any(|l| l.trim().starts_with("timeout:")) {
        issues.push("Timeout não definido (opcional mas recomendado)".to_string());
    }

    issues
}

/// Verifica caminhos de kernel e initramfs no limine.conf.
fn check_limine_kernel_initramfs_paths(
    boot_path: &Path,
    efi_path: &Path,
    environment: &BootEnvironment,
    chroot_root: &Option<PathBuf>,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    let config_paths = vec![
        boot_path.join("limine.conf"),
        efi_path.join("limine.conf"),
        efi_path.join("EFI/limine/limine.conf"),
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
        return Ok(findings); // Já reportado
    }

    let root = chroot_root.clone().unwrap_or_else(|| PathBuf::from("/"));

    // Extrai paths kernel: e initrd:
    let kernel_pattern = regex::Regex::new(r#"kernel:\s*(/?[^\s]+)"#).unwrap();
    let initrd_pattern = regex::Regex::new(r#"initrd:\s*(/?[^\s]+)"#).unwrap();

    let mut missing_kernels = Vec::new();
    let mut missing_initramfs = Vec::new();

    for cap in kernel_pattern.captures_iter(&config_content) {
        if let Some(path_match) = cap.get(1) {
            let kernel_path = path_match.as_str();
            let full_path = resolve_limine_path(kernel_path, &root, &environment.boot_layout);
            if !full_path.exists() {
                missing_kernels.push((kernel_path.to_string(), full_path.display().to_string()));
            }
        }
    }

    for cap in initrd_pattern.captures_iter(&config_content) {
        if let Some(path_match) = cap.get(1) {
            let initrd_path = path_match.as_str();
            let full_path = resolve_limine_path(initrd_path, &root, &environment.boot_layout);
            if !full_path.exists() {
                missing_initramfs.push((initrd_path.to_string(), full_path.display().to_string()));
            }
        }
    }

    if !missing_kernels.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::LimineKernelPathInvalid,
            severity: Severity::Critical,
            title: "Kernel(s) referenciado(s) no limine.conf não encontrado(s)".to_string(),
            description: format!(
                "{} entrada(s) de kernel no limine.conf apontam para arquivos inexistentes.",
                missing_kernels.len()
            ),
            evidence: missing_kernels
                .iter()
                .map(|(ref_path, full_path)| Evidence {
                    kind: EvidenceKind::FileMissing,
                    description: format!("Kernel referenciado: {}", ref_path),
                    path: Some(PathBuf::from(full_path)),
                    expected: Some("Arquivo de kernel existente".to_string()),
                    observed: Some("Arquivo não encontrado".to_string()),
                    raw_data: HashMap::new(),
                })
                .collect(),
            recommendations: vec![Recommendation {
                id: "limine_fix_kernel_path".to_string(),
                description: "Corrigir caminhos do kernel no limine.conf ou instalar kernel."
                    .to_string(),
                action: "Editar limine.conf com caminhos corretos ou instalar kernel".to_string(),
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
            id: DiagnosticId::LimineInitramfsPathInvalid,
            severity: Severity::Critical,
            title: "Initramfs referenciado(s) no limine.conf não encontrado(s)".to_string(),
            description: format!(
                "{} entrada(s) de initramfs no limine.conf apontam para arquivos inexistentes.",
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
            recommendations: vec![Recommendation {
                id: "limine_fix_initrd_path".to_string(),
                description:
                    "Corrigir caminhos do initramfs no limine.conf ou regenerar initramfs."
                        .to_string(),
                action: "Editar limine.conf com caminhos corretos ou executar mkinitcpio -P"
                    .to_string(),
                is_mutating: true,
                risk: RiskLevel::High,
                prerequisites: vec!["Kernel instalado".to_string()],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    if missing_kernels.is_empty() && missing_initramfs.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::LimineKernelPathInvalid, // Info
            severity: Severity::Info,
            title: "Kernels e initramfs referenciados no limine.conf encontrados".to_string(),
            description:
                "Todos os kernels e initramfs referenciados no limine.conf existem no filesystem."
                    .to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::ContentMatches,
                description: "Caminhos válidos".to_string(),
                path: Some(boot_path.join("limine.conf")),
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

/// Resolve caminho do Limine para caminho absoluto.
fn resolve_limine_path(limine_path: &str, root: &Path, boot_layout: &BootLayout) -> PathBuf {
    if limine_path.starts_with('/') {
        // Caminho absoluto - Limine resolve relativo à raiz da ESP
        // Se /boot é a ESP, caminho absoluto /vmlinuz -> /boot/vmlinuz
        // Se ESP é separada, caminho absoluto pode ser relativo à ESP
        match boot_layout {
            BootLayout::UnifiedBootOnEsp => root
                .join("boot")
                .join(limine_path.strip_prefix('/').unwrap_or(limine_path)),
            _ => root.join(limine_path.strip_prefix('/').unwrap_or(limine_path)),
        }
    } else {
        // Caminho relativo - relativo ao local do limine.conf
        root.join("boot").join(limine_path)
    }
}

/// Verifica layout ESP compatível com Limine.
fn check_limine_esp_layout(environment: &BootEnvironment) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    // Limine funciona melhor quando /boot É a ESP (layout unificado)
    // ou quando a ESP está montada em /boot
    let is_compatible = matches!(
        environment.boot_layout,
        BootLayout::UnifiedBootOnEsp | BootLayout::EfiAtEfiBootSeparate
    );

    if !is_compatible {
        findings.push(Finding {
            id: DiagnosticId::LimineEspLayoutMismatch,
            severity: Severity::Warning,
            title: "Layout ESP pode não ser ideal para Limine".to_string(),
            description: format!(
                "Layout detectado: {:?}. O Limine funciona melhor quando /boot está na ESP (layout unificado) ou ESP montada em /boot. Layouts com /boot separado e ESP em /boot/efi podem requerer configuração adicional no limine.conf.",
                environment.boot_layout
            ),
            evidence: vec![Evidence {
                kind: EvidenceKind::ConfigValueMismatch,
                description: "Layout ESP subótimo para Limine".to_string(),
                path: None,
                expected: Some("UnifiedBootOnEsp ou EfiAtEfiBootSeparate".to_string()),
                observed: Some(format!("{:?}", environment.boot_layout)),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![
                Recommendation {
                    id: "limine_adjust_config".to_string(),
                    description: "Ajustar caminhos no limine.conf para apontar para a ESP correta.".to_string(),
                    action: "Editar limine.conf usando caminhos absolutos a partir da raiz da ESP".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::Medium,
                    prerequisites: vec!["Entender layout de partições".to_string()],
                },
                Recommendation {
                    id: "limine_reconsider_layout".to_string(),
                    description: "Considerar migrar para layout unificado (/boot na ESP) para simplicidade com Limine.".to_string(),
                    action: "Reconfigurar partições: mover /boot para ESP (requer backup e reparticionamento)".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::Critical,
                    prerequisites: vec!["Backup completo".to_string(), "Live USB".to_string(), "Tempo para reconfiguração".to_string()],
                },
            ],
            verified_in_chroot: false,
            context: HashMap::new(),
        });
    }

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_validate_limine_config_syntax() {
        // Config válida
        let valid = r#"
timeout: 5

[Linux]
kernel: /vmlinuz-linux
initrd: /initramfs-linux.img
cmdline: root=UUID=xxx rw
"#;
        let issues = validate_limine_config_syntax(valid);
        assert!(issues.is_empty());

        // Sem entradas
        let no_entry = "timeout: 5\n";
        let issues = validate_limine_config_syntax(no_entry);
        assert!(issues.iter().any(|i| i.contains("entrada")));

        // Entrada sem kernel
        let no_kernel = r#"
[Linux]
initrd: /initramfs-linux.img
"#;
        let issues = validate_limine_config_syntax(no_kernel);
        assert!(issues.iter().any(|i| i.contains("kernel")));

        // Entrada sem initrd
        let no_initrd = r#"
[Linux]
kernel: /vmlinuz-linux
"#;
        let issues = validate_limine_config_syntax(no_initrd);
        assert!(issues.iter().any(|i| i.contains("initrd")));
    }

    #[test]
    fn test_resolve_limine_path() {
        let root = PathBuf::from("/mnt");

        // Layout unificado - caminho absoluto
        assert_eq!(
            resolve_limine_path("/vmlinuz-linux", &root, &BootLayout::UnifiedBootOnEsp),
            PathBuf::from("/mnt/boot/vmlinuz-linux")
        );

        // Layout separado - caminho absoluto
        assert_eq!(
            resolve_limine_path("/vmlinuz-linux", &root, &BootLayout::SeparateBootEfi),
            PathBuf::from("/mnt/vmlinuz-linux")
        );

        // Caminho relativo
        assert_eq!(
            resolve_limine_path("vmlinuz-linux", &root, &BootLayout::UnifiedBootOnEsp),
            PathBuf::from("/mnt/boot/vmlinuz-linux")
        );
    }
}
