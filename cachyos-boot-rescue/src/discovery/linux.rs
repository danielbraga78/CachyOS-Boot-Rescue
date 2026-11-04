//! Identificação de instalações Linux candidatas.
//!
//! Usa marcadores como /etc/os-release, /etc/fstab e diretórios de boot
//! quando puderem ser inspecionados com segurança (já montados).
//! Não monta partições - limita-se a dispositivos já montados ou metadados disponíveis.

use crate::discovery::types::{
    BlockDevice, DeviceType, EspCandidate, FilesystemInfo, FilesystemType,
    LinuxInstallationCandidate,
};
use std::path::{Path, PathBuf};

/// Marcadores de instalação Linux.
const OS_RELEASE_PATH: &str = "etc/os-release";
const FSTAB_PATH: &str = "etc/fstab";
const BOOT_DIRS: &[&str] = &["boot", "efi", "boot/efi"];

/// Tenta identificar instalações Linux a partir de dispositivos já montados.
pub fn identify_linux_installations(
    devices: &[BlockDevice],
    esp_candidates: &[EspCandidate],
) -> Vec<LinuxInstallationCandidate> {
    identify_linux_installations_with_root(devices, esp_candidates, Path::new("/"))
}

/// Versão interna que permite especificar o caminho raiz.
pub fn identify_linux_installations_with_root(
    devices: &[BlockDevice],
    esp_candidates: &[EspCandidate],
    root_path: &Path,
) -> Vec<LinuxInstallationCandidate> {
    let mut candidates = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    fn collect_mounted_partitions_recursive(
        device: &BlockDevice,
        esp_candidates: &[EspCandidate],
        root_path: &Path,
        candidates: &mut Vec<LinuxInstallationCandidate>,
        seen_paths: &mut std::collections::HashSet<PathBuf>,
    ) {
        // Verifica se este dispositivo tem filesystem Linux nativo montado no caminho raiz
        // Aceita qualquer tipo de dispositivo (Partition, Crypto/dm, LVM, etc) que possa ser root
        if let Some(fs) = device.filesystem
            && fs.is_linux_native()
            && let Some(mp) = &device.mountpoint
            && mp == root_path
            && let Some(candidate) = analyze_root_candidate(device, esp_candidates)
        {
            let path = device.path.clone();
            if seen_paths.insert(path) {
                candidates.push(candidate);
            }
        }

        // Recursivamente verifica children
        for child in &device.children {
            collect_mounted_partitions_recursive(
                child,
                esp_candidates,
                root_path,
                candidates,
                seen_paths,
            );
        }
    }

    for device in devices {
        collect_mounted_partitions_recursive(
            device,
            esp_candidates,
            root_path,
            &mut candidates,
            &mut seen_paths,
        );
    }

    // Ordena por confiança (maior primeiro)
    candidates.sort_by_key(|a| std::cmp::Reverse(a.confidence));

    candidates
}

