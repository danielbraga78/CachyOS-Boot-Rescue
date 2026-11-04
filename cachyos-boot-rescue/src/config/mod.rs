//! Módulo de configuração tipada do cachyos-boot-rescue.
//!
//! Define a estrutura de configuração com valores padrão seguros,
//! validação e carregamento a partir de arquivo TOML.

use crate::error::{Error, Result, ResultIoExt};
use crate::logging::LoggingConfig;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

/// Nível de log disponível.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Apenas erros.
    Error,
    /// Avisos e erros.
    Warn,
    /// Informações gerais, avisos e erros.
    #[default]
    Info,
    /// Depuração detalhada.
    Debug,
    /// Rastreamento completo.
    Trace,
}

impl FromStr for LogLevel {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "error" => Ok(Self::Error),
            "warn" | "warning" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            _ => Err(Error::validation(
                format!("nível de log inválido: '{s}'. Use: error, warn, info, debug, trace"),
                Some("log_level".to_string()),
            )),
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warn => write!(f, "warn"),
            Self::Info => write!(f, "info"),
            Self::Debug => write!(f, "debug"),
            Self::Trace => write!(f, "trace"),
        }
    }
}

/// Modo de interface do usuário.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum InterfaceMode {
    /// Interface de linha de comando simples.
    #[default]
    Cli,
    /// Interface textual interativa (TUI) - planejado para fase futura.
    #[serde(rename = "tui")]
    Tui,
}

impl FromStr for InterfaceMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "cli" => Ok(Self::Cli),
            "tui" => Ok(Self::Tui),
            _ => Err(Error::validation(
                format!("modo de interface inválido: '{s}'. Use: cli, tui"),
                Some("interface_mode".to_string()),
            )),
        }
    }
}

impl fmt::Display for InterfaceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cli => write!(f, "cli"),
            Self::Tui => write!(f, "tui"),
        }
    }
}

/// Configuração principal do aplicativo.
///
/// Contém apenas opções não perigosas (sem comandos arbitrários,
/// sem caminhos de dispositivos, sem operações de escrita).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Nível de log.
    #[serde(default)]
    pub log_level: LogLevel,

    /// Diretório de dados do aplicativo (cache, estado, etc).
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    /// Modo de interface.
    #[serde(default)]
    pub interface_mode: InterfaceMode,

    /// Se deve usar cores na saída (auto, always, never).
    #[serde(default = "default_color")]
    pub color: ColorChoice,

    /// Se deve mostrar barra de progresso.
    #[serde(default = "default_true")]
    pub progress: bool,

    /// Configuração de logging em arquivo.
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Escolha de cor para saída.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ColorChoice {
    #[default]
    Auto,
    Always,
    Never,
}

impl FromStr for ColorChoice {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "always" | "yes" | "true" => Ok(Self::Always),
            "never" | "no" | "false" => Ok(Self::Never),
            _ => Err(Error::validation(
                format!("escolha de cor inválida: '{s}'. Use: auto, always, never"),
                Some("color".to_string()),
            )),
        }
    }
}

impl fmt::Display for ColorChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Always => write!(f, "always"),
            Self::Never => write!(f, "never"),
        }
    }
}

fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cachyos-boot-rescue")
}

fn default_color() -> ColorChoice {
    ColorChoice::Auto
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: LogLevel::default(),
            data_dir: default_data_dir(),
            interface_mode: InterfaceMode::default(),
            color: ColorChoice::default(),
            progress: true,
            logging: LoggingConfig::default(),
        }
    }
}

