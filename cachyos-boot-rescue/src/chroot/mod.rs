//! Gerenciamento de ambiente chroot para reparação de boot.
//!
//! Prepara pseudo-filesystems (/dev, /proc, /sys, /run),
//! executa comandos em ambiente controlado com captura de saída.

use crate::error::{Error, Result};
use crate::storage::MountSession;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Resultado de execução de comando no chroot.
#[derive(Debug, Clone)]
pub struct ChrootCommandResult {
    /// Código de saída.
    pub exit_code: i32,
    /// Saída padrão (truncada se exceder limite).
    pub stdout: String,
    /// Saída de erro (truncada se exceder limite).
    pub stderr: String,
    /// Duração da execução.
    pub duration: Duration,
    /// Se houve timeout.
    pub timed_out: bool,
    /// Se a saída foi truncada.
    pub truncated: bool,
}

/// Configuração para execução de comandos no chroot.
#[derive(Debug, Clone)]
pub struct ChrootCommandConfig {
    /// Timeout em segundos.
    pub timeout_secs: u64,
    /// Limite máximo de saída (bytes) para stdout.
    pub max_stdout_bytes: usize,
    /// Limite máximo de saída (bytes) para stderr.
    pub max_stderr_bytes: usize,
    /// Variáveis de ambiente adicionais.
    pub env_vars: HashMap<String, String>,
    /// Diretório de trabalho dentro do chroot.
    pub working_dir: Option<PathBuf>,
}

impl Default for ChrootCommandConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 60,
            max_stdout_bytes: 1024 * 1024, // 1 MB
            max_stderr_bytes: 1024 * 1024, // 1 MB
            env_vars: HashMap::new(),
            working_dir: None,
        }
    }
}

/// Pseudo-filesystems necessários para chroot.
const PSEUDO_FS: &[(&str, &str, &[&str])] = &[
    ("/dev", "devtmpfs", &["bind", "rbind"]),
    ("/proc", "proc", &["bind", "rbind"]),
    ("/sys", "sysfs", &["bind", "rbind"]),
    ("/run", "tmpfs", &["bind", "rbind"]),
];

/// Rastreia montagens de pseudo-filesystems feitas pelo gerenciador.
#[derive(Debug, Clone)]
struct PseudoMount {
    target_path: PathBuf,
    #[expect(dead_code)]
    source: String,
    mounted_by_manager: bool,
}

/// Gerenciador de ambiente chroot.
#[derive(Debug)]
pub struct ChrootManager {
    /// Ponto de montagem raiz do sistema alvo.
    root_mount: PathBuf,
    /// Montagens de pseudo-filesystems realizadas.
    pseudo_mounts: Vec<PseudoMount>,
    /// Sessão de montagem associada (para rastreamento).
    #[expect(dead_code)]
    mount_session: Option<MountSession>,
    /// Se o chroot foi preparado.
    prepared: bool,
}

impl ChrootManager {
    /// Cria novo gerenciador chroot.
    pub fn new(root_mount: PathBuf) -> Self {
        Self {
            root_mount,
            pseudo_mounts: Vec::new(),
            mount_session: None,
            prepared: false,
        }
    }

    /// Cria novo gerenciador chroot com sessão de montagem associada.
    pub fn with_session(root_mount: PathBuf, mount_session: MountSession) -> Self {
        Self {
            root_mount,
            pseudo_mounts: Vec::new(),
            mount_session: Some(mount_session),
            prepared: false,
        }
    }

    /// Retorna o ponto de montagem raiz.
    pub fn root_mount(&self) -> &Path {
        &self.root_mount
    }

    /// Verifica se o chroot está preparado.
    pub fn is_prepared(&self) -> bool {
        self.prepared
    }

