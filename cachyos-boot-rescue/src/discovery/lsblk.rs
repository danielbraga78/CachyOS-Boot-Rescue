//! Adaptador para consultar `lsblk` em JSON.
//!
//! Usa argumentos fixos e parsing robusto. Não analisa saída humana/localizada.

use crate::discovery::types::{
    BlockDevice, CommandConfig, CommandOutput, DeviceId, DeviceType, FilesystemType,
    PartitionTableType, PartitionType,
};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use tracing::{debug, warn};

/// Estrutura JSON retornada pelo lsblk --json.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LsblkJson {
    blockdevices: Vec<LsblkDevice>,
}

/// Dispositivo individual no JSON do lsblk.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LsblkDevice {
    name: String,
    path: Option<String>,
    #[serde(rename = "maj:min")]
    maj_min: Option<String>,
    #[serde(rename = "type")]
    device_type: String,
    size: Option<serde_json::Value>,
    sectorsize: Option<serde_json::Value>,
    logicalsectorsize: Option<serde_json::Value>,
    model: Option<String>,
    serial: Option<String>,
    vendor: Option<String>,
    rota: Option<serde_json::Value>,
    rm: Option<bool>,
    ro: Option<bool>,
    fstype: Option<String>,
    label: Option<String>,
    uuid: Option<String>,
    partuuid: Option<String>,
    parttype: Option<String>,
    parttypename: Option<String>,
    mountpoint: Option<String>,
    mountpoints: Option<Vec<Option<String>>>,
    mountoptions: Option<Vec<String>>,
    pkname: Option<String>,
    pttype: Option<String>,
    children: Option<Vec<LsblkDevice>>,
}

/// Executa `lsblk` com argumentos fixos para saída JSON.
pub fn run_lsblk(config: &CommandConfig) -> Result<CommandOutput> {
    let args = [
        "--json",
        "--bytes",
        "--output",
        "NAME,PATH,MAJ:MIN,TYPE,SIZE,SECTORSIZE,LOGICAL_SECTOR_SIZE,MODEL,SERIAL,VENDOR,ROTA,RM,RO,FSTYPE,LABEL,UUID,PARTUUID,PARTTYPE,PARTTYPENAME,MOUNTPOINT,MOUNTPOINTS,MOUNTOPTIONS,PKNAME,PTTYPE",
        "--paths",
    ];

    debug!(command = "lsblk", args = ?args, "Executando lsblk");

    let mut cmd = Command::new("lsblk");
    cmd.args(args);

    // Adiciona variáveis de ambiente customizadas
    for (key, value) in &config.env_vars {
        cmd.env(key, value);
    }

    // Força locale C para saída consistente
    cmd.env("LC_ALL", "C");
    cmd.env("LANG", "C");

    let output = execute_with_timeout(&mut cmd, config)?;

    debug!(
        exit_code = output.exit_code,
        stdout_len = output.stdout.len(),
        stderr_len = output.stderr.len(),
        timed_out = output.timed_out,
        "lsblk concluído"
    );

    Ok(output)
}

