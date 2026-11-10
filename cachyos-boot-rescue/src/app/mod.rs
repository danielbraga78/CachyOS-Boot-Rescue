//! Módulo principal da aplicação cachyos-boot-rescue.
//!
//! Contém a lógica de inicialização, boas-vindas e execução principal.

#[allow(clippy::module_inception)]
pub mod app;
pub mod commands;
pub mod events;
pub mod model;
pub mod reducer;
pub mod state;

use std::io::IsTerminal;

use crate::cli::{CliArgs, MergedConfig};
use crate::config::Config;
use crate::error::Result;
use crate::logging::{self, export_session_report};
use clap::Parser;
use tracing::info;

// Re-exports para conveniência
pub use app::App;
pub use commands::AppCommand;
pub use events::AppEvent;
pub use model::{AppModel, MountStage, OpId, OpKind};
pub use reducer::reduce;
pub use state::{AppState, OperationState, SharedData};

/// Executa a aplicação principal.
pub async fn run() -> Result<()> {
    let args = CliArgs::parse();

    // Se --version foi passado, mostra versão e sai
    if args.version {
        print_version();
        return Ok(());
    }

    // Carrega configuração mesclada
    let merged = MergedConfig::load(&args)?;
    let config = merged.config();

    // Inicializa logging
    init_logging(config)?;

    // Se --show-log-dir foi passado, mostra diretório e sai
    if args.show_log_dir {
        if let Some(log_dir) = logging::current_log_dir() {
            println!("{}", log_dir.display());
        } else {
            println!("Logging não inicializado");
        }
        return Ok(());
    }

    // Se --export-report foi passado, exporta relatório e sai
    if let Some(output_path) = &args.export_report {
        let final_path = export_session_report(output_path)?;
        println!("Relatório exportado para: {}", final_path.display());
        return Ok(());
    }

    // Mostra boas-vindas no console em modo CLI de diagnóstico.
    // Se o terminal for interativo, a TUI de fase 5 entra em cena;
    // em ambientes não interativos (CI, testes) mantemos o comportamento
    // seguro e previsível do CLI.
    show_welcome(config);

    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        info!("Terminal não interativo; mantendo execução em modo CLI");
        return Ok(());
    }

    info!("Iniciando interface TUI de recuperação de boot");
    App::run().await
}

/// Inicializa o sistema de logging baseado na configuração.
fn init_logging(config: &Config) -> Result<()> {
    // Usa o novo módulo de logging com configuração completa
    logging::init_logging(config.logging.clone())?;
    Ok(())
}

/// Mostra mensagem de boas-vindas/ajuda inicial.
fn show_welcome(config: &Config) {
    println!("{}", welcome_message(config));
}

/// Gera a mensagem de boas-vindas.
fn welcome_message(config: &Config) -> String {
    let version = env!("CARGO_PKG_VERSION");
    let name = env!("CARGO_PKG_NAME");

    format!(
        r#"
╔══════════════════════════════════════════════════════════════╗
║  {} v{}                                     ║
║  Diagnóstico e recuperação de inicialização Linux            ║
║  Foco: CachyOS / Arch Linux                                  ║
╠══════════════════════════════════════════════════════════════╣
║  Modo: {:<5}  Log: {:<5}  Cor: {:<6}  Progresso: {}        ║
╚══════════════════════════════════════════════════════════════╝

Esta é a Fase 1 - Fundação do projeto.
Funcionalidades de diagnóstico e reparo serão implementadas nas próximas fases.

Use --help para ver opções disponíveis.
"#,
        name,
        version,
        config.interface_mode,
        config.log_level,
        config.color,
        if config.progress { "sim" } else { "não" }
    )
}

/// Imprime apenas a versão.
fn print_version() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_welcome_message_contains_version() {
        let config = Config::default();
        let msg = welcome_message(&config);
        assert!(msg.contains(env!("CARGO_PKG_VERSION")));
        assert!(msg.contains(env!("CARGO_PKG_NAME")));
    }

    #[test]
    fn test_welcome_message_contains_config_info() {
        let config = Config::default();
        let msg = welcome_message(&config);
        assert!(msg.contains("cli")); // interface_mode
        assert!(msg.contains("info")); // log_level
    }

    #[test]
    fn test_print_version() {
        // Apenas verifica que não panic
        print_version();
    }
}