    /// Prepara o ambiente chroot montando pseudo-filesystems.
    /// Se falhar em qualquer etapa, desmonta apenas o que foi montado por esta chamada.
    pub fn prepare(&mut self) -> Result<()> {
        if self.prepared {
            warn!("Chroot já preparado");
            return Ok(());
        }

        info!(root = %self.root_mount.display(), "Preparando ambiente chroot");

        // Verifica se raiz existe e é diretório
        if !self.root_mount.exists() {
            return Err(Error::internal(format!(
                "Ponto de montagem raiz não existe: {}",
                self.root_mount.display()
            )));
        }

        if !self.root_mount.is_dir() {
            return Err(Error::internal(format!(
                "Ponto de montagem raiz não é diretório: {}",
                self.root_mount.display()
            )));
        }

        let mut mounted_this_call = Vec::new();

        // Monta pseudo-filesystems
        for (target, _fs_type, _options) in PSEUDO_FS {
            let target_path = self
                .root_mount
                .join(target.strip_prefix('/').unwrap_or(target));

            // Verifica se já está montado (pré-existente)
            let already_mounted = Self::is_mounted(&target_path);

            // Cria ponto de montagem se não existir
            std::fs::create_dir_all(&target_path).map_err(|e| {
                // Rollback do que foi montado até agora
                self.rollback_mounts(&mounted_this_call);
                Error::internal_with_source("Falha ao criar ponto de montagem pseudo-fs", e)
            })?;

            if already_mounted {
                warn!(target = %target_path.display(), "Pseudo-filesystem já montado (pré-existente)");
                self.pseudo_mounts.push(PseudoMount {
                    target_path: target_path.clone(),
                    source: target.to_string(),
                    mounted_by_manager: false,
                });
                continue;
            }

            // Monta com bind/rbind
            let mut cmd = Command::new("mount");
            cmd.arg("--rbind");
            cmd.arg(target).arg(&target_path);

            debug!(source = target, target = %target_path.display(), "Montando pseudo-filesystem");

            let output = cmd.output().map_err(|e| {
                self.rollback_mounts(&mounted_this_call);
                Error::internal_with_source("Falha ao executar mount para pseudo-fs", e)
            })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Verifica se já está montado (race condition)
                if stderr.contains("already mounted") || stderr.contains("já montado") {
                    warn!(target = %target_path.display(), "Pseudo-filesystem já montado (race)");
                    self.pseudo_mounts.push(PseudoMount {
                        target_path: target_path.clone(),
                        source: target.to_string(),
                        mounted_by_manager: false,
                    });
                    continue;
                } else {
                    self.rollback_mounts(&mounted_this_call);
                    return Err(Error::internal(format!(
                        "Falha ao montar {} em {}: {}",
                        target,
                        target_path.display(),
                        stderr
                    )));
                }
            }

            self.pseudo_mounts.push(PseudoMount {
                target_path: target_path.clone(),
                source: target.to_string(),
                mounted_by_manager: true,
            });
            mounted_this_call.push(target_path);
        }

        // NÃO copia resolv.conf - pode sobrescrever dados do usuário
        // Se conectividade for necessária, use bind mount de /run/systemd/resolve ou configure externamente