/// Analisa uma partição candidata a raiz (/).
fn analyze_root_candidate(
    root_device: &BlockDevice,
    esp_candidates: &[EspCandidate],
) -> Option<LinuxInstallationCandidate> {
    let mountpoint = root_device.mountpoint.as_ref()?;

    let mut reasons = Vec::new();
    let mut confidence: u8 = 30; // Base por ter filesystem Linux e estar montado

    // Verifica /etc/os-release
    let os_release_path = mountpoint.join(OS_RELEASE_PATH);
    let (distribution, version, distribution_id, has_os_release) =
        parse_os_release(&os_release_path);
    if has_os_release {
        confidence += 30;
        reasons.push("Encontrado /etc/os-release".to_string());
    }

    // Verifica /etc/fstab
    let fstab_path = mountpoint.join(FSTAB_PATH);
    let (has_fstab, fstab_content) = read_fstab(&fstab_path);
    if has_fstab {
        confidence += 20;
        reasons.push("Encontrado /etc/fstab".to_string());
    }

    // Verifica diretórios de boot
    let (kernel_paths, initramfs_paths) = find_kernels_and_initramfs(mountpoint);
    if !kernel_paths.is_empty() {
        confidence += 15;
        reasons.push(format!("Kernels encontrados: {}", kernel_paths.len()));
    }

    // Associa partições de boot/ESP
    let boot_partitions = associate_boot_partitions(root_device, esp_candidates, mountpoint);

    // Verifica se é Btrfs com subvolume
    let (is_btrfs_subvolume, btrfs_subvolume) = detect_btrfs_subvolume(root_device, mountpoint);

    // Partições adicionais do mesmo disco
    let additional_partitions = find_additional_partitions(root_device);

    // Informações de filesystem
    let root_filesystem = FilesystemInfo {
        fstype: root_device.filesystem.unwrap_or(FilesystemType::Unknown),
        uuid: root_device.uuid.clone(),
        label: root_device.label.clone(),
        partuuid: root_device.partuuid.clone(),
        version: None,
        subsystems_uuid: None,
        subsystems_type: None,
    };

    // Ajusta confiança baseado em evidências
    if distribution_id.is_some() {
        confidence += 5;
    }

    Some(LinuxInstallationCandidate {
        root_device: root_device.clone(),
        root_filesystem,
        boot_partitions,
        additional_partitions,
        distribution,
        version,
        distribution_id,
        kernel_paths,
        initramfs_paths,
        has_fstab,
        fstab_content,
        has_os_release,
        is_btrfs_subvolume,
        btrfs_subvolume,
        confidence: confidence.min(100),
        reasons,
    })
}

/// Parseia /etc/os-release.
pub fn parse_os_release(path: &Path) -> (Option<String>, Option<String>, Option<String>, bool) {
    if !path.exists() {
        return (None, None, None, false);
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (None, None, None, false),
    };

    let mut distribution = None;
    let mut version = None;
    let mut distribution_id = None;

    for line in content.lines() {
        let line = line.trim();
        if let Some(stripped) = line.strip_prefix("PRETTY_NAME=") {
            distribution = Some(stripped.trim_matches('"').to_string());
        } else if let Some(stripped) = line
            .strip_prefix("VERSION=")
            .or_else(|| line.strip_prefix("VERSION_ID="))
        {
            version = Some(stripped.trim_matches('"').to_string());
        } else if let Some(stripped) = line.strip_prefix("ID=") {
            distribution_id = Some(stripped.trim_matches('"').to_string());
        } else if let Some(stripped) = line
            .strip_prefix("ID_LIKE=")
            .filter(|_| distribution_id.is_none())
        {
            distribution_id = Some(stripped.trim_matches('"').to_string());
        }
    }

    (distribution, version, distribution_id, true)
}

/// Lê /etc/fstab.
pub fn read_fstab(path: &Path) -> (bool, Option<String>) {
    if !path.exists() {
        return (false, None);
    }

    match std::fs::read_to_string(path) {
        Ok(content) => (true, Some(content)),
        Err(_) => (false, None),
    }
}

/// Encontra kernels e initramfs em /boot.
pub fn find_kernels_and_initramfs(root_mount: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut kernels = Vec::new();
    let mut initramfs = Vec::new();

    // Verifica /boot no ponto de montagem raiz
    let boot_path = root_mount.join("boot");
    if boot_path.exists() {
        scan_boot_dir(&boot_path, &mut kernels, &mut initramfs);
    }

    // Verifica /efi e /boot/efi se montados separadamente
    for dir in BOOT_DIRS {
        let path = root_mount.join(dir);
        if path.exists() && path != boot_path {
            scan_boot_dir(&path, &mut kernels, &mut initramfs);
        }
    }

    (kernels, initramfs)
}

/// Escaneia diretório de boot por kernels e initramfs.
fn scan_boot_dir(dir: &Path, kernels: &mut Vec<PathBuf>, initramfs: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if name.starts_with("vmlinuz") || name.starts_with("kernel") || name == "vmlinuz" {
                kernels.push(path);
            } else if name.starts_with("initramfs") || name.starts_with("initrd") {
                initramfs.push(path);
            } else if name.ends_with(".efi") && (name.contains("linux") || name.contains("grub")) {
                // Kernels EFI unificados
                kernels.push(path);
            }
        }
    }
}

