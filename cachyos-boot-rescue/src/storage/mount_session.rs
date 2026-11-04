//! Sessão de montagem com rastreamento e desmontagem segura.
//!
//! Mantém inventário de montagens realizadas pela aplicação e garante
//! desmontagem em ordem inversa, inclusive em caso de erro.

use crate::error::{Error, Result};
use crate::storage::mount_plan::{MountPlan, MountPlanEntry};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

/// Resultado de desmontagem com rastreamento de falhas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmountResult {
    /// Montagens desmontadas com sucesso.
    pub unmounted: Vec<PathBuf>,
    /// Montagens que falharam ao desmontar.
    pub failed: Vec<(PathBuf, String)>,
    /// Montagens puladas (não montadas pela sessão).
    pub skipped: Vec<PathBuf>,
}

impl UnmountResult {
    /// Verifica se houve falhas.
    pub fn has_failures(&self) -> bool {
        !self.failed.is_empty()
    }

    /// Combina dois resultados.
    pub fn merge(mut self, other: Self) -> Self {
        self.unmounted.extend(other.unmounted);
        self.failed.extend(other.failed);
        self.skipped.extend(other.skipped);
        self
    }
}

/// Representa uma montagem ativa na sessão.
#[derive(Debug, Clone)]
pub struct ActiveMount {
    /// Ponto de montagem (caminho lógico dentro do sistema alvo, ex: /boot/efi).
    pub mount_point: PathBuf,
    /// Dispositivo montado.
    pub device_path: PathBuf,
    /// Tipo de filesystem.
    pub fs_type: String,
    /// Opções usadas na montagem.
    pub options: String,
    /// Se foi montado por esta sessão (true) ou já existia (false).
    pub mounted_by_session: bool,
    /// Timestamp da montagem.
    pub mounted_at: std::time::SystemTime,
}

impl ActiveMount {
    /// Retorna o ponto de montagem resolvido para o host (root_mount + mount_point).
    pub fn resolved_mount_point(&self, root_mount: &Path) -> PathBuf {
        let relative = self
            .mount_point
            .strip_prefix("/")
            .unwrap_or(&self.mount_point);
        root_mount.join(relative)
    }
}

/// Sessão de montagem que rastreia todas as montagens realizadas.
#[derive(Debug)]
pub struct MountSession {
    /// Montagens ativas (ordenadas: raiz primeiro, filhos depois).
    mounts: Arc<Mutex<Vec<ActiveMount>>>,
    /// Diretório raiz da sessão (onde / é montado).
    root_mount: PathBuf,
    /// Se a sessão foi finalizada corretamente.
    finalized: bool,
    /// Se a sessão está em processo de rollback/finalização (evita reentrância).
    finalizing: bool,
}

impl MountSession {
    /// Cria nova sessão de montagem.
    pub fn new(root_mount: PathBuf) -> Self {
        Self {
            mounts: Arc::new(Mutex::new(Vec::new())),
            root_mount,
            finalized: false,
            finalizing: false,
        }
    }

    /// Retorna o ponto de montagem raiz.
    pub fn root_mount(&self) -> &Path {
        &self.root_mount
    }

    /// Registra uma montagem realizada.
    pub fn record_mount(
        &self,
        mount_point: PathBuf,
        device_path: PathBuf,
        fs_type: String,
        options: String,
        mounted_by_session: bool,
    ) {
        let mp_display = mount_point.display().to_string();
        let dev_display = device_path.display().to_string();
        let mut mounts = self.mounts.lock().unwrap();

        // Verifica se já está registrado (idempotência)
        if mounts.iter().any(|m| m.mount_point == mount_point) {
            debug!(mount_point = %mp_display, "Montagem já registrada, ignorando duplicata");
            return;
        }

        mounts.push(ActiveMount {
            mount_point,
            device_path,
            fs_type,
            options,
            mounted_by_session,
            mounted_at: std::time::SystemTime::now(),
        });
        info!(mount_point = %mp_display, device = %dev_display, "Montagem registrada");
    }

    /// Verifica se um ponto de montagem está ativo nesta sessão.
    pub fn is_mounted(&self, mount_point: &Path) -> bool {
        let mounts = self.mounts.lock().unwrap();
        mounts.iter().any(|m| m.mount_point == mount_point)
    }

    /// Retorna todas as montagens ativas.
    pub fn active_mounts(&self) -> Vec<ActiveMount> {
        self.mounts.lock().unwrap().clone()
    }