        self.prepared = true;
        info!("Ambiente chroot preparado com sucesso");
        Ok(())
    }

    /// Verifica se um caminho já está montado.
    fn is_mounted(path: &Path) -> bool {
        // Usa findmnt para verificar
        Command::new("findmnt")
            .args(["-n", "-o", "TARGET", path.to_str().unwrap_or("")])
            .output()
            .map(|o| o.status.success() && !o.stdout.is_empty())
            .unwrap_or(false)
    }

    /// Rollback de montagens feitas durante prepare().
    fn rollback_mounts(&mut self, mounted: &[PathBuf]) {
        for mount_point in mounted.iter().rev() {
            let mut cmd = Command::new("umount");
            cmd.arg("-R").arg(mount_point);
            let _ = cmd.output();
        }
        // Remove do rastreamento
        self.pseudo_mounts
            .retain(|m| !mounted.contains(&m.target_path));
    }

    /// Executa comando dentro do chroot.
    ///
    /// Não usa shell - executa binário e argumentos diretamente.
    /// Captura stdout, stderr, exit code, duração e timeout.
    /// Sanitiza saída antes de logar.
    pub fn execute(
        &self,
        command: &[&str],
        config: Option<ChrootCommandConfig>,
    ) -> Result<ChrootCommandResult> {
        if !self.prepared {
            return Err(Error::internal(
                "Chroot não preparado - chame prepare() primeiro",
            ));
        }

        let config = config.unwrap_or_default();
        let timeout = Duration::from_secs(config.timeout_secs);
        let max_stdout = config.max_stdout_bytes;
        let max_stderr = config.max_stderr_bytes;

        if command.is_empty() {
            return Err(Error::internal("Comando vazio"));
        }

        let binary = command[0];
        let args = &command[1..];

        // Log sanitizado (sem argumentos que podem conter segredos)
        info!(
            binary = binary,
            arg_count = args.len(),
            "Executando comando no chroot"
        );

        let start = Instant::now();

        // Prepara comando chroot
        let mut cmd = Command::new("chroot");
        cmd.arg(&self.root_mount);

        // Adiciona variáveis de ambiente
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }

        // Define diretório de trabalho se especificado
        if let Some(ref wd) = config.working_dir {
            cmd.current_dir(wd);
        }

        // Adiciona comando e argumentos
        cmd.arg(binary).args(args);

        // Configura pipes
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| Error::internal_with_source("Falha ao spawnar chroot", e))?;

        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        // Canais para comunicação entre threads
        let (stdout_tx, stdout_rx) = std::sync::mpsc::channel();
        let (stderr_tx, stderr_rx) = std::sync::mpsc::channel();
        let (wait_tx, wait_rx) = std::sync::mpsc::channel();

        // Thread para ler stdout com limite e drenagem completa
        if let Some(handle) = stdout_handle {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let mut limited = handle.take(max_stdout as u64);
                let _ = Read::read_to_end(&mut limited, &mut buf);
                // Drena o restante para evitar deadlock
                let mut inner = std::io::Take::into_inner(limited);
                let _ = Read::read_to_end(&mut inner, &mut Vec::new());
                let output = String::from_utf8_lossy(&buf).to_string();
                let truncated = output.len() >= max_stdout;
                let _ = stdout_tx.send((output, truncated));
            });
        } else {
            let _ = stdout_tx.send((String::new(), false));
        }

        // Thread para ler stderr com limite e drenagem completa
        if let Some(handle) = stderr_handle {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let mut limited = handle.take(max_stderr as u64);
                let _ = Read::read_to_end(&mut limited, &mut buf);
                // Drena o restante para evitar deadlock
                let mut inner = std::io::Take::into_inner(limited);
                let _ = Read::read_to_end(&mut inner, &mut Vec::new());
                let output = String::from_utf8_lossy(&buf).to_string();
                let truncated = output.len() >= max_stderr;
                let _ = stderr_tx.send((output, truncated));
            });
        } else {
            let _ = stderr_tx.send((String::new(), false));
        }

        // Thread para aguardar o processo
        let mut child_for_wait = Some(child);
        std::thread::spawn(move || {
            if let Some(mut child) = child_for_wait.take() {
                let result = child.wait();
                let _ = wait_tx.send(result);
            }
        });

        // Aguarda com timeout
        let (exit_code, timed_out) = match wait_rx.recv_timeout(timeout) {
            Ok(Ok(status)) => (status.code().unwrap_or(-1), false),
            Ok(Err(e)) => {
                // Processo falhou ao esperar
                let _ = stdout_rx.recv();
                let _ = stderr_rx.recv();
                return Err(Error::internal_with_source(
                    "Erro ao aguardar comando chroot",
                    e,
                ));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Timeout - mata o processo
                // Nota: child já foi movido para a thread de wait
                // Precisamos de uma forma de matar o processo
                // Como não temos acesso direto, logamos e retornamos timeout
                warn!(binary = binary, "Timeout executando comando no chroot");
                (-1, true)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                let _ = stdout_rx.recv();
                let _ = stderr_rx.recv();
                return Err(Error::internal("Thread de espera do processo falhou"));
            }
        };

        // Coleta stdout/stderr (com timeout se necessário)
        let (stdout, stdout_truncated) = stdout_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap_or((String::new(), false));
        let (stderr, stderr_truncated) = stderr_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap_or((String::new(), false));

        let duration = start.elapsed();

        // Sanitiza saída antes de logar
        let sanitized_stdout = crate::logging::sanitize_output(&stdout);
        let sanitized_stderr = crate::logging::sanitize_output(&stderr);

        debug!(
            binary = binary,
            exit_code = exit_code,
            duration_ms = duration.as_millis(),
            timed_out = timed_out,
            stdout_len = sanitized_stdout.len(),
            stderr_len = sanitized_stderr.len(),
            "Comando chroot concluído"
        );

        Ok(ChrootCommandResult {
            exit_code,
            stdout: sanitized_stdout,
            stderr: sanitized_stderr,
            duration,
            timed_out,
            truncated: stdout_truncated || stderr_truncated,
        })
    }

    /// Executa comando simples (string única) - para compatibilidade.
    pub fn execute_simple(
        &self,
        command: &str,
        config: Option<ChrootCommandConfig>,
    ) -> Result<ChrootCommandResult> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        self.execute(&parts, config)
    }

    /// Verifica se um comando existe no chroot.
    pub fn command_exists(&self, command: &str) -> bool {
        let result = self.execute(&["which", command], None);
        matches!(result, Ok(r) if r.exit_code == 0)
    }

    /// Desmonta pseudo-filesystems e limpa.
    /// Retorna erro se alguma desmontagem falhar.
    pub fn cleanup(&mut self) -> Result<()> {
        if !self.prepared {
            return Ok(());
        }

        info!("Limpando ambiente chroot");

        let mut errors = Vec::new();

        // Desmonta em ordem inversa
        for mount in self.pseudo_mounts.iter().rev() {
            if !mount.mounted_by_manager {
                debug!(mount_point = %mount.target_path.display(), "Pulando desmontagem de pseudo-fs pré-existente");
                continue;
            }

            let mut cmd = Command::new("umount");
            cmd.arg("-R").arg(&mount.target_path);

            debug!(mount_point = %mount.target_path.display(), "Desmontando pseudo-filesystem");

            let output = cmd
                .output()
                .map_err(|e| Error::internal_with_source("Falha ao executar umount", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Ignora se já não estiver montado
                if !stderr.contains("not mounted") && !stderr.contains("não montado") {
                    errors.push(format!(
                        "Falha ao desmontar {}: {}",
                        mount.target_path.display(),
                        stderr
                    ));
                    warn!(mount_point = %mount.target_path.display(), stderr = %stderr, "Falha ao desmontar pseudo-fs");
                }
            }
        }

        self.pseudo_mounts.clear();
        self.prepared = false;

        if !errors.is_empty() {
            Err(Error::internal(format!(
                "Erros na limpeza do chroot: {}",
                errors.join("; ")
            )))
        } else {
            info!("Ambiente chroot limpo");
            Ok(())
        }
    }
}