/// Associa partições de boot/ESP à instalação raiz.
fn associate_boot_partitions(
    root_device: &BlockDevice,
    esp_candidates: &[EspCandidate],
    root_mount: &Path,
) -> Vec<EspCandidate> {
    let mut associated = Vec::new();

    for esp in esp_candidates {
        // Verifica se a ESP está montada em um ponto de montagem sob a raiz
        if let Some(esp_mount) = &esp.mountpoint {
            // Normaliza caminhos para comparação
            let esp_path = esp_mount.to_string_lossy();
            let root_path = root_mount.to_string_lossy();

            // Se ESP está montada dentro da raiz (ex: /boot/efi, /efi)
            if esp_path.starts_with(&*root_path) {
                associated.push(esp.clone());
                continue;
            }
        }

        // Verifica se é do mesmo disco pai
        if let (Some(root_parent), Some(esp_parent)) = (&root_device.parent, &esp.device.parent)
            && root_parent == esp_parent
        {
            associated.push(esp.clone());
        }
    }

    associated
}

/// Detecta se é Btrfs com subvolume.
fn detect_btrfs_subvolume(root_device: &BlockDevice, root_mount: &Path) -> (bool, Option<String>) {
    if root_device.filesystem != Some(FilesystemType::Btrfs) {
        return (false, None);
    }

    // Tenta ler /proc/self/mountinfo para encontrar subvolume
    if let Ok(content) = std::fs::read_to_string("/proc/self/mountinfo") {
        let root_path = root_mount.to_string_lossy();
        for line in content.lines() {
            if line.contains(&*root_path) {
                // Formato: mount_id parent_id major:min root mountpoint ...
                // Procura por subvol= ou subvolid=
                if let Some(subvol_idx) = line.find("subvol=") {
                    let subvol = &line[subvol_idx + 7..];
                    let subvol = subvol.split_whitespace().next().unwrap_or("");
                    if !subvol.is_empty() && subvol != "/" {
                        return (true, Some(subvol.to_string()));
                    }
                }
            }
        }
    }

    // Verifica se há subvolumes típicos no ponto de montagem
    let subvols = ["@", "@root", "root"];
    for subvol in subvols {
        if root_mount.join(subvol).exists() {
            return (true, Some(subvol.to_string()));
        }
    }

    (false, None)
}

/// Encontra partições adicionais do mesmo disco.
fn find_additional_partitions(root_device: &BlockDevice) -> Vec<BlockDevice> {
    let additional = Vec::new();

    // Se temos o pai, procura irmãos
    if let Some(parent_name) = &root_device.parent {
        // Não temos acesso direto ao pai aqui, mas podemos retornar vazio
        // A associação completa seria feita no nível superior
        let _ = parent_name;
    }

    additional
}