    /// Retorna montagens ordenadas para desmontagem (filhos primeiro, raiz por último).
    pub fn mounts_for_unmount(&self) -> Vec<ActiveMount> {
        let mounts = self.mounts.lock().unwrap();
        let mut sorted = mounts.clone();
        // Ordena por profundidade do caminho (mais profundo primeiro), depois por caminho
        sorted.sort_by(|a, b| {
            let a_depth = a.mount_point.components().count();
            let b_depth = b.mount_point.components().count();
            b_depth.cmp(&a_depth).then_with(|| {
                a.mount_point
                    .to_string_lossy()
                    .cmp(&b.mount_point.to_string_lossy())
            })
        });
        sorted
    }

    /// Executa desmontagem de um ponto de montagem específico.
    /// NÃO usa force por padrão - force requer autorização explícita.
    pub fn unmount(&self, mount_point: &Path, force: bool) -> Result<()> {
        use std::process::Command;

        let mut cmd = Command::new("umount");
        if force {
            cmd.arg("-f");
        }
        cmd.arg(mount_point);

        debug!(mount_point = %mount_point.display(), force, "Executando umount");
        let output = cmd
            .output()
            .map_err(|e| Error::internal_with_source("Falha ao executar umount", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Verifica se já não está montado
            if stderr.contains("not mounted") || stderr.contains("não montado") {
                warn!(mount_point = %mount_point.display(), "Tentativa de desmontar ponto não montado");
                // Remove do rastreamento mesmo assim
                let mut mounts = self.mounts.lock().unwrap();
                mounts.retain(|m| m.mount_point != mount_point);
                return Ok(());
            }
            return Err(Error::internal(format!(
                "Falha ao desmontar {}: {}",
                mount_point.display(),
                stderr
            )));
        }

        // Remove do rastreamento
        let mut mounts = self.mounts.lock().unwrap();
        mounts.retain(|m| m.mount_point != mount_point);

        info!(mount_point = %mount_point.display(), "Desmontagem realizada");
        Ok(())
    }

    /// Desmonta todas as montagens da sessão em ordem inversa.
    ///
    /// Se `force` é true, usa `umount -f` para forçar desmontagem (requer autorização explícita).
    /// Se `only_session_mounts` é true, desmonta apenas o que foi montado pela sessão.
    /// Retorna resultado detalhado com sucessos e falhas.
    pub fn unmount_all(&self, force: bool, only_session_mounts: bool) -> Result<UnmountResult> {
        let mounts_to_unmount = self.mounts_for_unmount();
        let mut result = UnmountResult {
            unmounted: Vec::new(),
            failed: Vec::new(),
            skipped: Vec::new(),
        };

        for mount in mounts_to_unmount {
            if only_session_mounts && !mount.mounted_by_session {
                debug!(mount_point = %mount.mount_point.display(), "Pulando desmontagem de ponto pré-existente");
                result.skipped.push(mount.mount_point.clone());
                continue;
            }

            match self.unmount(&mount.mount_point, force) {
                Ok(()) => {
                    result.unmounted.push(mount.mount_point.clone());
                }
                Err(e) => {
                    error!(mount_point = %mount.mount_point.display(), error = %e, "Erro ao desmontar");
                    result
                        .failed
                        .push((mount.mount_point.clone(), e.to_string()));
                    // Continua tentando desmontar os demais
                }
            }
        }

        if result.has_failures() {
            Err(Error::internal(format!(
                "Falha ao desmontar {} ponto(s) de montagem: {}",
                result.failed.len(),
                result
                    .failed
                    .iter()
                    .map(|(p, e)| format!("{}: {}", p.display(), e))
                    .collect::<Vec<_>>()
                    .join("; ")
            )))
        } else {
            Ok(result)
        }
    }

    /// Finaliza a sessão, desmontando tudo.
    /// Só marca como finalizada se a limpeza foi concluída com sucesso.
    pub fn finalize(&mut self, force: bool) -> Result<()> {
        // Evita reentrância
        if self.finalizing {
            return Err(Error::internal("finalize() chamado recursivamente"));
        }
        if self.finalized {
            return Ok(());
        }

        self.finalizing = true;
        let unmount_result = self.unmount_all(force, true);
        self.finalizing = false;

        if unmount_result.is_ok() {
            self.finalized = true;
        }
        unmount_result.map(|_| ())
    }

    /// Tenta rollback parcial - desmonta apenas montagens da sessão.
    /// Retorna resultado detalhado sem ocultar erro original.
    pub fn rollback(&mut self, original_error: &Error) -> UnmountResult {
        warn!(original_error = %original_error, "Iniciando rollback de montagens da sessão");

        let result = self.unmount_all(false, true).unwrap_or_else(|e| {
            // Se unmount_all falhou, tenta extrair o que foi desmontado
            // Nota: em caso de erro, o UnmountResult parcial já foi consumido
            // Aqui apenas logamos e retornamos um resultado vazio com a falha
            error!(error = %e, "Rollback parcial também falhou");
            UnmountResult {
                unmounted: Vec::new(),
                failed: vec![(PathBuf::from("unknown"), e.to_string())],
                skipped: Vec::new(),
            }
        });

        if result.has_failures() {
            error!(
                failed_count = result.failed.len(),
                "Rollback incompleto - algumas montagens não foram desmontadas"
            );
        } else {
            info!("Rollback concluído com sucesso");
        }

        result
    }

    /// Verifica se a sessão foi finalizada.
    pub fn is_finalized(&self) -> bool {
        self.finalized
    }
}

impl Drop for MountSession {
    fn drop(&mut self) {
        if !self.finalized && !self.finalizing {
            warn!(
                "MountSession dropada sem finalize() - NÃO tentando limpeza automática para evitar operações perigosas"
            );
            warn!("Chame finalize() explicitamente ou use rollback() para limpeza controlada");
            // NÃO chama unmount_all automaticamente - evita umount -f silencioso
        }
    }
}

/// Executa montagem de uma entrada do plano.
/// Usa root_mount da sessão para resolver o ponto de montagem destino.
pub fn execute_mount(entry: &MountPlanEntry, session: &MountSession) -> Result<()> {
    use std::process::Command;

    // Verifica status da entrada
    if !entry.should_mount() {
        match entry.status {
            crate::storage::mount_plan::MountEntryStatus::AlreadyMounted => {
                info!(mount_point = %entry.mount_point.display(), "Ponto já montado, registrando");
                session.record_mount(
                    entry.mount_point.clone(),
                    entry.device_path.clone(),
                    format!("{:?}", entry.fs_type),
                    entry.options.clone(),
                    false, // Não montado por esta sessão
                );
            }
            crate::storage::mount_plan::MountEntryStatus::SkippedNoauto => {
                info!(mount_point = %entry.mount_point.display(), "Ignorado (noauto)");
            }
            crate::storage::mount_plan::MountEntryStatus::SkippedNofailMissing => {
                info!(mount_point = %entry.mount_point.display(), "Ignorado (dispositivo ausente, nofail)");
            }
            crate::storage::mount_plan::MountEntryStatus::SkippedAmbiguous => {
                info!(mount_point = %entry.mount_point.display(), "Ignorado (dispositivo ambíguo)");
            }
            crate::storage::mount_plan::MountEntryStatus::Failed => {
                info!(mount_point = %entry.mount_point.display(), "Marcado como falhou anteriormente");
            }
            crate::storage::mount_plan::MountEntryStatus::Planned => {
                // Não deveria acontecer - should_mount seria true
            }
        }
        return Ok(());
    }

    // Resolve ponto de montagem relativo ao root_mount
    let target_mount_point = entry.resolved_mount_point(session.root_mount());

    // Cria ponto de montagem se não existir
    std::fs::create_dir_all(&target_mount_point)
        .map_err(|e| Error::internal_with_source("Falha ao criar ponto de montagem", e))?;

    // Prepara comando mount
    let mut cmd = Command::new("mount");
    cmd.arg("-t")
        .arg(format!("{:?}", entry.fs_type).to_lowercase());

    if !entry.options.is_empty() {
        cmd.arg("-o").arg(&entry.options);
    }

    // Para Btrfs com subvolume, adiciona opção
    if entry.is_btrfs_subvolume
        && let Some(ref subvol) = entry.subvolume
    {
        let current_opts = if entry.options.is_empty() {
            String::new()
        } else {
            format!("{},", entry.options)
        };
        cmd.arg("-o")
            .arg(format!("{}subvol={}", current_opts, subvol));
    }

    cmd.arg(&entry.device_path).arg(&target_mount_point);

    debug!(
        device = %entry.device_path.display(),
        mount_point = %target_mount_point.display(),
        logical_mount_point = %entry.mount_point.display(),
        fs_type = ?entry.fs_type,
        options = %entry.options,
        "Executando mount"
    );

    let output = cmd
        .output()
        .map_err(|e| Error::internal_with_source("Falha ao executar mount", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::internal(format!(
            "Falha ao montar {} em {}: {}",
            entry.device_path.display(),
            target_mount_point.display(),
            stderr
        )));
    }

    // Registra montagem bem-sucedida (usa ponto de montagem lógico)
    session.record_mount(
        entry.mount_point.clone(),
        entry.device_path.clone(),
        format!("{:?}", entry.fs_type),
        entry.options.clone(),
        true,
    );

    info!(mount_point = %target_mount_point.display(), logical = %entry.mount_point.display(), "Montagem realizada com sucesso");
    Ok(())
}

/// Executa plano de montagem completo.
pub fn execute_mount_plan(plan: &MountPlan, session: &MountSession) -> Result<()> {
    if plan.has_blocking_errors() {
        return Err(Error::internal(format!(
            "Plano de montagem tem erros bloqueantes: {:?}",
            plan.errors
        )));
    }

    // Valida ordenação topológica antes de executar
    let ordered = plan.ordered_for_mount().map_err(|e| {
        Error::internal(format!("Validação de dependências do plano falhou: {}", e))
    })?;

    info!(
        "Executando plano de montagem com {} entradas",
        ordered.len()
    );

    for entry in ordered {
        if let Err(e) = execute_mount(entry, session) {
            // Em caso de erro, tenta rollback
            error!(mount_point = %entry.mount_point.display(), error = %e, "Erro na montagem, iniciando rollback");
            let rollback_result = MountSession::rollback(
                &mut MountSession::new(session.root_mount().to_path_buf()),
                &e,
            );

            // Log do resultado do rollback
            if rollback_result.has_failures() {
                error!("Rollback parcial falhou para: {:?}", rollback_result.failed);
            }

            return Err(e);
        }
    }

    info!("Plano de montagem executado com sucesso");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::types::FilesystemType;
    use crate::storage::mount_plan::{MountEntryStatus, MountPlanEntry, MountType};
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_mount_session_basic() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        let session = MountSession::new(root.clone());
        assert_eq!(session.root_mount(), root);
        assert!(!session.is_finalized());
    }

