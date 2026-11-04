//! Módulo de argumentos CLI do cachyos-boot-rescue.
//!
//! Define os argumentos de linha de comando mínimos:
//! `--version`, `--help` e `--config` opcional.

use crate::config::{ColorChoice, InterfaceMode, LogLevel};
use crate::error::Result;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Argumentos de linha de comando do cachyos-boot-rescue.
#[derive(Debug, Parser)]
#[command(
    name = "cachyos-boot-rescue",
    version,
    about = "CachyOS Boot Rescue - Diagnóstico e recuperação de inicialização Linux",
    long_about = None,
    disable_help_subcommand = true,
    disable_version_flag = true,
    arg_required_else_help = false,
)]
pub struct CliArgs {
    /// Arquivo de configuração TOML.
    #[arg(
        short = 'c',
        long = "config",
        value_name = "PATH",
        help = "Caminho para arquivo de configuração TOML"
    )]
    pub config: Option<PathBuf>,

    /// Nível de log (sobrescreve configuração).
    #[arg(
        long = "log-level",
        value_name = "LEVEL",
        help = "Nível de log: error, warn, info, debug, trace"
    )]
    pub log_level: Option<LogLevelArg>,

    /// Modo de interface (sobrescreve configuração).
    #[arg(
        long = "interface",
        value_name = "MODE",
        help = "Modo de interface: cli, tui"
    )]
    pub interface: Option<InterfaceModeArg>,

    /// Controle de cor na saída (sobrescreve configuração).
    #[arg(
        long = "color",
        value_name = "WHEN",
        help = "Quando usar cores: auto, always, never"
    )]
    pub color: Option<ColorChoiceArg>,

    /// Desabilita barra de progresso.
    #[arg(
        long = "no-progress",
        help = "Desabilita exibição de barra de progresso"
    )]
    pub no_progress: bool,

    /// Diretório de logs (sobrescreve configuração).
    #[arg(
        long = "log-dir",
        value_name = "PATH",
        help = "Diretório para arquivos de log"
    )]
    pub log_dir: Option<PathBuf>,

    /// Habilita/desabilita logging em arquivo.
    #[arg(
        long = "file-logging",
        value_name = "BOOL",
        help = "Habilita logging em arquivo (true/false)"
    )]
    pub file_logging: Option<bool>,

    /// Nível de log para arquivo (sobrescreve configuração).
    #[arg(
        long = "file-log-level",
        value_name = "LEVEL",
        help = "Nível de log para arquivo: error, warn, info, debug, trace"
    )]
    pub file_log_level: Option<LogLevelArg>,

    /// Exporta relatório da sessão atual para arquivo.
    #[arg(
        long = "export-report",
        value_name = "PATH",
        help = "Exporta relatório sanitizado da sessão para arquivo .tar.gz"
    )]
    pub export_report: Option<PathBuf>,

    /// Mostra caminho do diretório de logs e sai.
    #[arg(
        long = "show-log-dir",
        help = "Mostra o diretório de logs configurado e sai"
    )]
    pub show_log_dir: bool,

    /// Mostra versão e sai.
    #[arg(short = 'V', long = "version", help = "Mostra versão e sai")]
    pub version: bool,
}

/// Wrapper para LogLevel com ValueEnum do clap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LogLevelArg {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<LogLevelArg> for LogLevel {
    fn from(arg: LogLevelArg) -> Self {
        match arg {
            LogLevelArg::Error => LogLevel::Error,
            LogLevelArg::Warn => LogLevel::Warn,
            LogLevelArg::Info => LogLevel::Info,
            LogLevelArg::Debug => LogLevel::Debug,
            LogLevelArg::Trace => LogLevel::Trace,
        }
    }
}

/// Wrapper para InterfaceMode com ValueEnum do clap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InterfaceModeArg {
    Cli,
    Tui,
}

impl From<InterfaceModeArg> for InterfaceMode {
    fn from(arg: InterfaceModeArg) -> Self {
        match arg {
            InterfaceModeArg::Cli => InterfaceMode::Cli,
            InterfaceModeArg::Tui => InterfaceMode::Tui,
        }
    }
}

/// Wrapper para ColorChoice com ValueEnum do clap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorChoiceArg {
    Auto,
    Always,
    Never,
}

impl From<ColorChoiceArg> for ColorChoice {
    fn from(arg: ColorChoiceArg) -> Self {
        match arg {
            ColorChoiceArg::Auto => ColorChoice::Auto,
            ColorChoiceArg::Always => ColorChoice::Always,
            ColorChoiceArg::Never => ColorChoice::Never,
        }
    }
}

/// Configuração mesclada: arquivo + CLI (CLI tem prioridade).
#[derive(Debug, Clone)]
#[allow(dead_code)] // campos públicos para uso futuro
pub struct MergedConfig {
    pub config: crate::config::Config,
    pub config_path: Option<PathBuf>,
}

impl MergedConfig {
    /// Carrega configuração do arquivo e aplica sobrescritas do CLI.
    pub fn load(args: &CliArgs) -> Result<Self> {
        let config_path = args.config.clone();
        let default_path = default_config_path();
        let mut config =
            crate::config::Config::from_file(config_path.as_ref().unwrap_or(&default_path))?;

        // Aplica sobrescritas do CLI
        if let Some(level) = args.log_level {
            config.log_level = level.into();
        }
        if let Some(mode) = args.interface {
            config.interface_mode = mode.into();
        }
        if let Some(choice) = args.color {
            config.color = choice.into();
        }
        if args.no_progress {
            config.progress = false;
        }
        if let Some(dir) = &args.log_dir {
            config.logging.log_dir = dir.clone();
        }
        if let Some(enabled) = args.file_logging {
            config.logging.file_logging_enabled = enabled;
        }
        if let Some(level) = args.file_log_level {
            config.logging.file_log_level = level.into();
        }

        config.validate()?;
        Ok(Self {
            config,
            config_path,
        })
    }