impl Config {
    /// Carrega configuração a partir de arquivo TOML.
    ///
    /// Se o arquivo não existir, retorna configuração padrão.
    /// Se o arquivo existir mas for inválido, retorna erro.
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)
            .with_path_context(path, "lendo arquivo de configuração")?;

        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Salva configuração em arquivo TOML.
    #[allow(dead_code)] // API pública para uso futuro
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_path_context(parent, "criando diretório de configuração")?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)
            .with_path_context(path, "escrevendo arquivo de configuração")?;
        Ok(())
    }

    /// Valida a configuração.
    pub fn validate(&self) -> Result<()> {
        // Valida data_dir: deve ser caminho absoluto ou relativo válido
        if self.data_dir.as_os_str().is_empty() {
            return Err(Error::validation(
                "diretório de dados não pode ser vazio",
                Some("data_dir".to_string()),
            ));
        }

        // Não permite caminhos perigosos (exato ou como prefixo de diretório)
        let dangerous = ["/boot", "/efi", "/sys", "/proc", "/dev"];
        for d in dangerous {
            let dangerous_path = PathBuf::from(d);
            if self.data_dir == dangerous_path || self.data_dir.starts_with(format!("{d}/")) {
                return Err(Error::validation(
                    format!("diretório de dados não pode estar em '{d}'"),
                    Some("data_dir".to_string()),
                ));
            }
        }

        // Não permite raiz do sistema
        if self.data_dir == Path::new("/") {
            return Err(Error::validation(
                "diretório de dados não pode ser a raiz do sistema '/'",
                Some("data_dir".to_string()),
            ));
        }

        // Valida configuração de logging
        self.logging.validate()?;

        Ok(())
    }

    /// Retorna o caminho efetivo do diretório de dados (criando se necessário).
    #[allow(dead_code)] // API pública para uso futuro
    pub fn ensure_data_dir(&self) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.data_dir)
            .with_path_context(&self.data_dir, "criando diretório de dados")?;
        Ok(self.data_dir.clone())
    }
}

use std::fmt;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.log_level, LogLevel::Info);
        assert_eq!(config.interface_mode, InterfaceMode::Cli);
        assert_eq!(config.color, ColorChoice::Auto);
        assert!(config.progress);
        assert!(
            config
                .data_dir
                .to_string_lossy()
                .contains("cachyos-boot-rescue")
        );
    }

    #[test]
    fn test_config_from_file_nonexistent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.toml");
        let config = Config::from_file(&path).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_config_from_file_valid() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let content = r#"
log_level = "debug"
data_dir = "/tmp/custom-data"
interface_mode = "cli"
color = "always"
progress = false
"#;
        std::fs::write(&path, content).unwrap();
        let config = Config::from_file(&path).unwrap();
        assert_eq!(config.log_level, LogLevel::Debug);
        assert_eq!(config.data_dir, PathBuf::from("/tmp/custom-data"));
        assert_eq!(config.interface_mode, InterfaceMode::Cli);
        assert_eq!(config.color, ColorChoice::Always);
        assert!(!config.progress);
    }

    #[test]
    fn test_config_from_file_invalid_log_level() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let content = "log_level = \"invalid\"\n";
        std::fs::write(&path, content).unwrap();
        let err = Config::from_file(&path).unwrap_err();
        assert!(err.to_string().contains("inválid"));
    }

    #[test]
    fn test_config_from_file_invalid_interface_mode() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let content = "interface_mode = \"gui\"\n";
        std::fs::write(&path, content).unwrap();
        let err = Config::from_file(&path).unwrap_err();
        assert!(err.to_string().contains("inválid"));
    }

    #[test]
    fn test_config_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let config = Config {
            log_level: LogLevel::Debug,
            color: ColorChoice::Always,
            ..Default::default()
        };
        config.save(&path).unwrap();

        let loaded = Config::from_file(&path).unwrap();
        assert_eq!(loaded.log_level, LogLevel::Debug);
        assert_eq!(loaded.color, ColorChoice::Always);
    }

    #[test]
    fn test_config_validate_empty_data_dir() {
        let config = Config {
            data_dir: PathBuf::new(),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("não pode ser vazio"));
    }

    #[test]
    fn test_config_validate_dangerous_data_dir() {
        let config = Config {
            data_dir: PathBuf::from("/boot/grub"),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("não pode estar em"));
    }

    #[test]
    fn test_log_level_parse() {
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("warning".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("trace".parse::<LogLevel>().unwrap(), LogLevel::Trace);
        assert!("invalid".parse::<LogLevel>().is_err());
    }

    #[test]
    fn test_interface_mode_parse() {
        assert_eq!("cli".parse::<InterfaceMode>().unwrap(), InterfaceMode::Cli);
        assert_eq!("tui".parse::<InterfaceMode>().unwrap(), InterfaceMode::Tui);
        assert!("gui".parse::<InterfaceMode>().is_err());
    }

    #[test]
    fn test_color_choice_parse() {
        assert_eq!("auto".parse::<ColorChoice>().unwrap(), ColorChoice::Auto);
        assert_eq!(
            "always".parse::<ColorChoice>().unwrap(),
            ColorChoice::Always
        );
        assert_eq!("never".parse::<ColorChoice>().unwrap(), ColorChoice::Never);
        assert!("invalid".parse::<ColorChoice>().is_err());
    }
}
