# Mapa da documentação

`Docs/` é o diretório canônico da documentação Markdown do CachyOS Boot Rescue. O `AGENTS.md` da raiz permanece como exceção obrigatória para descoberta de instruções e aponta para esta documentação.

## Documentos atuais

| Documento | Uso |
|---|---|
| [`../README.md`](../README.md) | Visão geral, comandos básicos e índice |
| [`START_HERE.md`](START_HERE.md) | Contexto inicial e regras de navegação |
| [`CURRENT_STATUS.md`](CURRENT_STATUS.md) | Estados e evidências de fases |
| [`PROJECT_CONTEXT.md`](PROJECT_CONTEXT.md) | Decisões e limites do projeto |
| [`DOCUMENTATION_MAP.md`](DOCUMENTATION_MAP.md) | Este mapa |
| [`DEVELOPMENT_RULES.md`](DEVELOPMENT_RULES.md) | Regras de desenvolvimento |
| [`TASK_WORKFLOW.md`](TASK_WORKFLOW.md) | Fluxo por tarefa |
| [`../planning/CACHYOS_BOOT_RESCUE_10_PHASES_PROMPTS.md`](../planning/CACHYOS_BOOT_RESCUE_10_PHASES_PROMPTS.md) | Roadmap e prompts de fases |
| [`../planning/CACHYOS_BOOT_RESCUE_AI_DOCUMENTATION_PLAN.md`](../planning/CACHYOS_BOOT_RESCUE_AI_DOCUMENTATION_PLAN.md) | Proposta histórica supersedida |

## Documentos por tarefa

Os caminhos abaixo são destinos planejados quando marcados como “planejado”; confira se existem antes de lê-los.

| Tarefa | Documentos relevantes |
|---|---|
| Logs, sessões e relatórios (Fase 2) | `modules/logging.md`, `modules/reporting.md`, `safety/PRIVACY.md` — planejados |
| Descoberta de discos e ESPs | `modules/discovery.md`, `domain/FILESYSTEMS.md`, `domain/UEFI.md` — planejados |
| Montagem Btrfs/fstab | `modules/storage.md`, `domain/BTRFS.md`, `domain/FSTAB_CRYPTTAB.md`, `safety/SECURITY_MODEL.md` — planejados |
| Chroot e privilégios | `modules/chroot.md`, `domain/CHROOT.md`, `architecture/PRIVILEGE_MODEL.md` — planejados |
| Diagnóstico e reparo de GRUB/Limine | `modules/diagnostics.md`, `modules/repair_engine.md`, `domain/BOOTLOADERS/`, `safety/ROLLBACK.md` — planejados |
| Testes e compatibilidade | `testing/TEST_STRATEGY.md`, `testing/VM_TESTING.md`, `testing/HARDWARE_MATRIX.md` — planejados |
| Empacotamento | `development/PACKAGING.md`, `development/DEPENDENCIES.md` — planejados |

## Referências externas

O diretório [`../../Sources/references/`](../../Sources/references/README.md)
mantém notas de fontes upstream, catálogo JSON, mapa CSV e bibliografia BibTeX
para bootloaders, armazenamento, recuperação e desenvolvimento. Use as notas
como índice; a documentação upstream é a referência para detalhes atuais.
Para Limine, consulte [`../../Sources/references/bootloaders/LIMINE.md`](../../Sources/references/bootloaders/LIMINE.md)
e [`../../Sources/references/bootloaders/CACHYOS_LIMINE.md`](../../Sources/references/bootloaders/CACHYOS_LIMINE.md):
o snapshot atual do instalador CachyOS seleciona Limine como default, mas isso
não substitui a detecção do bootloader efetivamente usado pela instalação.

## Regras de manutenção

- Uma fonte canônica por assunto; outros documentos devem apontar para ela.
- Diferenciar claramente estado planejado, implementado e validado.
- Atualizar este mapa quando documentos forem movidos ou criados.
- Não manter roadmaps divergentes: o plano em dez fases é a referência de escopo e sequência.
- Documentação de repositórios independentes sob `Sources/` permanece nos seus próprios projetos.
