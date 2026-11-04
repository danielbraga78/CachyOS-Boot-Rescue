# Contexto do projeto — CachyOS Boot Rescue

## Propósito

Construir uma ferramenta em Rust para diagnosticar e recuperar problemas de boot Linux, priorizando CachyOS e Arch Linux. A ferramenta deve identificar a instalação e seu ambiente antes de qualquer alteração, preservar dados e produzir evidências verificáveis.

## Estado confirmado e limites

- A fonte do plano progressivo é [`../planning/CACHYOS_BOOT_RESCUE_10_PHASES_PROMPTS.md`](../planning/CACHYOS_BOOT_RESCUE_10_PHASES_PROMPTS.md).
- As Fases 1, 2, 3 e 4 (fundação, logging/sessões/relatórios, descoberta somente leitura, montagem/Chroot) estão implementadas e testadas. Ver [`CURRENT_STATUS.md`](CURRENT_STATUS.md) para evidências.
- O crate Cargo está em `cachyos-boot-rescue/`; a documentação Markdown do produto fica em `Docs/`.
- `Sources/` contém projetos/repositórios separados; sua existência não significa que seus módulos devam ser copiados para este crate.
- A implementação efetiva e a validação devem ser confirmadas no código e em [`CURRENT_STATUS.md`](CURRENT_STATUS.md).

## Decisões de arquitetura para o plano atual

- Separar diagnóstico somente leitura de operações mutáveis.
- Usar comandos externos com argumentos separados; nunca interpolar entrada em uma shell.
- Representar ausência de informação como desconhecida, não como um valor presumido.
- Tratar layouts `/boot`, `/efi` e `/boot/efi` como possibilidades distintas.
- Fazer backup, mostrar o plano e pedir confirmação específica antes de qualquer reparo.
- Não usar discos físicos do desenvolvedor em testes automatizados.
- Registrar eventos sem segredos e exportar relatórios somente após sanitização.

## Foco de plataforma

Prioridade inicial: Arch Linux e CachyOS. GRUB e Limine são os bootloaders de reparo previstos no plano de dez fases. A configuração atual do instalador CachyOS consultada em 2026-11-03 seleciona Limine como padrão, conforme o snapshot do Calamares referenciado em [`../../Sources/references/bootloaders/CACHYOS_LIMINE.md`](../../Sources/references/bootloaders/CACHYOS_LIMINE.md). O instalador oferece alternativas; esse padrão não comprova qual bootloader está ativo numa instalação específica, nem se aplica automaticamente a instalações antigas. A detecção deve reconhecer configurações diversas e não pode presumir que o bootloader planejado esteja instalado.

Suporte a outras distribuições, Secure Boot, configurações complexas de RAID/LVM, migração de bootloader e layouts não testados devem ser tratados como não confirmados até haver decisão e evidência específicas.

## Toolchain e comandos

O manifesto atual do crate declara Rust 1.99.0. Verifique a toolchain disponível e o estado real do manifesto antes de depender dessa versão.

O contexto anterior registra `clap`, `serde`, `toml`, `thiserror`, `anyhow`, `tracing`, `tracing-subscriber`, `tracing-appender`, `chrono`, `uuid`, `flate2`, `tar`, `regex`, `serde_json`, `dirs` e `tokio` como dependências do crate. Confirme `Cargo.toml` para versões e uso reais antes de alterar dependências.

O arranjo Rust inicialmente documentado inclui os módulos `app`, `cli`, `config`, `error` e `logging`; sua compilação deve ser validada no checkout atual, não inferida deste resumo.

O contexto anterior também registrava as opções CLI `--config`, `--log-level`, `--interface`, `--color`, `--no-progress`, `--version` e `--help`; configuração tipada TOML com nível de log, diretório de dados, modo de interface, cor e progresso; e validação para rejeitar caminhos críticos (`/`, `/boot`, `/efi`, `/sys`, `/proc`, `/dev`). Verifique o comportamento no código antes de afirmar que esses controles estão ativos.

Os erros anteriormente descritos incluíam contexto para operações de I/O, mensagens sem stack trace por padrão e códigos de saída POSIX. Trate esses detalhes como histórico a confirmar, não como contrato validado.

Na raiz do crate:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --locked
```

Consulte a documentação canônica antes de alterar contratos, dependências ou escopo de fase.
