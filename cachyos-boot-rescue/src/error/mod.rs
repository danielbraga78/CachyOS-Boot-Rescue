//! Módulo central de tratamento de erros do cachyos-boot-rescue.
//!
//! Fornece tipos de erro com contexto, conversão de erros de I/O e
//! mensagens compreensíveis para o usuário final sem stack traces por padrão.

use std::{io, path::PathBuf};
use thiserror::Error;

/// Tipo de resultado padrão do projeto.
pub type Result<T> = std::result::Result<T, Error>;

/// Enumeração principal de erros do cachyos-boot-rescue.
///
/// Cada variante carrega contexto suficiente para diagnóstico e
/// apresenta mensagens amigáveis ao usuário.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Erro de configuração inválida ou ausente.
    #[error("Configuração inválida: {message}")]
    Config {
        /// Descrição do problema de configuração.
        message: String,
        /// Caminho do arquivo de configuração, se aplicável.
        path: Option<PathBuf>,
        /// Erro de origem (ex: parsing TOML).
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Erro de validação de entrada do usuário ou argumentos CLI.
    #[error("Validação falhou: {message}")]
    Validation {
        /// Descrição do erro de validação.
        message: String,
        /// Campo ou argumento que falhou.
        field: Option<String>,
    },

    /// Erro de I/O com contexto de caminho.
    #[error("Erro de E/S em '{}': {}", path.display(), message)]
    Io {
        /// Mensagem descritiva da operação que falhou.
        message: String,
        /// Caminho envolvido na operação.
        path: PathBuf,
        /// Erro de I/O original.
        #[source]
        source: io::Error,
    },

    /// Erro ao executar comando externo.
    #[allow(dead_code)] // Para fases futuras
    #[error("Falha ao executar comando '{command}': {message}")]
    Command {
        /// Comando que foi executado.
        command: String,
        /// Descrição do erro.
        message: String,
        /// Código de saída do processo, se disponível.
        exit_code: Option<i32>,
        /// Saída de erro do processo.
        stderr: Option<String>,
    },

    /// Erro de sistema de arquivos (montagem, permissões, etc).
    #[allow(dead_code)] // Para fases futuras
    #[error("Erro de sistema de arquivos: {message}")]
    Filesystem {
        /// Descrição do erro.
        message: String,
        /// Caminho relacionado.
        path: Option<PathBuf>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Erro de detecção de hardware/firmware/partições.
    #[allow(dead_code)] // Para fases futuras
    #[error("Erro de detecção: {message}")]
    Detection {
        /// Descrição do que falhou ao detectar.
        message: String,
        /// Componente que estava sendo detectado.
        component: Option<String>,
    },

    /// Erro de operação não suportada no ambiente atual.
    #[allow(dead_code)] // Para fases futuras
    #[error("Operação não suportada: {message}")]
    Unsupported {
        /// Descrição da operação não suportada.
        message: String,
        /// Razão técnica.
        reason: Option<String>,
    },

    /// Erro de operação bloqueada por outra operação mutável concorrente.
    #[error("Operação bloqueada: {message}")]
    OperationBlocked {
        /// Descrição da operação que tentou rodar.
        message: String,
    },

    /// Erro de estado inválido (ex: pré-condição não atendida).
    #[allow(dead_code)] // Para fases futuras
    #[error("Estado inválido: {message}")]
    InvalidState {
        /// Descrição do estado inválido.
        message: String,
        /// Estado esperado vs atual.
        expected: Option<String>,
    },

    /// Erro interno inesperado (bug).
    #[error("Erro interno: {message}")]
    Internal {
        /// Descrição do erro interno.
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl Error {
    /// Cria um erro de configuração.
    #[allow(dead_code)] // Para fases futuras
    pub fn config(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self::Config {
            message: message.into(),
            path,
            source: None,
        }
    }

    /// Cria um erro de configuração com causa.
    #[allow(dead_code)] // Para fases futuras
    pub fn config_with_source(
        message: impl Into<String>,
        path: Option<PathBuf>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Config {
            message: message.into(),
            path,
            source: Some(Box::new(source)),
        }
    }

    /// Cria um erro de validação.
    pub fn validation(message: impl Into<String>, field: Option<String>) -> Self {
        Self::Validation {
            message: message.into(),
            field,
        }
    }

    /// Cria um erro de I/O com contexto.
    pub fn io(message: impl Into<String>, path: PathBuf, source: io::Error) -> Self {
        Self::Io {
            message: message.into(),
            path,
            source,
        }
    }

    /// Cria um erro de comando externo.
    #[allow(dead_code)] // Para fases futuras
    pub fn command(
        command: impl Into<String>,
        message: impl Into<String>,
        exit_code: Option<i32>,
        stderr: Option<String>,
    ) -> Self {
        Self::Command {
            command: command.into(),
            message: message.into(),
            exit_code,
            stderr,
        }
    }

    /// Cria um erro de sistema de arquivos.
    #[allow(dead_code)] // Para fases futuras
    pub fn filesystem(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self::Filesystem {
            message: message.into(),
            path,
            source: None,
        }
    }

    /// Cria um erro de detecção.
    #[allow(dead_code)] // Para fases futuras
    pub fn detection(message: impl Into<String>, component: Option<String>) -> Self {
        Self::Detection {
            message: message.into(),
            component,
        }
    }

    /// Cria um erro de operação não suportada.
    #[allow(dead_code)] // Para fases futuras
    pub fn unsupported(message: impl Into<String>, reason: Option<String>) -> Self {
        Self::Unsupported {
            message: message.into(),
            reason,
        }
    }

    /// Cria um erro de operação bloqueada.
    pub fn operation_blocked(message: impl Into<String>) -> Self {
        Self::OperationBlocked {
            message: message.into(),
        }
    }

    /// Cria um erro de estado inválido.
    #[allow(dead_code)] // Para fases futuras
    pub fn invalid_state(message: impl Into<String>, expected: Option<String>) -> Self {
        Self::InvalidState {
            message: message.into(),
            expected,
        }
    }

    /// Cria um erro interno.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            source: None,
        }
    }

    /// Cria um erro interno com causa.
    pub fn internal_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Internal {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Retorna uma mensagem amigável para o usuário (sem stack trace).
    #[allow(dead_code)] // Para fases futuras
    pub fn user_message(&self) -> String {
        self.to_string()
    }

    /// Retorna se o erro é recuperável (pode tentar novamente).
    #[allow(dead_code)] // Para fases futuras
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Error::Io { .. } | Error::Command { .. } | Error::Filesystem { .. }
        )
    }

    /// Retorna o código de saída sugerido para o processo.
    #[allow(dead_code)] // Para fases futuras
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::Config { .. } => 64,       // EX_USAGE
            Error::Validation { .. } => 64,   // EX_USAGE
            Error::Io { .. } => 74,           // EX_IOERR
            Error::Command { .. } => 70,      // EX_SOFTWARE
            Error::Filesystem { .. } => 73,   // EX_CANTCREAT
            Error::Detection { .. } => 69,    // EX_UNAVAILABLE
            Error::Unsupported { .. } => 78,  // EX_CONFIG
            Error::OperationBlocked { .. } => 70, // EX_SOFTWARE
            Error::InvalidState { .. } => 70, // EX_SOFTWARE
            Error::Internal { .. } => 70,     // EX_SOFTWARE
        }
    }
}

