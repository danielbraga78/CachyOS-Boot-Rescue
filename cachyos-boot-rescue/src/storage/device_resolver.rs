//! Resolução de identificadores de dispositivo (UUID, PARTUUID, LABEL, path).
//!
//! Converte especificações do fstab para caminhos de dispositivo reais
//! usando informações de descoberta.

use crate::discovery::types::BlockDevice;
use crate::storage::fstab::FstabDeviceSpec;
use std::collections::HashMap;
use std::path::PathBuf;

/// Resolvedor de especificações de dispositivo para caminhos reais.
#[derive(Debug, Clone)]
pub struct DeviceResolver {
    /// Mapa UUID -> device path
    uuid_map: HashMap<String, PathBuf>,
    /// Mapa PARTUUID -> device path
    partuuid_map: HashMap<String, PathBuf>,
    /// Mapa LABEL -> device path
    label_map: HashMap<String, PathBuf>,
    /// Mapa path -> device path (para paths diretos)
    path_map: HashMap<PathBuf, PathBuf>,
}

impl DeviceResolver {
    /// Cria resolvedor a partir de dispositivos descobertos.
    pub fn new(devices: &[BlockDevice]) -> Self {
        let mut resolver = Self {
            uuid_map: HashMap::new(),
            partuuid_map: HashMap::new(),
            label_map: HashMap::new(),
            path_map: HashMap::new(),
        };

        // Função recursiva para coletar todos os dispositivos
        fn collect_devices<'a>(dev: &'a BlockDevice, out: &mut Vec<&'a BlockDevice>) {
            out.push(dev);
            for child in &dev.children {
                collect_devices(child, out);
            }
        }

        let mut all_devices = Vec::new();
        for dev in devices {
            collect_devices(dev, &mut all_devices);
        }

        for dev in all_devices {
            // Path direto
            resolver.path_map.insert(dev.path.clone(), dev.path.clone());

            // UUID
            if let Some(ref uuid) = dev.uuid {
                resolver.uuid_map.insert(uuid.clone(), dev.path.clone());
            }

            // PARTUUID
            if let Some(ref partuuid) = dev.partuuid {
                resolver
                    .partuuid_map
                    .insert(partuuid.clone(), dev.path.clone());
            }

            // LABEL
            if let Some(ref label) = dev.label {
                resolver.label_map.insert(label.clone(), dev.path.clone());
            }
        }

        resolver
    }

    /// Resolve uma especificação para caminho de dispositivo.
    pub fn resolve(&self, spec: &FstabDeviceSpec) -> Option<PathBuf> {
        match spec {
            FstabDeviceSpec::Uuid(uuid) => self.uuid_map.get(uuid).cloned(),
            FstabDeviceSpec::PartUuid(partuuid) => self.partuuid_map.get(partuuid).cloned(),
            FstabDeviceSpec::Label(label) => self.label_map.get(label).cloned(),
            FstabDeviceSpec::Path(path) => self.path_map.get(path).cloned(),
        }
    }

    /// Resolve múltiplas especificações.
    pub fn resolve_all(&self, specs: &[FstabDeviceSpec]) -> HashMap<FstabDeviceSpec, PathBuf> {
        let mut result = HashMap::new();
        for spec in specs {
            if let Some(path) = self.resolve(spec) {
                result.insert(spec.clone(), path);
            }
        }
        result
    }

    /// Verifica se uma especificação pode ser resolvida.
    pub fn can_resolve(&self, spec: &FstabDeviceSpec) -> bool {
        self.resolve(spec).is_some()
    }

    /// Retorna todos os UUIDs conhecidos.
    pub fn known_uuids(&self) -> Vec<String> {
        self.uuid_map.keys().cloned().collect()
    }

    /// Retorna todos os PARTUUIDs conhecidos.
    pub fn known_partuuids(&self) -> Vec<String> {
        self.partuuid_map.keys().cloned().collect()
    }

    /// Retorna todos os LABELs conhecidos.
    pub fn known_labels(&self) -> Vec<String> {
        self.label_map.keys().cloned().collect()
    }
}