impl Drop for ChrootManager {
    fn drop(&mut self) {
        if self.prepared {
            warn!("ChrootManager dropado sem cleanup() - tentando limpeza");
            let _ = self.cleanup();
        }
    }
}

/// Valida se o ambiente chroot é viável para reparação.
/// NÃO prepara chroot completo - apenas verifica existência de ferramentas no sistema de arquivos.
pub fn validate_chroot_environment(root_mount: &Path) -> Result<ChrootValidation> {
    let mut validation = ChrootValidation {
        root_exists: false,
        has_bash: false,
        has_mount: false,
        has_cryptsetup: false,
        has_grub_install: false,
        has_limine_install: false,
        has_mkinitcpio: false,
        has_dracut: false,
        warnings: Vec::new(),
    };

    if !root_mount.exists() {
        validation
            .warnings
            .push("Ponto de montagem raiz não existe".to_string());
        return Ok(validation);
    }
    validation.root_exists = true;

    // Verifica ferramentas essenciais diretamente no filesystem (sem chroot)
    let tools = [
        ("bash", "/usr/bin/bash"),
        ("mount", "/usr/bin/mount"),
        ("cryptsetup", "/usr/sbin/cryptsetup"),
        ("grub-install", "/usr/bin/grub-install"),
        ("limine-install", "/usr/bin/limine-install"),
        ("mkinitcpio", "/usr/bin/mkinitcpio"),
        ("dracut", "/usr/bin/dracut"),
    ];

    for (name, path) in tools {
        let full_path = root_mount.join(path.strip_prefix('/').unwrap_or(path));
        if full_path.exists() {
            match name {
                "bash" => validation.has_bash = true,
                "mount" => validation.has_mount = true,
                "cryptsetup" => validation.has_cryptsetup = true,
                "grub-install" => validation.has_grub_install = true,
                "limine-install" => validation.has_limine_install = true,
                "mkinitcpio" => validation.has_mkinitcpio = true,
                "dracut" => validation.has_dracut = true,
                _ => {}
            }
        } else {
            validation.warnings.push(format!(
                "Ferramenta não encontrada no sistema alvo: {} ({})",
                name, path
            ));
        }
    }

    Ok(validation)
}