    /// Retorna a configuração final.
    pub fn config(&self) -> &crate::config::Config {
        &self.config
    }

    /// Retorna o caminho do arquivo de configuração usado.
    #[allow(dead_code)] // API pública para uso futuro
    pub fn config_path(&self) -> Option<&PathBuf> {
        self.config_path.as_ref()
    }
}

/// Caminho padrão do arquivo de configuração.
fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cachyos-boot-rescue")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cli_args_default() {
        let args = CliArgs::try_parse_from(["cachyos-boot-rescue"]).unwrap();
        assert!(args.config.is_none());
        assert!(args.log_level.is_none());
        assert!(args.interface.is_none());
        assert!(args.color.is_none());
        assert!(!args.no_progress);
        assert!(!args.version);
    }

    #[test]
    fn test_cli_args_config() {
        let args =
            CliArgs::try_parse_from(["cachyos-boot-rescue", "-c", "/tmp/config.toml"]).unwrap();
        assert_eq!(args.config, Some(PathBuf::from("/tmp/config.toml")));
    }

    #[test]
    fn test_cli_args_long_config() {
        let args = CliArgs::try_parse_from(["cachyos-boot-rescue", "--config", "/etc/config.toml"])
            .unwrap();
        assert_eq!(args.config, Some(PathBuf::from("/etc/config.toml")));
    }

    #[test]
    fn test_cli_args_log_level() {
        let args =
            CliArgs::try_parse_from(["cachyos-boot-rescue", "--log-level", "debug"]).unwrap();
        assert_eq!(args.log_level, Some(LogLevelArg::Debug));
    }

    #[test]
    fn test_cli_args_interface() {
        let args = CliArgs::try_parse_from(["cachyos-boot-rescue", "--interface", "tui"]).unwrap();
        assert_eq!(args.interface, Some(InterfaceModeArg::Tui));
    }

    #[test]
    fn test_cli_args_color() {
        let args = CliArgs::try_parse_from(["cachyos-boot-rescue", "--color", "never"]).unwrap();
        assert_eq!(args.color, Some(ColorChoiceArg::Never));
    }

    #[test]
    fn test_cli_args_no_progress() {
        let args = CliArgs::try_parse_from(["cachyos-boot-rescue", "--no-progress"]).unwrap();
        assert!(args.no_progress);
    }

    #[test]
    fn test_cli_args_version() {
        let args = CliArgs::try_parse_from(["cachyos-boot-rescue", "--version"]).unwrap();
        assert!(args.version);
    }

    #[test]
    fn test_merged_config_load_default() {
        let dir = tempdir().unwrap();
        let args = CliArgs {
            config: Some(dir.path().join("nonexistent.toml")),
            log_level: None,
            interface: None,
            color: None,
            no_progress: false,
            version: false,
            export_report: None,
            file_log_level: None,
            file_logging: None,
            show_log_dir: false,
            log_dir: None,
        };
        let merged = MergedConfig::load(&args).unwrap();
        assert_eq!(merged.config.log_level, LogLevel::Info);
        assert_eq!(merged.config.interface_mode, InterfaceMode::Cli);
    }

    #[test]
    fn test_merged_config_cli_overrides() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        std::fs::write(
            &config_path,
            "log_level = \"warn\"\ninterface_mode = \"cli\"\n",
        )
        .unwrap();

        let args = CliArgs {
            config: Some(config_path),
            log_level: Some(LogLevelArg::Debug),
            interface: Some(InterfaceModeArg::Tui),
            color: Some(ColorChoiceArg::Always),
            no_progress: true,
            version: false,
            export_report: None,
            file_log_level: None,
            file_logging: None,
            show_log_dir: false,
            log_dir: None,
        };
        let merged = MergedConfig::load(&args).unwrap();
        assert_eq!(merged.config.log_level, LogLevel::Debug); // CLI override
        assert_eq!(merged.config.interface_mode, InterfaceMode::Tui); // CLI override
        assert_eq!(merged.config.color, ColorChoice::Always); // CLI override
        assert!(!merged.config.progress); // CLI override
    }

    #[test]
    fn test_log_level_arg_conversion() {
        assert_eq!(LogLevel::from(LogLevelArg::Error), LogLevel::Error);
        assert_eq!(LogLevel::from(LogLevelArg::Warn), LogLevel::Warn);
        assert_eq!(LogLevel::from(LogLevelArg::Info), LogLevel::Info);
        assert_eq!(LogLevel::from(LogLevelArg::Debug), LogLevel::Debug);
        assert_eq!(LogLevel::from(LogLevelArg::Trace), LogLevel::Trace);
    }

    #[test]
    fn test_interface_mode_arg_conversion() {
        assert_eq!(
            InterfaceMode::from(InterfaceModeArg::Cli),
            InterfaceMode::Cli
        );
        assert_eq!(
            InterfaceMode::from(InterfaceModeArg::Tui),
            InterfaceMode::Tui
        );
    }

    #[test]
    fn test_color_choice_arg_conversion() {
        assert_eq!(ColorChoice::from(ColorChoiceArg::Auto), ColorChoice::Auto);
        assert_eq!(
            ColorChoice::from(ColorChoiceArg::Always),
            ColorChoice::Always
        );
        assert_eq!(ColorChoice::from(ColorChoiceArg::Never), ColorChoice::Never);
    }
}
