//! Adaptador para consultar `blkid` para informações complementares.
//!
//! Usado somente para complementar dados que não estão disponíveis no lsblk.
//! Trata ausência da ferramenta como diagnóstico, não como panic.

use crate::discovery::types::{
    BlockDevice, CommandConfig, CommandOutput, FilesystemInfo, FilesystemType,
};
use crate::error::{Error, Result};
use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;
use tracing::{debug, warn};

/// Estrutura para saída JSON do blkid.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
struct BlkidJson {
    blockdevices: Vec<BlkidDevice>,
}

/// Dispositivo individual no JSON do blkid.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
struct BlkidDevice {
    name: String,
    #[serde(rename = "type")]
    fstype: Option<String>,
    uuid: Option<String>,
    label: Option<String>,
    partuuid: Option<String>,
    #[allow(dead_code)]
    partlabel: Option<String>,
    #[serde(rename = "subsystems")]
    subsystems: Option<Vec<BlkidSubsystem>>,
}

/// Subsistema (ex: LUKS).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
struct BlkidSubsystem {
    uuid: Option<String>,
    #[serde(rename = "type")]
    fstype: Option<String>,
}

/// Executa `blkid` com saída JSON.
pub fn run_blkid(config: &CommandConfig) -> Result<CommandOutput> {
    let args = ["-o", "json", "-p"];

    debug!(command = "blkid", args = ?args, "Executando blkid");

    let mut cmd = Command::new("blkid");
    cmd.args(args);

    for (key, value) in &config.env_vars {
        cmd.env(key, value);
    }

    cmd.env("LC_ALL", "C");
    cmd.env("LANG", "C");

    let output = execute_with_timeout(&mut cmd, config)?;

    debug!(
        exit_code = output.exit_code,
        stdout_len = output.stdout.len(),
        stderr_len = output.stderr.len(),
        timed_out = output.timed_out,
        "blkid concluído"
    );

    Ok(output)
}