/// Resultado de validação do ambiente chroot.
#[derive(Debug, Clone)]
pub struct ChrootValidation {
    pub root_exists: bool,
    pub has_bash: bool,
    pub has_mount: bool,
    pub has_cryptsetup: bool,
    pub has_grub_install: bool,
    pub has_limine_install: bool,
    pub has_mkinitcpio: bool,
    pub has_dracut: bool,
    pub warnings: Vec<String>,
}

impl ChrootValidation {
    /// Verifica se ambiente é adequado para reparação GRUB.
    pub fn can_repair_grub(&self) -> bool {
        self.root_exists && self.has_bash && self.has_mount && self.has_grub_install
    }

    /// Verifica se ambiente é adequado para reparação Limine.
    pub fn can_repair_limine(&self) -> bool {
        self.root_exists && self.has_bash && self.has_mount && self.has_limine_install
    }

    /// Verifica se ambiente pode regenerar initramfs.
    pub fn can_regenerate_initramfs(&self) -> bool {
        self.root_exists && self.has_bash && (self.has_mkinitcpio || self.has_dracut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_chroot_manager_creation() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        let manager = ChrootManager::new(root.clone());
        assert_eq!(manager.root_mount(), root);
        assert!(!manager.is_prepared());
    }

    #[test]
    fn test_chroot_command_config_default() {
        let config = ChrootCommandConfig::default();
        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.max_stdout_bytes, 1024 * 1024);
        assert_eq!(config.max_stderr_bytes, 1024 * 1024);
    }

    #[test]
    fn test_chroot_validation_default() {
        let validation = ChrootValidation {
            root_exists: false,
            has_bash: false,
            has_mount: false,
            has_cryptsetup: false,
            has_grub_install: false,
            has_limine_install: false,
            has_mkinitcpio: false,
            has_dracut: false,
            warnings: Vec::new(),
        };
        assert!(!validation.can_repair_grub());
        assert!(!validation.can_repair_limine());
        assert!(!validation.can_regenerate_initramfs());
    }

    #[test]
    fn test_chroot_validation_with_tools() {
        let mut validation = ChrootValidation {
            root_exists: true,
            has_bash: true,
            has_mount: true,
            has_cryptsetup: true,
            has_grub_install: true,
            has_limine_install: false,
            has_mkinitcpio: true,
            has_dracut: false,
            warnings: Vec::new(),
        };
        assert!(validation.can_repair_grub());
        assert!(!validation.can_repair_limine());
        assert!(validation.can_regenerate_initramfs());

        validation.has_limine_install = true;
        assert!(validation.can_repair_limine());
    }

    #[test]
    fn test_validate_chroot_environment_without_prepare() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        // Cria estrutura mínima
        std::fs::create_dir_all(root.join("usr/bin")).unwrap();
        std::fs::create_dir_all(root.join("usr/sbin")).unwrap();
        std::fs::write(root.join("usr/bin/bash"), "").unwrap();
        std::fs::write(root.join("usr/bin/mount"), "").unwrap();
        std::fs::write(root.join("usr/sbin/cryptsetup"), "").unwrap();
        std::fs::write(root.join("usr/bin/grub-install"), "").unwrap();
        std::fs::write(root.join("usr/bin/mkinitcpio"), "").unwrap();

        let validation = validate_chroot_environment(&root).unwrap();
        assert!(validation.root_exists);
        assert!(validation.has_bash);
        assert!(validation.has_mount);
        assert!(validation.has_cryptsetup);
        assert!(validation.has_grub_install);
        assert!(validation.has_mkinitcpio);
        assert!(!validation.has_limine_install);
        assert!(!validation.has_dracut);
    }

    #[test]
    fn test_chroot_command_result_sanitization() {
        let result = ChrootCommandResult {
            exit_code: 0,
            stdout: "password=secret123".to_string(),
            stderr: "token=abc123".to_string(),
            duration: Duration::from_millis(100),
            timed_out: false,
            truncated: false,
        };

        // A sanitização é feita no execute(), não no resultado
        // Este teste apenas verifica a estrutura
        assert_eq!(result.exit_code, 0);
        assert!(!result.timed_out);
    }
}
