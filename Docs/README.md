# CachyOS Boot Rescue

Ferramenta em Rust para orientar o diagnóstico e a recuperação de inicialização Linux, com foco inicial em CachyOS e Arch Linux. O princípio central é identificar o sistema e apresentar evidências antes de qualquer operação que possa modificá-lo.

## Estado

- **Fase 1 — Fundação:** implementada e testada. Cargo package, módulos `error`, `config`, `cli`, `app`, configuração TOML, CLI, tratamento de erros, logging `tracing`.
- **Fase 2 — Logging, sessões e relatórios:** implementada e testada. `tracing` + `tracing-subscriber` + `tracing-appender`, sessões com UUID, rotação diária, sanitização de segredos, exportação tar.gz sanitizada.
- **Fase 3 — Descoberta somente leitura:** implementada e testada. `lsblk --json` parsing, enriquecimento `blkid`, identificação ESP, detecção Linux (montadas + metadados), detecção UEFI/BIOS, classificação filesystems, timeouts/limites de saída, busca recursiva, `removable`/`read_only` como `Option<bool>`, `fstab_content` protegido.
- **Fase 4 — Montagem e Chroot:** implementada e testada. Parser puro fstab, MountPlan com dependências, MountSession com rollback, DeviceResolver, Btrfs subvolumes, LUKS detection, ChrootManager com pseudo-filesystems, execução comandos com timeout/sanitização, validação ambiente.

O documento [CURRENT_STATUS](ai/CURRENT_STATUS.md) é a fonte de verdade para o estado verificado. O [plano de dez fases](planning/CACHYOS_BOOT_RESCUE_10_PHASES_PROMPTS.md) detalha o escopo e os prompts de execução.

## Documentação

| Conteúdo | Documento |
|---|---|
| Guia de entrada e regras de agentes | [START_HERE](ai/START_HERE.md) |
| Estado atual | [CURRENT_STATUS](ai/CURRENT_STATUS.md) |
| Contexto e decisões | [PROJECT_CONTEXT](ai/PROJECT_CONTEXT.md) |
| Mapa por tarefa | [DOCUMENTATION_MAP](ai/DOCUMENTATION_MAP.md) |
| Regras de desenvolvimento | [DEVELOPMENT_RULES](ai/DEVELOPMENT_RULES.md) |
| Fluxo de trabalho | [TASK_WORKFLOW](ai/TASK_WORKFLOW.md) |
| Plano em dez fases | [CACHYOS_BOOT_RESCUE_10_PHASES_PROMPTS](planning/CACHYOS_BOOT_RESCUE_10_PHASES_PROMPTS.md) |
| Proposta histórica supersedida | [CACHYOS_BOOT_RESCUE_AI_DOCUMENTATION_PLAN](planning/CACHYOS_BOOT_RESCUE_AI_DOCUMENTATION_PLAN.md) |

As categorias `architecture/`, `domain/`, `modules/` e `testing/` organizam os contratos especializados existentes. Outras categorias citadas no plano permanecem planejadas até que seus documentos sejam criados.

## Uso e desenvolvimento

O crate Rust permanece em `../cachyos-boot-rescue/`:

```bash
cd cachyos-boot-rescue
cargo build --locked
cargo run -- --help
cargo test --all-targets
```

Pré-requisito documentado pelo crate: Rust 1.99.0 ou posterior. Para build de release e instalação local:

```bash
cargo build --release --locked
cargo install --locked --path .
```

CLI básica:

```bash
cachyos-boot-rescue --help
cachyos-boot-rescue --version
cachyos-boot-rescue --config /caminho/config.toml --log-level debug
```

Configuração TOML opcional:

```toml
log_level = "info"
data_dir = "~/.local/share/cachyos-boot-rescue"
interface_mode = "cli"
color = "auto"
progress = true
```

Para as regras de segurança operacional, consulte o `AGENTS.md` da raiz do repositório. Esta documentação não autoriza operações destrutivas nem altera o status de uma fase.

## Contribuição

Consulte [`ai/DEVELOPMENT_RULES.md`](ai/DEVELOPMENT_RULES.md) e [`ai/TASK_WORKFLOW.md`](ai/TASK_WORKFLOW.md) antes de propor alterações. Use branches específicas para tarefas e mensagens de commit descritivas. Não publique releases nem submeta pacotes AUR automaticamente.

Licença declarada no manifesto atual: MIT OR Apache-2.0. Confirme o estado do repositório antes de distribuir.