/// Executa `blkid` para um dispositivo específico.
pub fn run_blkid_device(device_path: &str, config: &CommandConfig) -> Result<CommandOutput> {
    let args = ["-o", "json", "-p", device_path];

    debug!(
        command = "blkid",
        device = device_path,
        "Executando blkid para dispositivo"
    );

    let mut cmd = Command::new("blkid");
    cmd.args(args);

    for (key, value) in &config.env_vars {
        cmd.env(key, value);
    }

    cmd.env("LC_ALL", "C");
    cmd.env("LANG", "C");

    let output = execute_with_timeout(&mut cmd, config)?;

    debug!(
        exit_code = output.exit_code,
        stdout_len = output.stdout.len(),
        stderr_len = output.stderr.len(),
        timed_out = output.timed_out,
        "blkid dispositivo concluído"
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
        .map_err(|e| Error::internal_with_source("Falha ao spawnar blkid", e))?;

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
            return Err(Error::internal_with_source("Erro ao aguardar blkid", e));
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

/// Parseia a saída JSON do blkid em mapa de informações de filesystem.
pub fn parse_blkid_json(output: &CommandOutput) -> Result<HashMap<String, FilesystemInfo>> {
    if output.timed_out {
        return Err(Error::detection(
            "blkid excedeu timeout".to_string(),
            Some("blkid".to_string()),
        ));
    }

    if !output.stderr.is_empty() && output.exit_code != 0 {
        warn!(stderr = %output.stderr, "blkid produziu erro");
    }

    if output.stdout.trim().is_empty() {
        return Ok(HashMap::new());
    }

    let blkid_json: BlkidJson = serde_json::from_str(&output.stdout)
        .map_err(|e| Error::internal_with_source("Falha ao parsear JSON do blkid", e))?;

    let mut result = HashMap::new();
    for device in blkid_json.blockdevices {
        let name = device.name.clone();
        let info = parse_blkid_device(device);
        result.insert(name, info);
    }

    Ok(result)
}

/// Parseia um dispositivo blkid.
fn parse_blkid_device(device: BlkidDevice) -> FilesystemInfo {
    let fstype = device
        .fstype
        .as_ref()
        .map(|s| FilesystemType::from_string(s))
        .unwrap_or(FilesystemType::Unknown);

    let mut subsystems_uuid = None;
    let mut subsystems_type = None;

    if let Some(subsystems) = device.subsystems {
        for sub in subsystems {
            if let (Some(uuid), Some(fstype)) = (sub.uuid, sub.fstype) {
                subsystems_uuid = Some(uuid);
                subsystems_type = Some(fstype);
                break;
            }
        }
    }

    FilesystemInfo {
        fstype,
        uuid: device.uuid.filter(|s| !s.is_empty()),
        label: device.label.filter(|s| !s.is_empty()),
        partuuid: device.partuuid.filter(|s| !s.is_empty()),
        version: None, // blkid -p não retorna versão no JSON
        subsystems_uuid,
        subsystems_type,
    }
}

/// Enriquece BlockDevices com informações do blkid.
pub fn enrich_with_blkid(
    devices: &mut [BlockDevice],
    blkid_data: &HashMap<String, FilesystemInfo>,
) {
    for device in devices.iter_mut() {
        enrich_device(device, blkid_data);
    }
}

/// Enriquece um único dispositivo recursivamente.
fn enrich_device(device: &mut BlockDevice, blkid_data: &HashMap<String, FilesystemInfo>) {
    // Tenta encontrar pelo caminho do dispositivo
    let key = device.path.to_string_lossy().to_string();

    if let Some(info) = blkid_data.get(&key) {
        apply_blkid_info(device, info);
    } else if let Some(info) = blkid_data.get(&device.name) {
        apply_blkid_info(device, info);
    }

    // Enriquece filhos recursivamente
    for child in &mut device.children {
        enrich_device(child, blkid_data);
    }
}

/// Aplica informações do blkid ao dispositivo.
fn apply_blkid_info(device: &mut BlockDevice, info: &FilesystemInfo) {
    // Só preenche se não tiver já do lsblk
    if device.uuid.is_none() {
        device.uuid = info.uuid.clone();
    }
    if device.label.is_none() {
        device.label = info.label.clone();
    }
    if device.partuuid.is_none() {
        device.partuuid = info.partuuid.clone();
    }
    if device.filesystem.is_none() || device.filesystem == Some(FilesystemType::Unknown) {
        device.filesystem = Some(info.fstype);
    }
}

/// Verifica se blkid está disponível.
pub fn is_blkid_available() -> bool {
    Command::new("blkid")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Obtém a versão do blkid.
pub fn get_blkid_version() -> Result<Option<String>> {
    let output = Command::new("blkid")
        .arg("--version")
        .output()
        .map_err(|e| Error::internal_with_source("Falha ao executar blkid --version", e))?;

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
    use std::collections::HashMap;

    #[test]
    fn test_parse_blkid_json_empty() {
        let output = CommandOutput {
            exit_code: 0,
            stdout: r#"{"blockdevices":[]}"#.to_string(),
            stderr: String::new(),
            timed_out: false,
        };
        let result = parse_blkid_json(&output).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_blkid_json_single() {
        let output = CommandOutput {
            exit_code: 0,
            stdout: r#"{"blockdevices":[{"name":"/dev/sda1","type":"vfat","uuid":"ABCD-1234","label":"EFI","partuuid":"12345678-1234-1234-1234-123456789abc","partlabel":"EFI System","subsystems":[]}]}"#.to_string(),
            stderr: String::new(),
            timed_out: false,
        };
        let result = parse_blkid_json(&output).unwrap();
        assert_eq!(result.len(), 1);
        let info = result.get("/dev/sda1").unwrap();
        assert_eq!(info.fstype, FilesystemType::Vfat);
        assert_eq!(info.uuid, Some("ABCD-1234".to_string()));
        assert_eq!(info.label, Some("EFI".to_string()));
    }

    #[test]
    fn test_parse_blkid_json_with_luks() {
        let output = CommandOutput {
            exit_code: 0,
            stdout: r#"{"blockdevices":[{"name":"/dev/sda2","type":"crypto_LUKS","uuid":"luks-uuid","label":null,"partuuid":"part-uuid","partlabel":null,"subsystems":[{"uuid":"inner-uuid","type":"ext4"}]}]}"#.to_string(),
            stderr: String::new(),
            timed_out: false,
        };
        let result = parse_blkid_json(&output).unwrap();
        let info = result.get("/dev/sda2").unwrap();
        assert_eq!(info.fstype, FilesystemType::Luks);
        assert_eq!(info.subsystems_uuid, Some("inner-uuid".to_string()));
        assert_eq!(info.subsystems_type, Some("ext4".to_string()));
    }

    #[test]
    fn test_enrich_with_blkid() {
        let mut devices = vec![BlockDevice {
            path: "/dev/sda1".into(),
            name: "sda1".to_string(),
            device_id: Some(crate::discovery::types::DeviceId::from_major_minor(0, 0)),
            device_type: crate::discovery::types::DeviceType::Partition,
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
            uuid: None,
            partuuid: None,
            label: None,
            partition_type: None,
            filesystem: None,
            mountpoint: None,
            mountpoints: vec![],
            mount_options: vec![],
            parent: Some("sda".to_string()),
            sysfs_path: None,
            children: vec![],
        }];

        let mut blkid_data = HashMap::new();
        blkid_data.insert(
            "/dev/sda1".to_string(),
            FilesystemInfo {
                fstype: FilesystemType::Vfat,
                uuid: Some("ABCD-1234".to_string()),
                label: Some("EFI".to_string()),
                partuuid: Some("part-uuid".to_string()),
                version: None,
                subsystems_uuid: None,
                subsystems_type: None,
            },
        );

        enrich_with_blkid(&mut devices, &blkid_data);

        assert_eq!(devices[0].uuid, Some("ABCD-1234".to_string()));
        assert_eq!(devices[0].label, Some("EFI".to_string()));
        assert_eq!(devices[0].filesystem, Some(FilesystemType::Vfat));
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
            max_output_bytes: 50,
            env_vars: Default::default(),
        };

        let mut cmd = Command::new("printf");
        cmd.arg("A").arg("%0.s").arg("{1..1000}");

        let output = execute_with_timeout(&mut cmd, &config).unwrap();
        assert!(!output.timed_out);
        assert!(output.stdout.len() <= config.max_output_bytes);
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

        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg("for i in {1..1000}; do echo 'error line' >&2; done");

        let output = execute_with_timeout(&mut cmd, &config).unwrap();
        assert!(!output.timed_out);
        assert!(output.stderr.len() <= config.max_output_bytes);
    }
}