/// Extensão para adicionar contexto a erros de I/O.
pub trait IoErrorExt {
    /// Adiciona contexto de caminho e mensagem a um erro de I/O.
    fn with_path_context(self, path: &std::path::Path, msg: &str) -> Error;
}

impl IoErrorExt for io::Error {
    fn with_path_context(self, path: &std::path::Path, msg: &str) -> Error {
        Error::io(msg, path.to_path_buf(), self)
    }
}

/// Extensão para `Result` de I/O adicionar contexto.
pub trait ResultIoExt<T> {
    /// Adiciona contexto de caminho a um resultado de I/O.
    fn with_path_context(self, path: &std::path::Path, msg: &str) -> Result<T>;
}

impl<T> ResultIoExt<T> for std::result::Result<T, io::Error> {
    fn with_path_context(self, path: &std::path::Path, msg: &str) -> Result<T> {
        self.map_err(|e| e.with_path_context(path, msg))
    }
}

/// Conversão de `anyhow::Error` para `Error`.
impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        // anyhow::Error não implementa std::error::Error diretamente em todas versões
        Error::internal(format!("Erro inesperado: {err}"))
    }
}

/// Conversão de `io::Error` para `Error` (sem contexto de caminho).
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::io("Erro de E/S", PathBuf::from("?"), err)
    }
}

/// Conversão de `toml::de::Error` para `Error`.
impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::config_with_source("Falha ao parsear configuração TOML", None, err)
    }
}

/// Conversão de `toml::ser::Error` para `Error`.
impl From<toml::ser::Error> for Error {
    fn from(err: toml::ser::Error) -> Self {
        Error::config_with_source("Falha ao serializar configuração TOML", None, err)
    }
}

/// Conversão de `clap::Error` para `Error`.
impl From<clap::Error> for Error {
    fn from(err: clap::Error) -> Self {
        Error::validation(err.to_string(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_creation() {
        let err = Error::config(
            "arquivo não encontrado",
            Some(PathBuf::from("/etc/config.toml")),
        );
        assert!(err.to_string().contains("Configuração inválida"));
        assert!(err.to_string().contains("arquivo não encontrado"));
    }

    #[test]
    fn test_error_io_with_context() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "arquivo não existe");
        let err = Error::io("lendo arquivo", PathBuf::from("/tmp/test.txt"), io_err);
        assert!(err.to_string().contains("Erro de E/S"));
        assert!(err.to_string().contains("/tmp/test.txt"));
    }

    #[test]
    fn test_error_exit_codes() {
        assert_eq!(Error::config("test", None).exit_code(), 64);
        assert_eq!(Error::validation("test", None).exit_code(), 64);
        assert_eq!(
            Error::io("test", PathBuf::new(), io::Error::other("x")).exit_code(),
            74
        );
    }

    #[test]
    fn test_error_is_recoverable() {
        let io_err = Error::io("test", PathBuf::new(), io::Error::other("x"));
        assert!(io_err.is_recoverable());

        let config_err = Error::config("test", None);
        assert!(!config_err.is_recoverable());
    }

    #[test]
    fn test_io_error_ext() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "acesso negado");
        let err = io_err.with_path_context(&PathBuf::from("/root/secret"), "lendo segredo");
        assert!(err.to_string().contains("/root/secret"));
        assert!(err.to_string().contains("lendo segredo"));
    }

    #[test]
    fn test_result_io_ext() {
        let result: std::result::Result<(), io::Error> =
            Err(io::Error::new(io::ErrorKind::NotFound, "x"));
        let err = result
            .with_path_context(&PathBuf::from("/missing"), "procurando")
            .unwrap_err();
        assert!(err.to_string().contains("/missing"));
    }
}
