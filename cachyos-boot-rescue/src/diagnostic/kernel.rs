//! Verificação de kernels e initramfs (somente leitura).
//!
//! Coleta kernels e initramfs instalados, verifica versões correspondentes.

use crate::diagnostic::types::*;
use crate::discovery::types::LinuxInstallationCandidate;
use crate::error::Error;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Coleta kernels e initramfs instalados.
pub fn collect_kernels_initramfs(
    candidate: &LinuxInstallationCandidate,
    chroot_root: &Option<PathBuf>,
) -> Result<(Vec<KernelInfo>, Vec<InitramfsInfo>), Error> {
    let mut kernels = Vec::new();
    let mut initramfs = Vec::new();

    let root = chroot_root.clone().unwrap_or_else(|| PathBuf::from("/"));

    // Determina diretório de boot baseado no candidato
    let boot_dirs = get_boot_directories(candidate, &root);

    for boot_dir in &boot_dirs {
        if !boot_dir.exists() {
            continue;
        }

        // Lista arquivos no diretório de boot
        let entries = fs::read_dir(boot_dir)
            .map_err(|e| Error::internal(format!("Erro ao ler diretório de boot: {e}")))?;

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Detecta kernels
            if is_kernel_file(&file_name) {
                if let Some(kernel_info) = parse_kernel_file(&path, &file_name, boot_dir) {
                    kernels.push(kernel_info);
                }
            }

            // Detecta initramfs
            if is_initramfs_file(&file_name) {
                if let Some(initramfs_info) = parse_initramfs_file(&path, &file_name, boot_dir) {
                    initramfs.push(initramfs_info);
                }
            }
        }
    }

    // Ordena kernels por versão (mais recente primeiro)
    kernels.sort_by(|a, b| version_cmp(&b.version, &a.version));

    // Marca o mais recente como default
    if let Some(k) = kernels.first_mut() {
        k.is_default = true;
    }

    // Ordena initramfs por variante
    initramfs.sort_by(|a, b| a.variant.cmp(&b.variant));

    Ok((kernels, initramfs))
}

/// Obtém diretórios de boot possíveis para o candidato.
fn get_boot_directories(candidate: &LinuxInstallationCandidate, root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. Partição de boot montada (se houver)
    for boot_part in &candidate.boot_partitions {
        if let Some(mp) = &boot_part.mountpoint {
            dirs.push(root.join(mp.strip_prefix("/").unwrap_or(mp)));
        }
    }

    // 2. Partições adicionais montadas em /boot
    for part in &candidate.additional_partitions {
        if let Some(mp) = &part.mountpoint {
            if mp == Path::new("/boot") {
                dirs.push(root.join("boot"));
            }
        }
    }

    // 3. /boot padrão
    if !dirs.iter().any(|d| d == &root.join("boot")) {
        dirs.push(root.join("boot"));
    }

    // 4. /efi (se layout unificado)
    if !dirs.iter().any(|d| d == &root.join("efi")) {
        dirs.push(root.join("efi"));
    }

    // 5. /boot/efi
    if !dirs.iter().any(|d| d == &root.join("boot/efi")) {
        dirs.push(root.join("boot/efi"));
    }

    dirs
}

/// Verifica se arquivo é kernel.
fn is_kernel_file(name: &str) -> bool {
    name.starts_with("vmlinuz")
        || name.starts_with("kernel-")
        || name.starts_with("Image")
        || (name.starts_with("linux") && name.contains("."))
}

/// Verifica se arquivo é initramfs.
fn is_initramfs_file(name: &str) -> bool {
    name.starts_with("initramfs")
        || name.starts_with("initrd")
        || name.ends_with(".img")
        || name.ends_with(".cpio")
        || name.ends_with(".cpio.gz")
        || name.ends_with(".cpio.zst")
}