/// Executa comando com timeout e limite de saída.
///
/// Esta implementação resolve dois problemas:
/// 1. O thread de espera não mantém o mutex durante `wait()`, permitindo que o thread principal
///    mate o processo no timeout.
/// 2. Os pipes de stdout/stderr são drenados completamente mesmo após atingir o limite de saída,
///    evitando que o processo filho bloqueie em escrita.
fn execute_with_timeout(cmd: &mut Command, config: &CommandConfig) -> Result<CommandOutput> {
    use std::io::Read;
    use std::process::Stdio;
    use std::sync::{Arc, Mutex, mpsc};
    use std::thread;

    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::internal_with_source("Falha ao spawnar lsblk", e))?;

    let timeout = Duration::from_secs(config.timeout_secs);
    let max_output = config.max_output_bytes;

    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();

    // Compartilha o child entre threads usando Arc<Mutex>
    // O child é removido do mutex ANTES de esperar, para não bloquear o kill no timeout
    let child = Arc::new(Mutex::new(Some(child)));

    // Thread para ler stdout com limite e drenagem completa
    let stdout_thread = if let Some(stdout_handle) = stdout_handle {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let mut limited = stdout_handle.take(max_output as u64);
            // Lê até o limite
            limited.read_to_end(&mut buf).ok();
            // Drena o restante do pipe para evitar bloqueio do processo filho
            // into_inner() retorna o reader interno diretamente
            let mut inner = limited.into_inner();
            let _ = inner.read_to_end(&mut Vec::new());
            String::from_utf8_lossy(&buf).to_string()
        })
    } else {
        thread::spawn(String::new)
    };

    // Thread para ler stderr com limite e drenagem completa
    let stderr_thread = if let Some(stderr_handle) = stderr_handle {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let mut limited = stderr_handle.take(max_output as u64);
            limited.read_to_end(&mut buf).ok();
            let mut inner = limited.into_inner();
            let _ = inner.read_to_end(&mut Vec::new());
            String::from_utf8_lossy(&buf).to_string()
        })
    } else {
        thread::spawn(String::new)
    };

    // Canal para comunicar resultado do wait
    let (tx, rx) = mpsc::channel();

    // Thread para aguardar o processo - REMOVE o child do mutex ANTES de esperar
    let child_clone = Arc::clone(&child);
    thread::spawn(move || {
        // Remove o child do mutex imediatamente, não segura o lock durante wait()
        let child_opt = {
            let mut guard = child_clone.lock().unwrap();
            guard.take()
        };
        if let Some(mut child) = child_opt {
            let result = child.wait();
            let _ = tx.send(result);
        }
    });

    // Aguarda com timeout
    let wait_result = match rx.recv_timeout(timeout) {
        Ok(Ok(status)) => {
            let exit_code = status.code().unwrap_or(-1);
            let stdout = stdout_thread.join().unwrap_or_default();
            let stderr = stderr_thread.join().unwrap_or_default();
            return Ok(CommandOutput {
                exit_code,
                stdout,
                stderr,
                timed_out: false,
            });
        }
        Ok(Err(e)) => {
            // Processo falhou
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            return Err(Error::internal_with_source("Erro ao aguardar lsblk", e));
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            // Timeout - mata o processo (mutex livre pois wait thread já liberou)
            if let Ok(mut child_guard) = child.lock()
                && let Some(mut child) = child_guard.take()
            {
                let _ = child.kill();
                let _ = child.wait();
            }
            true
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            return Err(Error::internal("Thread de espera do processo falhou"));
        }
    };

    // Se chegou aqui, foi timeout
    let timed_out = wait_result;
    let stdout = stdout_thread.join().unwrap_or_default();
    let stderr = stderr_thread.join().unwrap_or_default();

    Ok(CommandOutput {
        exit_code: if timed_out { -1 } else { 0 },
        stdout,
        stderr,
        timed_out,
    })
}

/// Parseia a saída JSON do lsblk em BlockDevices.
pub fn parse_lsblk_json(output: &CommandOutput) -> Result<Vec<BlockDevice>> {
    if output.timed_out {
        return Err(Error::detection(
            "lsblk excedeu timeout".to_string(),
            Some("lsblk".to_string()),
        ));
    }

    if !output.stderr.is_empty() && output.exit_code != 0 {
        warn!(stderr = %output.stderr, "lsblk produziu erro");
    }

    if output.stdout.trim().is_empty() {
        return Err(Error::detection(
            "lsblk retornou saída vazia".to_string(),
            Some("lsblk".to_string()),
        ));
    }

    let lsblk_json: LsblkJson = serde_json::from_str(&output.stdout)
        .map_err(|e| Error::internal_with_source("Falha ao parsear JSON do lsblk", e))?;

    let mut devices = Vec::new();
    for device in lsblk_json.blockdevices {
        devices.push(parse_lsblk_device_with_parent_removable(device, None)?);
    }

    Ok(devices)
}

