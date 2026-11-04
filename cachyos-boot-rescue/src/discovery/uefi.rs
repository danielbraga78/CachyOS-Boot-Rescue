//! Detecção do estado UEFI/BIOS do firmware.
//!
//! Detecta se o sistema iniciou em modo UEFI ou BIOS/Legacy.

use crate::discovery::types::UefiState;
use crate::error::Result;
use std::path::Path;
use tracing::{debug, warn};

/// Caminho para verificar estado UEFI.
const SYS_FIRMWARE_EFI: &str = "/sys/firmware/efi";

/// Verifica se o sistema está rodando em modo UEFI.
///
/// Retorna:
/// - `UefiState::Uefi` se /sys/firmware/efi existe e é acessível
/// - `UefiState::BiosLegacy` se o diretório não existe
/// - `UefiState::Unknown` em caso de erro de acesso
pub fn detect_uefi_state() -> Result<UefiState> {
    debug!("Verificando estado UEFI em {}", SYS_FIRMWARE_EFI);

    let path = Path::new(SYS_FIRMWARE_EFI);

    match path.try_exists() {
        Ok(true) => {
            // Verifica se é um diretório e tem conteúdo esperado
            if path.is_dir() {
                // Tenta listar para confirmar acesso
                match std::fs::read_dir(path) {
                    Ok(_) => {
                        debug!("Sistema detectado em modo UEFI");
                        Ok(UefiState::Uefi)
                    }
                    Err(e) => {
                        warn!(error = %e, "Diretório UEFI existe mas não é legível");
                        Ok(UefiState::Unknown)
                    }
                }
            } else {
                warn!("Caminho UEFI existe mas não é diretório");
                Ok(UefiState::Unknown)
            }
        }
        Ok(false) => {
            debug!("Sistema detectado em modo BIOS/Legacy");
            Ok(UefiState::BiosLegacy)
        }
        Err(e) => {
            warn!(error = %e, "Erro ao verificar estado UEFI");
            Ok(UefiState::Unknown)
        }
    }
}

/// Verifica se variáveis EFI estão acessíveis (apenas em modo UEFI).
pub fn are_efi_variables_accessible() -> bool {
    Path::new("/sys/firmware/efi/efivars")
        .try_exists()
        .unwrap_or(false)
}

/// Obtém lista de variáveis EFI disponíveis (somente leitura).
pub fn list_efi_variables() -> Result<Vec<String>> {
    if !are_efi_variables_accessible() {
        return Ok(Vec::new());
    }

    let path = Path::new("/sys/firmware/efi/efivars");
    let entries = std::fs::read_dir(path)
        .map_err(|e| crate::error::Error::io("lendo variáveis EFI", path.to_path_buf(), e))?;

    let mut vars = Vec::new();
    for entry in entries.flatten() {
        if let Some(name) = entry.file_name().to_str() {
            vars.push(name.to_string());
        }
    }

    Ok(vars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_uefi_state_returns_valid() {
        // Não podemos testar o estado real do sistema, mas verificamos que não panic
        let result = detect_uefi_state();
        assert!(result.is_ok());
        let state = result.unwrap();
        assert!(matches!(
            state,
            UefiState::Uefi | UefiState::BiosLegacy | UefiState::Unknown
        ));
    }
}