/// Extrai informações do kernel.
fn parse_kernel_file(path: &Path, file_name: &str, _boot_dir: &Path) -> Option<KernelInfo> {
    let version = extract_kernel_version(file_name)?;
    let size = fs::metadata(path).ok()?.len();

    Some(KernelInfo {
        path: path.to_path_buf(),
        version,
        is_default: false, // Será definido depois
        size,
    })
}

/// Extrai versão do kernel do nome do arquivo.
fn extract_kernel_version(file_name: &str) -> Option<String> {
    // vmlinuz-6.12.1-arch1-1 -> 6.12.1-arch1-1
    // vmlinuz-linux -> linux (genérico)
    // kernel-6.12.1 -> 6.12.1
    // Image-6.12.1 -> 6.12.1

    if let Some(stripped) = file_name.strip_prefix("vmlinuz-") {
        return Some(stripped.to_string());
    }
    if let Some(stripped) = file_name.strip_prefix("vmlinuz-") {
        return Some(stripped.to_string());
    }
    if let Some(stripped) = file_name.strip_prefix("kernel-") {
        return Some(stripped.to_string());
    }
    if let Some(stripped) = file_name.strip_prefix("Image-") {
        return Some(stripped.to_string());
    }
    if file_name == "vmlinuz" || file_name == "vmlinuz-linux" {
        return Some("linux".to_string());
    }

    None
}

/// Extrai informações do initramfs.
fn parse_initramfs_file(path: &Path, file_name: &str, _boot_dir: &Path) -> Option<InitramfsInfo> {
    let (kernel_version, variant) = extract_initramfs_info(file_name)?;
    let size = fs::metadata(path).ok()?.len();

    Some(InitramfsInfo {
        path: path.to_path_buf(),
        kernel_version,
        variant,
        size,
    })
}

/// Extrai versão do kernel e variante do initramfs.
fn extract_initramfs_info(file_name: &str) -> Option<(String, String)> {
    // initramfs-6.12.1-arch1-1.img -> (6.12.1-arch1-1, default)
    // initramfs-6.12.1-arch1-1-fallback.img -> (6.12.1-arch1-1, fallback)
    // initrd-6.12.1.img -> (6.12.1, default)
    // initramfs-linux.img -> (linux, default)
    // initramfs-linux-fallback.img -> (linux, fallback)

    let name = file_name.strip_suffix(".img").unwrap_or(file_name);
    let name = name.strip_suffix(".cpio").unwrap_or(name);
    let name = name.strip_suffix(".gz").unwrap_or(name);
    let name = name.strip_suffix(".zst").unwrap_or(name);

    if let Some(stripped) = name.strip_prefix("initramfs-") {
        if let Some(variant_stripped) = stripped.strip_suffix("-fallback") {
            return Some((variant_stripped.to_string(), "fallback".to_string()));
        }
        if let Some(variant_stripped) = stripped.strip_suffix("-lts") {
            return Some((variant_stripped.to_string(), "lts".to_string()));
        }
        return Some((stripped.to_string(), "default".to_string()));
    }

    if let Some(stripped) = name.strip_prefix("initrd-") {
        if let Some(variant_stripped) = stripped.strip_suffix("-fallback") {
            return Some((variant_stripped.to_string(), "fallback".to_string()));
        }
        return Some((stripped.to_string(), "default".to_string()));
    }

    if name == "initramfs-linux" || name == "initrd-linux" {
        return Some(("linux".to_string(), "default".to_string()));
    }
    if name == "initramfs-linux-fallback" || name == "initrd-linux-fallback" {
        return Some(("linux".to_string(), "fallback".to_string()));
    }

    None
}