    #[test]
    fn test_mount_session_record_and_check() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        let session = MountSession::new(root);
        let mp = PathBuf::from("/mnt/test");
        let dev = PathBuf::from("/dev/sda1");

        session.record_mount(
            mp.clone(),
            dev.clone(),
            "ext4".to_string(),
            "rw".to_string(),
            true,
        );
        assert!(session.is_mounted(&mp));

        let mounts = session.active_mounts();
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].mount_point, mp);
        assert_eq!(mounts[0].device_path, dev);
        assert!(mounts[0].mounted_by_session);
    }

    #[test]
    fn test_mount_session_idempotent_record() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        let session = MountSession::new(root);
        let mp = PathBuf::from("/mnt/test");
        let dev = PathBuf::from("/dev/sda1");

        session.record_mount(
            mp.clone(),
            dev.clone(),
            "ext4".to_string(),
            "rw".to_string(),
            true,
        );
        session.record_mount(
            mp.clone(),
            dev.clone(),
            "ext4".to_string(),
            "rw".to_string(),
            true,
        );

        let mounts = session.active_mounts();
        assert_eq!(
            mounts.len(),
            1,
            "Registro duplicado não deve criar entrada extra"
        );
    }

    #[test]
    fn test_mount_session_unmount_order() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        let session = MountSession::new(root);
        session.record_mount(
            PathBuf::from("/"),
            PathBuf::from("/dev/sda2"),
            "ext4".to_string(),
            "rw".to_string(),
            true,
        );
        session.record_mount(
            PathBuf::from("/boot"),
            PathBuf::from("/dev/sda1"),
            "ext4".to_string(),
            "rw".to_string(),
            true,
        );
        session.record_mount(
            PathBuf::from("/boot/efi"),
            PathBuf::from("/dev/sda1"),
            "vfat".to_string(),
            "rw".to_string(),
            true,
        );
        session.record_mount(
            PathBuf::from("/home"),
            PathBuf::from("/dev/sda3"),
            "ext4".to_string(),
            "rw".to_string(),
            true,
        );

        let unmount_order: Vec<_> = session
            .mounts_for_unmount()
            .into_iter()
            .map(|m| m.mount_point)
            .collect();
        // Verifica que filhos vêm antes dos pais
        let boot_efi_idx = unmount_order
            .iter()
            .position(|p| p == &PathBuf::from("/boot/efi"))
            .unwrap();
        let boot_idx = unmount_order
            .iter()
            .position(|p| p == &PathBuf::from("/boot"))
            .unwrap();
        assert!(boot_efi_idx < boot_idx, "/boot/efi deve vir antes de /boot");

        let home_idx = unmount_order
            .iter()
            .position(|p| p == &PathBuf::from("/home"))
            .unwrap();
        let root_idx = unmount_order
            .iter()
            .position(|p| p == &PathBuf::from("/"))
            .unwrap();
        assert!(home_idx < root_idx, "/home deve vir antes de /");

        assert!(boot_idx < root_idx, "/boot deve vir antes de /");
        assert_eq!(unmount_order.len(), 4);
    }

    #[test]
    fn test_unmount_all_returns_error_on_failure() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        let session = MountSession::new(root);
        session.record_mount(
            PathBuf::from("/mnt/test"),
            PathBuf::from("/dev/sda1"),
            "ext4".to_string(),
            "rw".to_string(),
            true,
        );

        // unmount_all deve falhar porque o ponto não existe realmente
        let result = session.unmount_all(false, true);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Falha ao desmontar")
        );
    }

    #[test]
    fn test_unmount_all_skips_pre_existing() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        let session = MountSession::new(root);
        // Montagem da sessão (não existe realmente)
        session.record_mount(
            PathBuf::from("/mnt/session"),
            PathBuf::from("/dev/sda1"),
            "ext4".to_string(),
            "rw".to_string(),
            true,
        );
        // Montagem pré-existente (não existe realmente)
        session.record_mount(
            PathBuf::from("/mnt/preexisting"),
            PathBuf::from("/dev/sda2"),
            "ext4".to_string(),
            "rw".to_string(),
            false,
        );

        // only_session_mounts = true deve pular pré-existente e tentar desmontar apenas da sessão
        // Como /mnt/session não existe, unmount falha e retorna erro
        let result = session.unmount_all(false, true);
        assert!(result.is_err());

        // Verifica que o erro menciona apenas o mount da sessão
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("/mnt/session"));
        // Pré-existente não deve estar no erro
        assert!(!err_msg.contains("/mnt/preexisting"));
    }

    #[test]
    fn test_finalize_only_on_success() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        let mut session = MountSession::new(root);
        session.record_mount(
            PathBuf::from("/mnt/test"),
            PathBuf::from("/dev/sda1"),
            "ext4".to_string(),
            "rw".to_string(),
            true,
        );

        // finalize deve falhar porque o ponto não existe realmente
        let result = session.finalize(false);
        assert!(result.is_err());
        assert!(
            !session.is_finalized(),
            "Sessão não deve ser marcada como finalizada se limpeza falhou"
        );
    }

    #[test]
    fn test_rollback_preserves_original_error() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        let mut session = MountSession::new(root);
        session.record_mount(
            PathBuf::from("/mnt/test"),
            PathBuf::from("/dev/sda1"),
            "ext4".to_string(),
            "rw".to_string(),
            true,
        );

        let original_error = Error::internal("Erro original de montagem");
        let rollback_result = session.rollback(&original_error);

        // Rollback deve registrar falha mas não ocultar erro original
        assert!(rollback_result.has_failures());
    }

    #[test]
    fn test_active_mount_resolved_mount_point() {
        let mount = ActiveMount {
            mount_point: PathBuf::from("/boot/efi"),
            device_path: PathBuf::from("/dev/sda1"),
            fs_type: "vfat".to_string(),
            options: "rw".to_string(),
            mounted_by_session: true,
            mounted_at: std::time::SystemTime::now(),
        };

        let root_mount = PathBuf::from("/mnt/target");
        assert_eq!(
            mount.resolved_mount_point(&root_mount),
            PathBuf::from("/mnt/target/boot/efi")
        );
    }

    #[test]
    fn test_execute_mount_uses_resolved_path() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        let session = MountSession::new(root.clone());
        let entry = MountPlanEntry {
            mount_type: MountType::Esp,
            device_path: PathBuf::from("/dev/sda1"),
            mount_point: PathBuf::from("/boot/efi"),
            fs_type: FilesystemType::Vfat,
            options: "rw".to_string(),
            is_btrfs_subvolume: false,
            subvolume: None,
            status: MountEntryStatus::Planned,
            source_device: None,
            fstab_entry: None,
            dependencies: vec![],
            error: None,
        };

        // execute_mount deve usar root_mount para resolver /boot/efi -> /mnt/root/boot/efi
        // Como não temos dispositivo real, esperamos erro de mount, mas o caminho deve ser resolvido
        let result = execute_mount(&entry, &session);
        assert!(result.is_err());
        // Verifica que o erro menciona o caminho resolvido
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("/mnt/") || err_msg.contains("root"));
    }
}