/// Parseia um dispositivo lsblk recursivamente, propagando flag de removível do pai.
fn parse_lsblk_device_with_parent_removable(
    device: LsblkDevice,
    parent_removable: Option<bool>,
) -> Result<BlockDevice> {
    let path = device
        .path
        .clone()
        .or_else(|| Some(format!("/dev/{}", device.name)))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| Error::internal("Dispositivo sem caminho"))?;

    let name = device.name.clone();

    let device_id = device.maj_min.as_ref().and_then(|s| parse_maj_min(s));

    let device_type = DeviceType::from_string(&device.device_type);

    let size = device.size.as_ref().and_then(parse_u64);

    let sector_size = device.sectorsize.as_ref().and_then(parse_u32);

    let logical_sector_size = device.logicalsectorsize.as_ref().and_then(parse_u32);

    let model = device.model.filter(|s| !s.is_empty());
    let serial = device.serial.filter(|s| !s.is_empty());
    let vendor = device.vendor.filter(|s| !s.is_empty());

    let rotation_rate = device.rota.as_ref().and_then(parse_u32);

    // rm e ro podem ser null no JSON do lsblk, propagamos None para indicar "desconhecido"
    let removable = device.rm.or(parent_removable);
    let read_only = device.ro;

    let partition_table = device
        .pttype
        .as_ref()
        .map(|s| PartitionTableType::from_string(s));

    let uuid = device.uuid.filter(|s| !s.is_empty());
    let partuuid = device.partuuid.filter(|s| !s.is_empty());
    let label = device.label.filter(|s| !s.is_empty());

    let partition_type = device
        .parttype
        .as_ref()
        .map(|s| PartitionType::from_string(s))
        .or_else(|| {
            device
                .parttypename
                .as_ref()
                .map(|s| PartitionType::from_string(s))
        });

    let filesystem = device
        .fstype
        .as_ref()
        .map(|s| FilesystemType::from_string(s));

    let mountpoint = device
        .mountpoint
        .as_ref()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from);

    let mount_options = device.mountoptions.unwrap_or_default();

    // Parse mountpoints (lsblk MOUNTPOINTS field)
    let mountpoints = device
        .mountpoints
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect();

    let parent = device.pkname.filter(|s| !s.is_empty());

    // Parse children, collecting errors but not discarding them silently
    let mut children = Vec::new();
    let mut child_errors = Vec::new();
    for child in device.children.unwrap_or_default() {
        match parse_lsblk_device_with_parent_removable(child, removable) {
            Ok(parsed) => children.push(parsed),
            Err(e) => {
                child_errors.push(e.to_string());
            }
        }
    }

    // Log child parsing errors as diagnostics (but don't fail the entire parse)
    if !child_errors.is_empty() {
        tracing::warn!(
            parent_device = %name,
            errors = ?child_errors,
            "Erros ao parsear dispositivos filhos"
        );
    }

    Ok(BlockDevice {
        path,
        name,
        device_id,
        device_type,
        size,
        sector_size,
        logical_sector_size,
        model,
        serial,
        vendor,
        rotation_rate,
        removable,
        read_only,
        partition_table,
        uuid,
        partuuid,
        label,
        partition_type,
        filesystem,
        mountpoint,
        mountpoints,
        mount_options,
        parent,
        sysfs_path: None, // Preenchido separadamente se necessário
        children,
    })
}

/// Parseia string "major:minor" em DeviceId.
fn parse_maj_min(s: &str) -> Option<DeviceId> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 2 {
        let major = parts[0].parse::<u32>().ok()?;
        let minor = parts[1].parse::<u32>().ok()?;
        Some(DeviceId::from_major_minor(major, minor))
    } else {
        None
    }
}