/// Compara versões de kernel (simplificada).
pub(crate) fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let a_main = a.split('-').next().unwrap_or(a);
    let b_main = b.split('-').next().unwrap_or(b);

    let a_parts: Vec<u32> = a_main
        .split('.')
        .filter_map(|part| part.parse::<u32>().ok())
        .collect();
    let b_parts: Vec<u32> = b_main
        .split('.')
        .filter_map(|part| part.parse::<u32>().ok())
        .collect();

    for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
        match a_part.cmp(b_part) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    match a_parts.len().cmp(&b_parts.len()) {
        std::cmp::Ordering::Equal => {
            let a_suffix = a.strip_prefix(a_main).unwrap_or_default();
            let b_suffix = b.strip_prefix(b_main).unwrap_or_default();
            let a_num = a_suffix
                .trim_start_matches('-')
                .chars()
                .filter(|ch| ch.is_ascii_digit())
                .collect::<String>()
                .parse::<u32>()
                .unwrap_or(0);
            let b_num = b_suffix
                .trim_start_matches('-')
                .chars()
                .filter(|ch| ch.is_ascii_digit())
                .collect::<String>()
                .parse::<u32>()
                .unwrap_or(0);
            a_num.cmp(&b_num)
        }
        other => other,
    }
}

/// Verifica kernels e initramfs - gera achados.
pub fn check_kernels_initramfs(
    _environment: &BootEnvironment,
    candidate: &LinuxInstallationCandidate,
    chroot_root: &Option<PathBuf>,
) -> Result<Vec<Finding>, Error> {
    let mut findings = Vec::new();

    let (kernels, initramfs) = collect_kernels_initramfs(candidate, chroot_root)?;

    // Verifica se há kernels
    if kernels.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::NoKernelsInstalled,
            severity: Severity::Critical,
            title: "Nenhum kernel instalado".to_string(),
            description: "Não foram encontrados arquivos de kernel (vmlinuz*, kernel*, Image*) nos diretórios de boot. O sistema não pode inicializar sem kernel.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::FileMissing,
                description: "Nenhum kernel encontrado em /boot, /efi, /boot/efi".to_string(),
                path: Some(chroot_root.clone().unwrap_or_else(|| PathBuf::from("/")).join("boot")),
                expected: Some("Pelo menos um vmlinuz-*".to_string()),
                observed: Some("Nenhum".to_string()),
                raw_data: HashMap::new(),
            }],
            recommendations: vec![Recommendation {
                id: "install_kernel".to_string(),
                description: "Instalar um kernel (ex: linux, linux-lts, linux-zen).".to_string(),
                action: "pacman -S linux linux-headers (ou linux-lts, linux-zen)".to_string(),
                is_mutating: true,
                risk: RiskLevel::High,
                prerequisites: vec!["Chroot preparado".to_string(), "Conexão de rede".to_string(), "Espaço em disco".to_string()],
            }],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
        return Ok(findings);
    }

    // Verifica se cada kernel tem initramfs correspondente
    for kernel in &kernels {
        let matching_initramfs: Vec<_> = initramfs
            .iter()
            .filter(|i| i.kernel_version == kernel.version)
            .collect();

        if matching_initramfs.is_empty() {
            findings.push(Finding {
                id: DiagnosticId::InitramfsNotFound,
                severity: Severity::Error,
                title: format!("Initramfs ausente para kernel {}", kernel.version),
                description: format!("Kernel {} encontrado mas nenhum initramfs correspondente detectado. O kernel não inicializará sem initramfs.", kernel.version),
                evidence: vec![Evidence {
                    kind: EvidenceKind::FileMissing,
                    description: format!("initramfs para kernel {}", kernel.version),
                    path: Some(kernel.path.clone()),
                    expected: Some(format!("initramfs-{}.img", kernel.version)),
                    observed: Some("Não encontrado".to_string()),
                    raw_data: HashMap::from([
                        ("kernel_version".to_string(), kernel.version.clone()),
                        ("kernel_path".to_string(), kernel.path.display().to_string()),
                    ]),
                }],
                recommendations: vec![
                    Recommendation {
                        id: "generate_initramfs".to_string(),
                        description: "Gerar initramfs para o kernel com mkinitcpio.".to_string(),
                        action: "mkinitcpio -p {}".to_string(),
                        is_mutating: true,
                        risk: RiskLevel::High,
                        prerequisites: vec!["Chroot preparado".to_string(), "Kernel instalado".to_string(), "/etc/mkinitcpio.conf válido".to_string()],
                    },
                ],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
        } else {
            // Verifica se tem variante fallback
            let has_fallback = matching_initramfs.iter().any(|i| i.variant == "fallback");
            if !has_fallback && kernel.is_default {
                findings.push(Finding {
                    id: DiagnosticId::InitramfsNotFound,
                    severity: Severity::Warning,
                    title: format!("Initramfs fallback ausente para kernel padrão {}", kernel.version),
                    description: "O kernel padrão não possui initramfs fallback. Recomendado para recuperação de boot.".to_string(),

                    evidence: vec![Evidence {
                        kind: EvidenceKind::FileMissing,
                        description: "initramfs fallback não encontrado".to_string(),
                        path: Some(kernel.path.clone()),
                        expected: Some(format!("initramfs-{}-fallback.img", kernel.version)),
                        observed: Some("Apenas initramfs default".to_string()),
                        raw_data: HashMap::from([
                            ("kernel_version".to_string(), kernel.version.clone()),
                        ]),
                    }],
                    recommendations: vec![Recommendation {
                        id: "generate_fallback_initramfs".to_string(),
                        description: "Gerar initramfs fallback com mkinitcpio.".to_string(),
                        action: "mkinitcpio -p {}".to_string(),
                        is_mutating: true,
                        risk: RiskLevel::Low,
                        prerequisites: vec!["Chroot preparado".to_string()],
                    }],
                    verified_in_chroot: chroot_root.is_some(),
                    context: HashMap::new(),
                });
            }
        }
    }

    // Verifica version mismatch entre kernels e initramfs órfãos
    for initrd in &initramfs {
        let has_kernel = kernels.iter().any(|k| k.version == initrd.kernel_version);
        if !has_kernel {
            findings.push(Finding {
                id: DiagnosticId::KernelInitramfsVersionMismatch,
                severity: Severity::Warning,
                title: format!(
                    "Initramfs órfão para kernel {} (kernel não instalado)",
                    initrd.kernel_version
                ),
                description: format!(
                    "Initramfs {} ({}) existe mas o kernel correspondente não foi encontrado.",
                    initrd.variant, initrd.kernel_version
                ),
                evidence: vec![Evidence {
                    kind: EvidenceKind::FileMissing,
                    description: "Kernel correspondente não encontrado".to_string(),
                    path: Some(initrd.path.clone()),
                    expected: Some(format!("vmlinuz-{}", initrd.kernel_version)),
                    observed: Some("Não encontrado".to_string()),
                    raw_data: HashMap::from([
                        ("initramfs_variant".to_string(), initrd.variant.clone()),
                        (
                            "initramfs_version".to_string(),
                            initrd.kernel_version.clone(),
                        ),
                    ]),
                }],
                recommendations: vec![Recommendation {
                    id: "remove_orphan_initramfs".to_string(),
                    description: "Remover initramfs órfão ou instalar kernel correspondente."
                        .to_string(),
                    action: "rm <initramfs_path> ou instalar kernel correspondente".to_string(),
                    is_mutating: true,
                    risk: RiskLevel::Low,
                    prerequisites: vec![],
                }],
                verified_in_chroot: chroot_root.is_some(),
                context: HashMap::new(),
            });
        }
    }

    // Sucesso: kernels e initramfs consistentes
    if findings.is_empty() {
        findings.push(Finding {
            id: DiagnosticId::KernelNotFound, // Info
            severity: Severity::Info,
            title: format!("{} kernel(s) e {} initramfs consistentes", kernels.len(), initramfs.len()),
            description: "Todos os kernels possuem initramfs correspondentes. Nenhum initramfs órfão detectado.".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::ContentMatches,
                description: "Kernels e initramfs consistentes".to_string(),
                path: Some(chroot_root.clone().unwrap_or_else(|| PathBuf::from("/")).join("boot")),
                expected: Some("Pares kernel/initramfs válidos".to_string()),
                observed: Some("OK".to_string()),
                raw_data: HashMap::from([
                    ("kernels".to_string(), kernels.iter().map(|k| k.version.clone()).collect::<Vec<_>>().join(", ")),
                    ("initramfs".to_string(), initramfs.iter().map(|i| format!("{} ({})", i.kernel_version, i.variant)).collect::<Vec<_>>().join(", ")),
                ]),
            }],
            recommendations: vec![],
            verified_in_chroot: chroot_root.is_some(),
            context: HashMap::new(),
        });
    }

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_kernel_version() {
        assert_eq!(
            extract_kernel_version("vmlinuz-6.12.1-arch1-1"),
            Some("6.12.1-arch1-1".to_string())
        );
        assert_eq!(
            extract_kernel_version("vmlinuz-linux"),
            Some("linux".to_string())
        );
        assert_eq!(
            extract_kernel_version("kernel-6.12.1"),
            Some("6.12.1".to_string())
        );
        assert_eq!(
            extract_kernel_version("Image-6.12.1"),
            Some("6.12.1".to_string())
        );
        assert_eq!(extract_kernel_version("vmlinuz"), Some("linux".to_string()));
        assert_eq!(extract_kernel_version("random-file"), None);
    }

    #[test]
    fn test_extract_initramfs_info() {
        assert_eq!(
            extract_initramfs_info("initramfs-6.12.1-arch1-1.img"),
            Some(("6.12.1-arch1-1".to_string(), "default".to_string()))
        );
        assert_eq!(
            extract_initramfs_info("initramfs-6.12.1-arch1-1-fallback.img"),
            Some(("6.12.1-arch1-1".to_string(), "fallback".to_string()))
        );
        assert_eq!(
            extract_initramfs_info("initramfs-6.12.1-arch1-1-lts.img"),
            Some(("6.12.1-arch1-1".to_string(), "lts".to_string()))
        );
        assert_eq!(
            extract_initramfs_info("initrd-6.12.1.img"),
            Some(("6.12.1".to_string(), "default".to_string()))
        );
        assert_eq!(
            extract_initramfs_info("initramfs-linux.img"),
            Some(("linux".to_string(), "default".to_string()))
        );
        assert_eq!(
            extract_initramfs_info("initramfs-linux-fallback.img"),
            Some(("linux".to_string(), "fallback".to_string()))
        );
        assert_eq!(extract_initramfs_info("random.img"), None);
    }

    #[test]
    fn test_version_cmp() {
        assert_eq!(version_cmp("6.12.1", "6.11.5"), std::cmp::Ordering::Greater);
        assert_eq!(version_cmp("6.11.5", "6.12.1"), std::cmp::Ordering::Less);
        assert_eq!(
            version_cmp("6.12.1-arch1", "6.12.1-arch2"),
            std::cmp::Ordering::Less
        );
        assert_eq!(version_cmp("5.15.0", "6.1.0"), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_is_kernel_file() {
        assert!(is_kernel_file("vmlinuz-6.12.1"));
        assert!(is_kernel_file("vmlinuz-linux"));
        assert!(is_kernel_file("kernel-6.12.1"));
        assert!(is_kernel_file("Image-6.12.1"));
        assert!(!is_kernel_file("initramfs-6.12.1.img"));
        assert!(!is_kernel_file("random.txt"));
    }

    #[test]
    fn test_is_initramfs_file() {
        assert!(is_initramfs_file("initramfs-6.12.1.img"));
        assert!(is_initramfs_file("initrd-6.12.1.img"));
        assert!(is_initramfs_file("initramfs-linux-fallback.img"));
        assert!(is_initramfs_file("initramfs.cpio.gz"));
        assert!(!is_initramfs_file("vmlinuz-6.12.1"));
        assert!(!is_initramfs_file("random.txt"));
    }
}
