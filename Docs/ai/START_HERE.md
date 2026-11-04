# Guia de entrada — CachyOS Boot Rescue

Leia este guia antes de trabalhar no projeto. As regras obrigatórias para agentes estão em [`AGENTS.md`](../../AGENTS.md); o estado real está em [`CURRENT_STATUS.md`](CURRENT_STATUS.md); o mapa identifica os documentos específicos de cada tarefa.

## Projeto

CachyOS Boot Rescue é uma aplicação Rust para diagnóstico e recuperação de problemas de inicialização Linux. O foco inicial é CachyOS e Arch Linux, com ênfase em preservar dados, identificar a configuração real e validar cada etapa.

Princípios:

1. Diagnosticar antes de reparar.
2. Identificar sistema, firmware, discos, partições e bootloader antes de agir.
3. Preservar arquivos e entradas de boot existentes.
4. Solicitar confirmação específica para operações mutáveis.
5. Registrar resultados sem expor segredos.

O projeto não deve executar operações destrutivas automaticamente nem presumir UEFI, layout da ESP, filesystem, subvolume ou bootloader.

## Estado e fases

As Fases 1, 2, 3 e 4 estão implementadas e testadas. A fonte canônica do roadmap é [`../planning/CACHYOS_BOOT_RESCUE_10_PHASES_PROMPTS.md`](../planning/CACHYOS_BOOT_RESCUE_10_PHASES_PROMPTS.md); não use roadmaps antigos divergentes.

Leia [`CURRENT_STATUS.md`](CURRENT_STATUS.md) antes de afirmar que uma funcionalidade ou teste está concluído. Implementação, validação automatizada e compatibilidade real são estados distintos.

## Estrutura atual

```text
AGENTS.md                         # Regras obrigatórias de agentes
Docs/
├── README.md                     # Índice e documentação de usuário/desenvolvedor
├── ai/                           # Contexto, status e fluxo de trabalho
├── planning/                     # Planos e prompts
├── architecture/                 # Contratos arquiteturais (quando criados)
├── domain/                       # Conhecimento de domínio Linux (quando criado)
├── modules/                      # Contratos dos módulos (quando criados)
├── safety/                       # Políticas de segurança (quando criadas)
└── testing/                      # Estratégia e evidências (quando criadas)
cachyos-boot-rescue/              # Crate Rust; documentação Markdown centralizada em Docs/
Sources/                          # Repositórios separados; não duplicar código sem justificativa
```

## Build e verificações

Execute os comandos na raiz do crate, `cachyos-boot-rescue/`:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --locked
```

Para detalhes de contexto, regras e tarefas, consulte [`DOCUMENTATION_MAP.md`](DOCUMENTATION_MAP.md).