fn parse_u64(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

fn parse_u32(value: &serde_json::Value) -> Option<u32> {
    u32::try_from(parse_u64(value)?).ok()
}

/// Obtém a versão do lsblk.
pub fn get_lsblk_version() -> Result<Option<String>> {
    let output = Command::new("lsblk")
        .arg("--version")
        .output()
        .map_err(|e| Error::internal_with_source("Falha ao executar lsblk --version", e))?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() {
            return Ok(Some(version));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_maj_min() {
        assert_eq!(parse_maj_min("8:0"), Some(DeviceId::from_major_minor(8, 0)));
        assert_eq!(
            parse_maj_min("259:0"),
            Some(DeviceId::from_major_minor(259, 0))
        );
        assert_eq!(parse_maj_min("invalid"), None);
        assert_eq!(parse_maj_min("8"), None);
    }

    #[test]
    fn test_device_id_display() {
        let id = DeviceId::from_major_minor(8, 0);
        assert_eq!(id.to_string(), "8:0");
    }

    #[test]
    fn test_filesystem_type_from_string() {
        assert_eq!(FilesystemType::from_string("ext4"), FilesystemType::Ext4);
        assert_eq!(FilesystemType::from_string("btrfs"), FilesystemType::Btrfs);
        assert_eq!(FilesystemType::from_string("vfat"), FilesystemType::Vfat);
        assert_eq!(FilesystemType::from_string("ntfs"), FilesystemType::Ntfs);
        assert_eq!(
            FilesystemType::from_string("unknown"),
            FilesystemType::Unknown
        );
    }

    #[test]
    fn test_partition_type_from_string() {
        assert_eq!(
            PartitionType::from_string("c12a7328-f81f-11d2-ba4b-00a0c93ec93b"),
            PartitionType::EfiSystem
        );
        assert_eq!(
            PartitionType::from_string("0fc63daf-8483-4772-8e79-3d69d8477de4"),
            PartitionType::LinuxFilesystem
        );
        assert_eq!(PartitionType::from_string("0x83"), PartitionType::Mbr(0x83));
    }

    #[test]
    fn test_device_type_from_string() {
        assert_eq!(DeviceType::from_string("disk"), DeviceType::Disk);
        assert_eq!(DeviceType::from_string("part"), DeviceType::Partition);
        assert_eq!(DeviceType::from_string("lvm"), DeviceType::Lvm);
    }

    #[test]
    fn test_partition_table_type_from_string() {
        assert_eq!(
            PartitionTableType::from_string("gpt"),
            PartitionTableType::Gpt
        );
        assert_eq!(
            PartitionTableType::from_string("dos"),
            PartitionTableType::Mbr
        );
    }

    #[test]
    fn test_execute_with_timeout_normal_completion() {
        use crate::discovery::types::CommandConfig;
        use std::process::Command;

        let config = CommandConfig {
            timeout_secs: 5,
            max_output_bytes: 1024,
            env_vars: Default::default(),
        };

        // Comando que termina rapidamente
        let mut cmd = Command::new("echo");
        cmd.arg("hello");

        let output = execute_with_timeout(&mut cmd, &config).unwrap();
        assert!(!output.timed_out);
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("hello"));
    }

    #[test]
    fn test_execute_with_timeout_kills_on_timeout() {
        use crate::discovery::types::CommandConfig;
        use std::process::Command;

        let config = CommandConfig {
            timeout_secs: 1,
            max_output_bytes: 1024,
            env_vars: Default::default(),
        };

        // Comando que dorme por mais tempo que o timeout
        let mut cmd = Command::new("sleep");
        cmd.arg("10");

        let output = execute_with_timeout(&mut cmd, &config).unwrap();
        assert!(output.timed_out);
        assert_eq!(output.exit_code, -1);
    }

    #[test]
    fn test_execute_with_timeout_output_limit() {
        use crate::discovery::types::CommandConfig;
        use std::process::Command;

        let config = CommandConfig {
            timeout_secs: 5,
            max_output_bytes: 50, // Limite pequeno
            env_vars: Default::default(),
        };

        // Comando que produz mais saída que o limite
        let mut cmd = Command::new("printf");
        cmd.arg("A").arg("%0.s").arg("{1..1000}"); // Produz ~1000 bytes

        let output = execute_with_timeout(&mut cmd, &config).unwrap();
        assert!(!output.timed_out);
        // A saída deve ser truncada no limite
        assert!(output.stdout.len() <= config.max_output_bytes);
        // O processo deve ter terminado (não bloqueado na escrita)
    }

    #[test]
    fn test_execute_with_timeout_stderr_limit() {
        use crate::discovery::types::CommandConfig;
        use std::process::Command;

        let config = CommandConfig {
            timeout_secs: 5,
            max_output_bytes: 50,
            env_vars: Default::default(),
        };

        // Comando que escreve muito no stderr
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg("for i in {1..1000}; do echo 'error line' >&2; done");

        let output = execute_with_timeout(&mut cmd, &config).unwrap();
        assert!(!output.timed_out);
        // stderr deve ser truncado no limite
        assert!(output.stderr.len() <= config.max_output_bytes);
    }

    #[test]
    fn test_parse_removable_and_read_only_none_when_missing() {
        // Testa que rm e ro ausentes no JSON resultam em None (desconhecido)
        let json = r#"{
            "blockdevices": [{
                "name": "sda",
                "path": "/dev/sda",
                "maj:min": "8:0",
                "type": "disk",
                "size": "500107862016",
                "model": "Test Disk",
                "rota": "0"
            }]
        }"#;

        let output = CommandOutput {
            exit_code: 0,
            stdout: json.to_string(),
            stderr: String::new(),
            timed_out: false,
        };

        let devices = parse_lsblk_json(&output).unwrap();
        assert_eq!(devices[0].removable, None);
        assert_eq!(devices[0].read_only, None);
    }

    #[test]
    fn test_parse_removable_true_false() {
        // Testa rm=true
        let json_true = r#"{
            "blockdevices": [{
                "name": "sdb",
                "path": "/dev/sdb",
                "maj:min": "8:16",
                "type": "disk",
                "size": "1000204886016",
                "rm": true,
                "ro": false
            }]
        }"#;

        let output = CommandOutput {
            exit_code: 0,
            stdout: json_true.to_string(),
            stderr: String::new(),
            timed_out: false,
        };

        let devices = parse_lsblk_json(&output).unwrap();
        assert_eq!(devices[0].removable, Some(true));
        assert_eq!(devices[0].read_only, Some(false));
    }

    #[test]
    fn test_parse_removable_propagates_to_children() {
        // Testa que removable do pai é propagado para filhos quando filho não tem rm
        let json = r#"{
            "blockdevices": [{
                "name": "sda",
                "path": "/dev/sda",
                "maj:min": "8:0",
                "type": "disk",
                "size": "500107862016",
                "rm": true,
                "ro": false,
                "children": [{
                    "name": "sda1",
                    "path": "/dev/sda1",
                    "maj:min": "8:1",
                    "type": "part",
                    "size": "536870912"
                }]
            }]
        }"#;

        let output = CommandOutput {
            exit_code: 0,
            stdout: json.to_string(),
            stderr: String::new(),
            timed_out: false,
        };

        let devices = parse_lsblk_json(&output).unwrap();
        assert_eq!(devices[0].removable, Some(true));
        // Filho deve herdar removable do pai
        assert_eq!(devices[0].children[0].removable, Some(true));
    }

    #[test]
    fn test_parse_read_only_propagates() {
        // Testa que ro=false do pai não força read_only=false no filho se filho não tem ro
        // read_only NÃO é propagado (cada dispositivo tem seu próprio status)
        let json = r#"{
            "blockdevices": [{
                "name": "sda",
                "path": "/dev/sda",
                "maj:min": "8:0",
                "type": "disk",
                "size": "500107862016",
                "ro": true,
                "children": [{
                    "name": "sda1",
                    "path": "/dev/sda1",
                    "maj:min": "8:1",
                    "type": "part",
                    "size": "536870912"
                }]
            }]
        }"#;

        let output = CommandOutput {
            exit_code: 0,
            stdout: json.to_string(),
            stderr: String::new(),
            timed_out: false,
        };

        let devices = parse_lsblk_json(&output).unwrap();
        assert_eq!(devices[0].read_only, Some(true));
        // Filho NÃO herda read_only do pai (None = desconhecido)
        assert_eq!(devices[0].children[0].read_only, None);
    }
}
