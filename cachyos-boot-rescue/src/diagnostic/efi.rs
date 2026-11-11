//! Adaptador de diagnóstico EFI (somente leitura).
//!
//! Verifica entradas de boot UEFI NVRAM, BootOrder e status Secure Boot
//! usando efibootmgr e mokutil em modo somente leitura.

use crate::diagnostic::types::*;
use crate::error::Error;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Coleta entradas de boot EFI via efibootmgr -v.
pub fn collect_efi_entries() -> Result<(Vec<EfiBootEntry>, Option<Vec<u16>>), Error> {
    let mut entries = Vec::new();
    let mut boot_order = None;

    // Executa efibootmgr -v
    let output = Command::new("efibootmgr").args(["-v"]).output();

    let output = match output {
        Ok(o) => o,
        Err(_) => {
            // efibootmgr pode não estar instalado
            return Ok((entries, boot_order));
        }
    };

    if !output.status.success() {
        // Pode falhar se não for UEFI ou sem permissões
        return Ok((entries, boot_order));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    let mut current_entry: Option<EfiBootEntry> = None;

    for line in lines {
        let trimmed = line.trim();

        // BootOrder: 0001,0002,0000
        if trimmed.starts_with("BootOrder:") {
            let order_str = trimmed.strip_prefix("BootOrder:").unwrap().trim();
            boot_order = Some(
                order_str
                    .split(',')
                    .filter_map(|s| u16::from_str_radix(s.trim(), 16).ok())
                    .collect(),
            );
            continue;
        }

        // Boot0001* Ubuntu HD(1,GPT,...)/File(\EFI\ubuntu\shimx64.efi)
        if trimmed.starts_with("Boot") && trimmed.len() >= 8 {
            // Finaliza entrada anterior se houver
            if let Some(entry) = current_entry.take() {
                entries.push(entry);
            }

            // Parse da nova entrada
            // Formato: BootXXXX* Label HD(...)/File(\path) [args]
            let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
            if parts.len() < 2 {
                continue;
            }

            let boot_num_str = &parts[0][4..8]; // BootXXXX
            let boot_num = u16::from_str_radix(boot_num_str, 16).unwrap_or(0);
            let is_active = parts[0].chars().nth(8) == Some('*');

            // Resto da linha: Label HD(...)/File(\path) [args]
            let rest = parts.get(2).map_or("", |v| v).trim();

            // Extrai label (tudo antes de HD())
            let label_end = rest.find("HD(").unwrap_or(rest.len());
            let label = rest[..label_end].trim().to_string();

            let (device_path, loader_path) = if let Some(hd_start) = rest.find("HD(") {
                let hd_part = &rest[hd_start..];
                // HD(1,GPT,xxxx)/File(\EFI\xxx\xxx.efi)
                let file_start = hd_part.find("/File(");
                let device_path = if let Some(fs) = file_start {
                    hd_part[..fs].to_string()
                } else {
                    hd_part.to_string()
                };

                let loader_path = if let Some(fs) = file_start {
                    let file_part = &hd_part[fs + 6..]; // pula "/File("
                    let file_end = file_part.find(')').unwrap_or(file_part.len());
                    file_part[..file_end].replace('\\', "/")
                } else {
                    String::new()
                };
                (device_path, loader_path)
            } else {
                (String::new(), String::new())
            };

            current_entry = Some(EfiBootEntry {
                boot_num,
                label,
                loader_path,
                device_path,
                arguments: None,
                is_active,
            });
        }
    }

    // Adiciona última entrada
    if let Some(entry) = current_entry.take() {
        entries.push(entry);
    }

    Ok((entries, boot_order))
}

/// Verifica entradas EFI NVRAM.
pub fn check_efi_entries(
    environment: &BootEnvironment,
    chroot_root: &Option<PathBuf>,
    _config: &DiagnosticConfig,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    let (entries, boot_order) = collect_efi_entries()?;

    if entries.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::EfiBootEntriesMissing,
            severity: Severity::Warning,
            title: "Nenhuma entrada EFI de boot encontrada".to_string(),
            description: "O efibootmgr não retornou entradas de boot. Isso pode indicar que nenhuma entrada foi criada ou que o comando falhou.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::EfiEntryMissing,
                description: "efibootmgr -v não retornou entradas".to_string(),
                path: None,
                expected: Some("Entradas BootXXXX".to_string()),
                observed: Some("Nenhuma".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![
                Recommendation {
                    id: "create_efi_entry".to_string(),
                    description: "Criar entrada EFI para o bootloader instalado.".to_string(),
                    action: "efibootmgr -c -d <disk> -p <part> -L \"Label\" -l \"\\EFI\\...\\grubx64.efi\"".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::Medium,
                    prerequisites: vec!["ESP montada".to_string(), "Bootloader instalado na ESP".to_string()],
                },
            ],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
        return Ok(findings);
    }

    // Verifica BootOrder
    if boot_order.is_none() || boot_order.as_ref().unwrap().is_empty() {
        findings.push(Finding {
            id: DiagnosticId::EfiBootOrderMissing,
            severity: Severity::Warning,
            title: "BootOrder EFI não definido".to_string(),
            description: "Existem entradas EFI mas BootOrder não está definido. O firmware pode não saber em qual ordem tentar o boot.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::EfiEntryMissing,
                description: "BootOrder ausente ou vazio".to_string(),
                path: None,
                expected: Some("BootOrder: 0001,0002,...".to_string()),
                observed: Some("Não definido".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![Recommendation {
                id: "set_boot_order".to_string(),
                description: "Definir BootOrder com efibootmgr -o.".to_string(),
                action: "efibootmgr -o 0001,0002,0000".to_string(),
                is_mutating: true,
                risk: RiskLevel::Low,
                prerequisites: vec!["Entradas EFI existentes".to_string()],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    // Verifica cada entrada
    let root = chroot_root.clone().unwrap_or_else(|| PathBuf::from("/"));

    for entry in &entries {
        // Verifica se o arquivo EFI existe
        let loader_full_path =
            resolve_efi_loader_path(&entry.loader_path, &root, &environment.boot_layout);

        if !loader_full_path.exists() {
            findings.push(Finding {
                id: DiagnosticId::EfiBootEntryPointsToMissingFile,
                severity: Severity::Error,
                title: format!("Entrada EFI Boot{:04X} aponta para arquivo inexistente", entry.boot_num),
                description: format!("A entrada '{}' (Boot{:04X}) referencia {} que não existe no filesystem.", entry.label, entry.boot_num, entry.loader_path),
                evidence: vec![Evidence {
                    kind: EvidenceKind::EfiEntryInvalid,
                    description: "Arquivo EFI referenciado não encontrado".to_string(),
                    path: Some(loader_full_path.clone()),
                    expected: Some("Arquivo EFI existente".to_string()),
                    observed: Some("Arquivo não encontrado".to_string()),
                    raw_data: HashMap::from([
                        ("boot_num".to_string(), format!("{:04X}", entry.boot_num)),
                        ("label".to_string(), entry.label.clone()),
                        ("loader_path".to_string(), entry.loader_path.clone()),
                        ("device_path".to_string(), entry.device_path.clone()),
                    ]),
                }],
                recommendations: vec![
                    Recommendation {
                        id: "fix_efi_entry_path".to_string(),
                        description: "Corrigir caminho da entrada EFI ou instalar bootloader no local correto.".to_string(),
                        action: "efibootmgr -b {:04X} -l \"\\EFI\\...\\novo_caminho.efi\"".to_string(),
                        is_mutating: true,
                        risk: RiskLevel::Medium,
                        prerequisites: vec!["Bootloader instalado".to_string(), "ESP montada".to_string()],
                    },
                    Recommendation {
                        id: "delete_broken_efi_entry".to_string(),
                        description: "Remover entrada EFI quebrada.".to_string(),
                        action: "efibootmgr -b {:04X} -B".to_string(),
                        is_mutating: true,
                        risk: RiskLevel::Low,
                        prerequisites: vec!["Outras entradas funcionais".to_string()],
                    },
                ],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
        } else {
            findings.push(Finding {
                id: DiagnosticId::EfiBootEntryInvalid, // Info
                severity: Severity::Info,
                title: format!("Entrada EFI Boot{:04X} válida", entry.boot_num),
                description: format!(
                    "Entrada '{}' (Boot{:04X}) aponta para arquivo existente: {}",
                    entry.label, entry.boot_num, entry.loader_path
                ),
                evidence: vec![Evidence {
                    kind: EvidenceKind::EfiEntryValid,
                    description: "Entrada EFI válida".to_string(),
                    path: Some(loader_full_path),
                    expected: Some("Arquivo existente".to_string()),
                    observed: Some("OK".to_string()),
                    raw_data: HashMap::from([
                        ("boot_num".to_string(), format!("{:04X}", entry.boot_num)),
                        ("label".to_string(), entry.label.clone()),
                        ("loader_path".to_string(), entry.loader_path.clone()),
                        ("is_active".to_string(), entry.is_active.to_string()),
                    ]),
                }],
                recommendations: vec![],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
        }
    }

    // Verifica se há entrada para o bootloader detectado
    let has_grub_entry = entries
        .iter()
        .any(|e| e.loader_path.to_lowercase().contains("grub"));
    let has_limine_entry = entries
        .iter()
        .any(|e| e.loader_path.to_lowercase().contains("limine"));
    let has_systemd_entry = entries
        .iter()
        .any(|e| e.loader_path.to_lowercase().contains("systemd"));

    match environment.primary_bootloader {
        Some(BootloaderType::Grub) if !has_grub_entry => {
            findings.push(Finding {
                id: DiagnosticId::EfiBootEntriesMissing,
                severity: Severity::Warning,
                title: "Nenhuma entrada EFI para GRUB detectada".to_string(),
                description: "GRUB foi detectado no filesystem mas não há entrada EFI NVRAM correspondente.".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::EfiEntryMissing,
                    description: "Entrada GRUB ausente no NVRAM".to_string(),
                    path: None,
                    expected: Some("BootXXXX com \\EFI\\grub\\grubx64.efi".to_string()),
                    observed: Some("Não encontrado".to_string()),
                    raw_data: HashMap::new(),
                }],
                recommendations: vec![Recommendation {
                    id: "create_grub_efi_entry".to_string(),
                    description: "Criar entrada EFI para GRUB.".to_string(),
                    action: "efibootmgr -c -d <disk> -p <part> -L \"GRUB\" -l \"\\EFI\\grub\\grubx64.efi\"".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::Medium,
                    prerequisites: vec!["GRUB instalado na ESP".to_string(), "ESP montada".to_string()],
                }],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
        }
        Some(BootloaderType::Limine) if !has_limine_entry => {
            findings.push(Finding {
                id: DiagnosticId::EfiBootEntriesMissing,
                severity: Severity::Warning,
                title: "Nenhuma entrada EFI para Limine detectada".to_string(),
                description: "Limine foi detectado no filesystem mas não há entrada EFI NVRAM correspondente. Limine usa BOOTX64.EFI como fallback.".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::EfiEntryMissing,
                    description: "Entrada Limine ausente no NVRAM".to_string(),
                    path: None,
                    expected: Some("BootXXXX com \\EFI\\BOOT\\BOOTX64.EFI".to_string()),
                    observed: Some("Não encontrado".to_string()),
                    raw_data: HashMap::new(),
                }],
                recommendations: vec![Recommendation {
                    id: "create_limine_efi_entry".to_string(),
                    description: "Criar entrada EFI para Limine (opcional, usa fallback BOOTX64.EFI).".to_string(),
                    action: "efibootmgr -c -d <disk> -p <part> -L \"Limine\" -l \"\\EFI\\BOOT\\BOOTX64.EFI\"".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::Low,
                    prerequisites: vec!["Limine instalado na ESP".to_string(), "ESP montada".to_string()],
                }],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
        }
        Some(BootloaderType::SystemdBoot) if !has_systemd_entry => {
            findings.push(Finding {
                id: DiagnosticId::EfiBootEntriesMissing,
                severity: Severity::Warning,
                title: "Nenhuma entrada EFI para systemd-boot detectada".to_string(),
                description:
                    "systemd-boot foi detectado mas não há entrada EFI NVRAM correspondente."
                        .to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::EfiEntryMissing,
                    description: "Entrada systemd-boot ausente no NVRAM".to_string(),
                    path: None,
                    expected: Some("BootXXXX com \\EFI\\systemd\\systemd-bootx64.efi".to_string()),
                    observed: Some("Não encontrado".to_string()),
                    raw_data: HashMap::new(),
                }],
                recommendations: vec![Recommendation {
                    id: "create_systemd_efi_entry".to_string(),
                    description: "Criar entrada EFI para systemd-boot.".to_string(),
                    action: "bootctl install --path=<ESP> (cria entrada automaticamente)"
                        .to_string(),
                    is_mutating: true,
                    risk: RiskLevel::Medium,
                    prerequisites: vec![
                        "systemd-boot instalado na ESP".to_string(),
                        "ESP montada".to_string(),
                    ],
                }],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
        }
        _ => {}
    }

    // Verifica entradas Windows (preservação)
    let windows_entries: Vec<_> = entries
        .iter()
        .filter(|e| {
            e.label.to_lowercase().contains("windows")
                || e.loader_path.to_lowercase().contains("microsoft")
        })
        .collect();
    if !windows_entries.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::EfiBootEntryInvalid, // Info
            severity: Severity::Info,
            title: format!("{} entrada(s) Windows Boot Manager preservada(s)", windows_entries.len()),
            description: "Entradas do Windows Boot Manager detectadas e preservadas. Não devem ser removidas ou modificadas.".to_string(),
            evidence: windows_entries.iter().map(|e| Evidence {
                kind: EvidenceKind::EfiEntryExists,
                description: format!("Windows entry: {}", e.label),
                path: None,
                expected: Some("Preservar".to_string()),
                observed: Some("Presente".to_string()),
                raw_data: HashMap::from([
                    ("boot_num".to_string(), format!("{:04X}", e.boot_num)),
                    ("loader_path".to_string(), e.loader_path.clone()),
                ]),
            }).collect(),
            recommendations: vec![Recommendation {
                id: "preserve_windows_entries".to_string(),
                description: "Manter entradas Windows intactas durante qualquer reparo de boot.".to_string(),
                action: "Não remover nem modificar entradas EFI do Windows".to_string(),
                is_mutating: false,
                risk: RiskLevel::Low,
                prerequisites: vec![],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    Ok(findings)
}

/// Resolve caminho do loader EFI para caminho absoluto no filesystem.
fn resolve_efi_loader_path(
    efi_loader_path: &str,
    root: &Path,
    boot_layout: &BootLayout,
) -> PathBuf {
    // Caminho EFI usa backslashes e começa com \EFI\
    let normalized = efi_loader_path
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string();

    match boot_layout {
        BootLayout::UnifiedBootOnEsp => root.join("boot").join(&normalized),
        BootLayout::EfiAtEfiBootSeparate => root.join("efi").join(&normalized),
        _ => root.join("boot/efi").join(&normalized),
    }
}

/// Detecta status do Secure Boot.
pub fn detect_secure_boot() -> Result<SecureBootState, Error> {
    // Tenta mokutil --sb-state
    let output = Command::new("mokutil").args(["--sb-state"]).output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
            if stdout.contains("enabled") {
                return Ok(SecureBootState::Enabled);
            } else if stdout.contains("disabled") {
                return Ok(SecureBootState::Disabled);
            }
        }
        Ok(_) => {
            // Comando falhou - pode não estar instalado ou sem permissão
        }
        Err(_) => {
            // mokutil não encontrado
        }
    }

    // Fallback: verifica /sys/firmware/efi/efivars/ para variáveis SecureBoot
    let secure_boot_var =
        PathBuf::from("/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c");
    if secure_boot_var.exists() {
        // Tentar ler (pode falhar por permissão)
        if let Ok(data) = std::fs::read(&secure_boot_var) {
            // Formato: 4 bytes attributes + 1 byte value
            if data.len() >= 5 {
                let value = data[4];
                return Ok(if value == 1 {
                    SecureBootState::Enabled
                } else {
                    SecureBootState::Disabled
                });
            }
        }
    }

    Ok(SecureBootState::Unknown)
}

/// Verifica Secure Boot e reporta achados.
pub fn check_secure_boot(chroot_root: &Option<PathBuf>) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    let state = detect_secure_boot()?;

    match state {
        SecureBootState::Enabled => {
            findings.push(Finding {
                id: DiagnosticId::SecureBootEnabled,
                severity: Severity::Info,
                title: "Secure Boot ativo".to_string(),
                description: "O Secure Boot está habilitado no firmware. Bootloaders não assinados (GRUB padrão, Limine, kernels customizados) não inicializarão sem shim ou assinatura própria.".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutputMatches,
                    description: "Secure Boot enabled detectado".to_string(),
                    path: None,
                    expected: Some("Disabled para bootloaders não assinados".to_string()),
                    observed: Some("Enabled".to_string()),
                    raw_data: HashMap::new(),
                }],
                recommendations: vec![
                    Recommendation {
                        id: "secure_boot_use_shim".to_string(),
                        description: "Usar shim assinado (ex: shim-signed) para carregar GRUB não assinado com Secure Boot ativo.".to_string(),
                        action: "Instalar shim-signed e configurar GRUB via shim".to_string(),
                        is_mutating: true,
                        risk: RiskLevel::Medium,
                        prerequisites: vec!["Conexão de rede".to_string(), "Chroot preparado".to_string()],
                    },
                    Recommendation {
                        id: "secure_boot_disable".to_string(),
                        description: "Desativar Secure Boot no firmware UEFI/BIOS (mais simples para bootloaders não assinados).".to_string(),
                        action: "Entrar no setup UEFI e desativar Secure Boot".to_string(),
                        is_mutating: false, // Mudança no firmware, não no disco
                        risk: RiskLevel::Low,
                        prerequisites: vec!["Acesso físico ao equipamento".to_string()],
                    },
                    Recommendation {
                        id: "secure_boot_sign_own".to_string(),
                        description: "Assinar próprio bootloader/kernel com MOK (Machine Owner Key).".to_string(),
                        action: "Gerar chaves MOK, assinar GRUB/kernel, enrolar no firmware via mokutil".to_string(),
                        is_mutating: true,
                        risk: RiskLevel::High,
                        prerequisites: vec!["Conhecimento de PKI/assinatura".to_string(), "Chroot preparado".to_string()],
                    },
                ],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
        }
        SecureBootState::Disabled => {
            findings.push(Finding {
                id: DiagnosticId::SecureBootEnabled, // Info
                severity: Severity::Info,
                title: "Secure Boot desativado".to_string(),
                description: "O Secure Boot está desativado. Qualquer bootloader EFI poderá ser carregado sem restrições de assinatura.".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutputMatches,
                    description: "Secure Boot disabled".to_string(),
                    path: None,
                    expected: Some("Disabled".to_string()),
                    observed: Some("Disabled".to_string()),
                    raw_data: HashMap::new(),
                }],
                recommendations: vec![],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
        }
        SecureBootState::Unknown => {
            findings.push(Finding {
                id: DiagnosticId::SecureBootStateUnknown,
                severity: Severity::Warning,
                title: "Estado do Secure Boot desconhecido".to_string(),
                description: "Não foi possível determinar o estado do Secure Boot (mokutil não disponível ou sem permissão para ler variáveis EFI).".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandFailed,
                    description: "Falha ao detectar Secure Boot".to_string(),
                    path: None,
                    expected: Some("Enabled ou Disabled".to_string()),
                    observed: Some("Unknown".to_string()),
                    raw_data: HashMap::new(),
                }],
                recommendations: vec![Recommendation {
                    id: "check_secure_boot_manually".to_string(),
                    description: "Verificar manualmente no setup UEFI ou instalar mokutil.".to_string(),
                    action: "Instalar mokutil e executar mokutil --sb-state, ou verificar no firmware".to_string(),
                    is_mutating: false,
                    risk: RiskLevel::Low,
                    prerequisites: vec![],
                }],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
        }
        SecureBootState::Unsupported => {
            findings.push(Finding {
                id: DiagnosticId::SecureBootStateUnknown, // Info
                severity: Severity::Info,
                title: "Secure Boot não suportado (BIOS Legacy)".to_string(),
                description: "Sistema em modo BIOS/Legacy - Secure Boot não se aplica.".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::Other,
                    description: "Firmware BIOS Legacy".to_string(),
                    path: None,
                    expected: Some("N/A".to_string()),
                    observed: Some("Unsupported".to_string()),
                    raw_data: HashMap::new(),
                }],
                recommendations: vec![],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
        }
    }

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_resolve_efi_loader_path() {
        let root = PathBuf::from("/mnt");

        // Layout unificado
        assert_eq!(
            resolve_efi_loader_path(
                "\\EFI\\grub\\grubx64.efi",
                &root,
                &BootLayout::UnifiedBootOnEsp
            ),
            PathBuf::from("/mnt/boot/EFI/grub/grubx64.efi")
        );

        // Layout separado
        assert_eq!(
            resolve_efi_loader_path(
                "\\EFI\\grub\\grubx64.efi",
                &root,
                &BootLayout::SeparateBootEfi
            ),
            PathBuf::from("/mnt/boot/efi/EFI/grub/grubx64.efi")
        );

        // EFI at /efi
        assert_eq!(
            resolve_efi_loader_path(
                "\\EFI\\grub\\grubx64.efi",
                &root,
                &BootLayout::EfiAtEfiBootSeparate
            ),
            PathBuf::from("/mnt/efi/EFI/grub/grubx64.efi")
        );
    }
}