/// Tenta ler metadados de instalação de dispositivos não montados via blkid/lsblk.
/// Não monta - apenas usa metadados já disponíveis.
/// Não considera partições swap como instalações candidatas.
/// Percorre recursivamente todos os dispositivos (incluindo children) para encontrar partições aninhadas.
pub fn identify_from_metadata(devices: &[BlockDevice]) -> Vec<LinuxInstallationCandidate> {
    let mut candidates = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    fn collect_partitions_recursive(
        device: &BlockDevice,
        candidates: &mut Vec<LinuxInstallationCandidate>,
        seen_paths: &mut std::collections::HashSet<PathBuf>,
    ) {
        // Verifica se este dispositivo é uma partição candidata
        if device.device_type == DeviceType::Partition
            && device
                .partition_type
                .as_ref()
                .is_some_and(|p| p.is_linux_installation())
            && (device.uuid.is_some() || device.partuuid.is_some())
        {
            let path = device.path.clone();
            if seen_paths.insert(path) {
                let reasons = vec!["Partição Linux detectada via metadados".to_string()];
                let confidence = 15;

                let root_filesystem = FilesystemInfo {
                    fstype: device.filesystem.unwrap_or(FilesystemType::Unknown),
                    uuid: device.uuid.clone(),
                    label: device.label.clone(),
                    partuuid: device.partuuid.clone(),
                    version: None,
                    subsystems_uuid: None,
                    subsystems_type: None,
                };

                candidates.push(LinuxInstallationCandidate {
                    root_device: device.clone(),
                    root_filesystem,
                    boot_partitions: Vec::new(),
                    additional_partitions: Vec::new(),
                    distribution: None,
                    version: None,
                    distribution_id: None,
                    kernel_paths: Vec::new(),
                    initramfs_paths: Vec::new(),
                    has_fstab: false,
                    fstab_content: None,
                    has_os_release: false,
                    is_btrfs_subvolume: false,
                    btrfs_subvolume: None,
                    confidence,
                    reasons,
                });
            }
        }

        // Recursivamente verifica children
        for child in &device.children {
            collect_partitions_recursive(child, candidates, seen_paths);
        }
    }

    for device in devices {
        collect_partitions_recursive(device, &mut candidates, &mut seen_paths);
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::types::{
        BlockDevice, DeviceId, DeviceType, FilesystemType, PartitionType,
    };
    use std::path::PathBuf;

    fn create_test_device(path: &str, fs: FilesystemType, mount: Option<&str>) -> BlockDevice {
        // Define partition_type baseado no filesystem
        let partition_type = match fs {
            FilesystemType::Swap => PartitionType::LinuxSwap,
            FilesystemType::Ntfs => PartitionType::MicrosoftBasicData,
            FilesystemType::Fat32
            | FilesystemType::Fat16
            | FilesystemType::Fat12
            | FilesystemType::Vfat => PartitionType::EfiSystem,
            _ => PartitionType::LinuxFilesystem,
        };

        BlockDevice {
            path: PathBuf::from(path),
            name: path.trim_start_matches("/dev/").to_string(),
            device_id: Some(DeviceId::from_major_minor(0, 0)),
            device_type: DeviceType::Partition,
            size: Some(10 * 1024 * 1024 * 1024),
            sector_size: Some(512),
            logical_sector_size: Some(512),
            model: None,
            serial: None,
            vendor: None,
            rotation_rate: None,
            removable: Some(false),
            read_only: Some(false),
            partition_table: None,
            uuid: Some("test-uuid".to_string()),
            partuuid: Some("test-partuuid".to_string()),
            label: Some("test-label".to_string()),
            partition_type: Some(partition_type),
            filesystem: Some(fs),
            mountpoint: mount.map(PathBuf::from),
            mountpoints: vec![],
            mount_options: vec![],
            parent: Some("sda".to_string()),
            sysfs_path: None,
            children: vec![],
        }
    }

    fn create_esp_candidate(path: &str, mountpoint: Option<&str>) -> EspCandidate {
        let esp = BlockDevice {
            path: PathBuf::from(path),
            name: path.trim_start_matches("/dev/").to_string(),
            device_id: Some(DeviceId::from_major_minor(0, 0)),
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
            uuid: Some("test-uuid".to_string()),
            partuuid: Some("test-partuuid".to_string()),
            label: Some("EFI".to_string()),
            partition_type: Some(PartitionType::EfiSystem),
            filesystem: Some(FilesystemType::Vfat),
            mountpoint: mountpoint.map(PathBuf::from),
            mountpoints: vec![],
            mount_options: vec![],
            parent: Some("sda".to_string()),
            sysfs_path: None,
            children: vec![],
        };
        let fs_info = FilesystemInfo {
            fstype: FilesystemType::Vfat,
            uuid: Some("ESP-UUID".to_string()),
            label: Some("EFI".to_string()),
            partuuid: esp.partuuid.clone(),
            version: None,
            subsystems_uuid: None,
            subsystems_type: None,
        };
        EspCandidate {
            device: esp,
            filesystem: fs_info,
            mountpoint: mountpoint.map(PathBuf::from),
            is_boot_efi_mount: mountpoint == Some("/boot/efi") || mountpoint == Some("/efi"),
            is_boot_mount: mountpoint == Some("/boot"),
            size: 512 * 1024 * 1024,
            free_space: None,
            confidence: 90,
            reasons: vec!["test".to_string()],
        }
    }

    #[test]
    fn test_identify_linux_installations_empty() {
        let devices = vec![];
        let esps = vec![];
        let result = identify_linux_installations(&devices, &esps);
        assert!(result.is_empty());
    }

    #[test]
    fn test_identify_linux_installations_non_linux_fs() {
        let devices = vec![create_test_device(
            "/dev/sda1",
            FilesystemType::Ntfs,
            Some("/mnt/windows"),
        )];
        let esps = vec![];
        let result = identify_linux_installations(&devices, &esps);
        assert!(result.is_empty());
    }

    #[test]
    fn test_identify_linux_installations_not_mounted() {
        let devices = vec![create_test_device(
            "/dev/sda1",
            FilesystemType::Ext4,
            None, // não montada
        )];
        let esps = vec![];
        let result = identify_linux_installations(&devices, &esps);
        assert!(result.is_empty());
    }

    #[test]
    fn test_identify_linux_installations_mounted_ext4() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        // Cria /etc/os-release
        std::fs::create_dir_all(root.join("etc")).unwrap();
        std::fs::write(
            root.join("etc/os-release"),
            r#"PRETTY_NAME="CachyOS"
VERSION_ID="2026.11"
ID=cachyos
ID_LIKE=arch"#,
        )
        .unwrap();

        // Cria /etc/fstab
        std::fs::write(
            root.join("etc/fstab"),
            "UUID=xxx / ext4 defaults 0 1\nUUID=yyy /boot/efi vfat defaults 0 2\n",
        )
        .unwrap();

        // Cria kernels em /boot
        let boot = root.join("boot");
        std::fs::create_dir_all(&boot).unwrap();
        std::fs::write(boot.join("vmlinuz-linux"), b"kernel").unwrap();
        std::fs::write(boot.join("initramfs-linux.img"), b"initramfs").unwrap();

        let devices = vec![create_test_device(
            "/dev/sda2",
            FilesystemType::Ext4,
            Some(root.to_str().unwrap()),
        )];

        let esps = vec![create_esp_candidate("/dev/sda1", Some("/boot/efi"))];

        let result = identify_linux_installations_with_root(&devices, &esps, root.as_ref());
        assert_eq!(result.len(), 1);

        let candidate = &result[0];
        assert_eq!(candidate.root_device.path, PathBuf::from("/dev/sda2"));
        assert_eq!(candidate.distribution, Some("CachyOS".to_string()));
        assert_eq!(candidate.version, Some("2026.11".to_string()));
        assert_eq!(candidate.distribution_id, Some("cachyos".to_string()));
        assert!(candidate.has_os_release);
        assert!(candidate.has_fstab);
        assert_eq!(candidate.kernel_paths.len(), 1);
        assert_eq!(candidate.initramfs_paths.len(), 1);
        assert_eq!(candidate.boot_partitions.len(), 1);
        assert!(candidate.confidence > 50);
    }

    #[test]
    fn test_identify_linux_installations_btrfs_subvolume() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        std::fs::create_dir_all(root.join("etc")).unwrap();
        std::fs::write(root.join("etc/os-release"), "ID=arch\n").unwrap();

        // Simula subvolume @
        std::fs::create_dir_all(root.join("@")).unwrap();

        let devices = vec![create_test_device(
            "/dev/sda2",
            FilesystemType::Btrfs,
            Some(root.to_str().unwrap()),
        )];

        let esps = vec![];

        let result = identify_linux_installations_with_root(&devices, &esps, root.as_ref());
        assert_eq!(result.len(), 1);

        let candidate = &result[0];
        assert!(candidate.is_btrfs_subvolume);
        // A detecção via mountinfo não funciona em teste, mas verifica diretório @
        assert_eq!(candidate.btrfs_subvolume, Some("@".to_string()));
    }

    #[test]
    fn test_identify_from_metadata() {
        let devices = vec![
            create_test_device(
                "/dev/sda1",
                FilesystemType::Ext4,
                None, // não montada
            ),
            create_test_device("/dev/sda2", FilesystemType::Ntfs, None),
            create_test_device("/dev/sda3", FilesystemType::Swap, None),
        ];

        let result = identify_from_metadata(&devices);
        // Deve encontrar apenas sda1 (Linux filesystem). Swap NÃO é instalação candidata.
        assert_eq!(result.len(), 1);

        let root_candidate = result
            .iter()
            .find(|c| c.root_device.path.as_os_str() == "/dev/sda1");
        assert!(root_candidate.is_some());
        assert_eq!(root_candidate.unwrap().confidence, 15);
        assert!(
            root_candidate
                .unwrap()
                .reasons
                .iter()
                .any(|r| r.contains("metadados"))
        );
    }

    #[test]
    fn test_parse_os_release() {
        let dir = tempfile::tempdir().unwrap();
        let os_release = dir.path().join("os-release");
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(
            &os_release,
            r#"PRETTY_NAME="CachyOS"
VERSION_ID="2026.11"
ID=cachyos
ID_LIKE=arch"#,
        )
        .unwrap();

        let (dist, ver, id, has) = parse_os_release(&os_release);
        assert!(has);
        assert_eq!(dist, Some("CachyOS".to_string()));
        assert_eq!(ver, Some("2026.11".to_string()));
        assert_eq!(id, Some("cachyos".to_string()));
    }

    #[test]
    fn test_parse_os_release_missing() {
        let dir = tempfile::tempdir().unwrap();
        let os_release = dir.path().join("os-release");
        // Não cria o arquivo

        let (dist, ver, id, has) = parse_os_release(&os_release);
        assert!(!has);
        assert_eq!(dist, None);
        assert_eq!(ver, None);
        assert_eq!(id, None);
    }

    #[test]
    fn test_read_fstab() {
        let dir = tempfile::tempdir().unwrap();
        let fstab = dir.path().join("fstab");
        std::fs::write(
            &fstab,
            "UUID=xxx / ext4 defaults 0 1\nUUID=yyy /boot/efi vfat defaults 0 2\n",
        )
        .unwrap();

        let (has, content) = read_fstab(&fstab);
        assert!(has);
        let content = content.unwrap();
        assert!(content.contains("UUID=xxx"));
        assert!(content.contains("/boot/efi"));
    }

    #[test]
    fn test_read_fstab_missing() {
        let dir = tempfile::tempdir().unwrap();
        let fstab = dir.path().join("fstab");
        // Não cria

        let (has, content) = read_fstab(&fstab);
        assert!(!has);
        assert!(content.is_none());
    }

    #[test]
    fn test_find_kernels_and_initramfs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let boot = root.join("boot");
        std::fs::create_dir_all(&boot).unwrap();
        std::fs::write(boot.join("vmlinuz-linux"), b"kernel").unwrap();
        std::fs::write(boot.join("vmlinuz-linux-lts"), b"kernel-lts").unwrap();
        std::fs::write(boot.join("initramfs-linux.img"), b"initramfs").unwrap();
        std::fs::write(boot.join("initramfs-linux-lts.img"), b"initramfs-lts").unwrap();
        std::fs::write(boot.join("config.txt"), b"config").unwrap(); // não kernel

        let (kernels, initramfs) = find_kernels_and_initramfs(root);
        assert_eq!(kernels.len(), 2);
        assert_eq!(initramfs.len(), 2);

        // Verifica nomes
        let kernel_names: Vec<_> = kernels
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();
        assert!(kernel_names.contains(&"vmlinuz-linux"));
        assert!(kernel_names.contains(&"vmlinuz-linux-lts"));

        let initrd_names: Vec<_> = initramfs
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();
        assert!(initrd_names.contains(&"initramfs-linux.img"));
        assert!(initrd_names.contains(&"initramfs-linux-lts.img"));
    }

    #[test]
    fn test_find_kernels_efi_unified() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let efi = root.join("efi");
        std::fs::create_dir_all(&efi).unwrap();
        std::fs::write(efi.join("linux.efi"), b"unified kernel").unwrap();

        let (kernels, initramfs) = find_kernels_and_initramfs(root);
        assert_eq!(kernels.len(), 1);
        assert_eq!(initramfs.len(), 0);
        assert_eq!(kernels[0].file_name().unwrap(), "linux.efi");
    }

    #[test]
    fn test_identify_from_metadata_recursive() {
        // Testa que partições aninhadas (children de children) são encontradas
        let mut disk = create_test_device("/dev/sda", FilesystemType::Unknown, None);
        disk.device_type = DeviceType::Disk;

        let mut part1 = create_test_device("/dev/sda1", FilesystemType::Vfat, None);
        part1.partition_type = Some(PartitionType::EfiSystem);
        part1.filesystem = Some(FilesystemType::Vfat);

        let mut part2 = create_test_device("/dev/sda2", FilesystemType::Ext4, None);
        part2.partition_type = Some(PartitionType::LinuxFilesystem);

        // Partição aninhada (simulando LVM ou RAID)
        let mut nested_disk = create_test_device("/dev/mapper/vg-root", FilesystemType::Ext4, None);
        nested_disk.device_type = DeviceType::Partition;
        nested_disk.partition_type = Some(PartitionType::LinuxFilesystem);
        nested_disk.parent = Some("vg".to_string());

        part2.children = vec![nested_disk];

        disk.children = vec![part1, part2];

        let result = identify_from_metadata(&[disk]);
        // Deve encontrar sda2 e a partição aninhada
        assert_eq!(result.len(), 2);
        let paths: Vec<_> = result
            .iter()
            .map(|c| c.root_device.path.to_string_lossy().to_string())
            .collect();
        assert!(paths.contains(&"/dev/sda2".to_string()));
        assert!(paths.contains(&"/dev/mapper/vg-root".to_string()));
    }

    #[test]
    fn test_identify_linux_installations_with_root_recursive() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        std::fs::create_dir_all(root.join("etc")).unwrap();
        std::fs::write(root.join("etc/os-release"), "ID=arch\n").unwrap();
        std::fs::write(root.join("etc/fstab"), "UUID=xxx / ext4 defaults 0 1\n").unwrap();

        let boot = root.join("boot");
        std::fs::create_dir_all(&boot).unwrap();
        std::fs::write(boot.join("vmlinuz-linux"), b"kernel").unwrap();

        // Disco com partição aninhada
        let mut disk = create_test_device("/dev/sda", FilesystemType::Unknown, None);
        disk.device_type = DeviceType::Disk;

        let mut part1 = create_test_device("/dev/sda1", FilesystemType::Vfat, Some("/boot/efi"));
        part1.partition_type = Some(PartitionType::EfiSystem);
        part1.filesystem = Some(FilesystemType::Vfat);

        let mut part2 = create_test_device(
            "/dev/sda2",
            FilesystemType::Ext4,
            Some(root.to_str().unwrap()),
        );
        part2.partition_type = Some(PartitionType::LinuxFilesystem);

        // Partição aninhada também montada em /
        let mut nested_part = create_test_device(
            "/dev/mapper/luks-root",
            FilesystemType::Ext4,
            Some(root.to_str().unwrap()),
        );
        nested_part.partition_type = Some(PartitionType::LinuxFilesystem);
        nested_part.parent = Some("luks".to_string());

        part2.children = vec![nested_part];
        disk.children = vec![part1, part2];

        let esps = vec![]; // Não testar ESPs aqui
        let result = identify_linux_installations_with_root(&[disk], &esps, root.as_ref());

        // Deve encontrar ambas as partições montadas em / (sem duplicatas)
        assert_eq!(result.len(), 2);
        let paths: Vec<_> = result
            .iter()
            .map(|c| c.root_device.path.to_string_lossy().to_string())
            .collect();
        assert!(paths.contains(&"/dev/sda2".to_string()));
        assert!(paths.contains(&"/dev/mapper/luks-root".to_string()));
    }

    #[test]
    fn test_identify_from_metadata_no_duplicates() {
        // Testa que partições duplicadas (mesmo path) não são adicionadas duas vezes
        let mut disk = create_test_device("/dev/sda", FilesystemType::Unknown, None);
        disk.device_type = DeviceType::Disk;

        let mut part1 = create_test_device("/dev/sda1", FilesystemType::Ext4, None);
        part1.partition_type = Some(PartitionType::LinuxFilesystem);

        // Mesma partição referenciada em dois lugares (simulação)
        let part1_clone = part1.clone();
        disk.children = vec![part1, part1_clone];

        let result = identify_from_metadata(&[disk]);
        // Deve encontrar apenas uma vez
        assert_eq!(result.len(), 1);
    }
}