/// Função de conveniência para resolver specs usando dispositivos descobertos.
pub fn resolve_device_specs(
    specs: &[FstabDeviceSpec],
    devices: &[BlockDevice],
) -> HashMap<FstabDeviceSpec, PathBuf> {
    let resolver = DeviceResolver::new(devices);
    resolver.resolve_all(specs)
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
            filesystem: Some(FilesystemType::Ext4),
            mountpoint: None,
            mountpoints: vec![],
            mount_options: vec![],
            parent: Some("sda".to_string()),
            sysfs_path: None,
            children: vec![],
        }
    }

    #[test]
    fn test_device_resolver_uuid() {
        let devices = vec![
            create_test_device(
                "/dev/sda1",
                Some("uuid-root"),
                Some("partuuid-root"),
                Some("ROOT"),
            ),
            create_test_device(
                "/dev/sda2",
                Some("uuid-home"),
                Some("partuuid-home"),
                Some("HOME"),
            ),
        ];

        let resolver = DeviceResolver::new(&devices);
        let resolved = resolver.resolve(&FstabDeviceSpec::Uuid("uuid-root".to_string()));
        assert_eq!(resolved, Some(PathBuf::from("/dev/sda1")));

        let not_found = resolver.resolve(&FstabDeviceSpec::Uuid("uuid-nonexistent".to_string()));
        assert_eq!(not_found, None);
    }

    #[test]
    fn test_device_resolver_partuuid() {
        let devices = vec![create_test_device(
            "/dev/sda1",
            None,
            Some("partuuid-boot"),
            None,
        )];
        let resolver = DeviceResolver::new(&devices);
        let resolved = resolver.resolve(&FstabDeviceSpec::PartUuid("partuuid-boot".to_string()));
        assert_eq!(resolved, Some(PathBuf::from("/dev/sda1")));
    }

    #[test]
    fn test_device_resolver_label() {
        let devices = vec![create_test_device(
            "/dev/sda1",
            None,
            None,
            Some("MY_LABEL"),
        )];
        let resolver = DeviceResolver::new(&devices);
        let resolved = resolver.resolve(&FstabDeviceSpec::Label("MY_LABEL".to_string()));
        assert_eq!(resolved, Some(PathBuf::from("/dev/sda1")));
    }

    #[test]
    fn test_device_resolver_path() {
        let devices = vec![create_test_device("/dev/sda1", None, None, None)];
        let resolver = DeviceResolver::new(&devices);
        let resolved = resolver.resolve(&FstabDeviceSpec::Path(PathBuf::from("/dev/sda1")));
        assert_eq!(resolved, Some(PathBuf::from("/dev/sda1")));
    }

    #[test]
    fn test_device_resolver_resolve_all() {
        let devices = vec![
            create_test_device(
                "/dev/sda1",
                Some("uuid1"),
                Some("partuuid1"),
                Some("LABEL1"),
            ),
            create_test_device(
                "/dev/sda2",
                Some("uuid2"),
                Some("partuuid2"),
                Some("LABEL2"),
            ),
        ];

        let specs = vec![
            FstabDeviceSpec::Uuid("uuid1".to_string()),
            FstabDeviceSpec::PartUuid("partuuid2".to_string()),
            FstabDeviceSpec::Label("LABEL1".to_string()),
            FstabDeviceSpec::Path(PathBuf::from("/dev/nonexistent")),
        ];

        let resolver = DeviceResolver::new(&devices);
        let resolved = resolver.resolve_all(&specs);

        assert_eq!(resolved.len(), 3); // Path inexistente não resolve
        assert_eq!(
            resolved.get(&FstabDeviceSpec::Uuid("uuid1".to_string())),
            Some(&PathBuf::from("/dev/sda1"))
        );
        assert_eq!(
            resolved.get(&FstabDeviceSpec::PartUuid("partuuid2".to_string())),
            Some(&PathBuf::from("/dev/sda2"))
        );
        assert_eq!(
            resolved.get(&FstabDeviceSpec::Label("LABEL1".to_string())),
            Some(&PathBuf::from("/dev/sda1"))
        );
    }

    #[test]
    fn test_device_resolver_can_resolve() {
        let devices = vec![create_test_device("/dev/sda1", Some("uuid1"), None, None)];
        let resolver = DeviceResolver::new(&devices);
        assert!(resolver.can_resolve(&FstabDeviceSpec::Uuid("uuid1".to_string())));
        assert!(!resolver.can_resolve(&FstabDeviceSpec::Uuid("uuid2".to_string())));
    }
}
