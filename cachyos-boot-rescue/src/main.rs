//! cachyos-boot-rescue - Diagnóstico e recuperação de inicialização Linux
//!
//! Foco inicial: CachyOS e Arch Linux.
//! Fase 1: Fundação - estrutura modular, configuração, erros, CLI.

use cachyos_boot_rescue::{app, error::Result};

#[tokio::main]
async fn main() -> Result<()> {
    app::run().await
}
