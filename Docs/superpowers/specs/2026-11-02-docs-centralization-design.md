# Design: Centralizar e consolidar a documentação

## Intenção

Eliminar cópias e estados contraditórios, deixando `/home/daniel/Projetos/CachyOS-Boot-Rescue/Docs/` como local canônico de toda documentação Markdown do projeto. Manter somente `AGENTS.md` na raiz como exceção obrigatória para descoberta de instruções.

## Objetivos e limites

- Consolidar contexto, status, regras, fluxo de trabalho e mapa documental em `Docs/ai/`.
- Organizar os planos em `Docs/planning/` e criar um índice em `Docs/README.md`.
- Mover todos os Markdown atualmente na raiz e em `cachyos-boot-rescue/` para `Docs/`, removendo as cópias de origem em vez de manter stubs.
- Manter `AGENTS.md` na raiz como única exceção, preservando suas regras atuais e corrigindo instruções para os novos caminhos canônicos.
- Remover a referência a `README.md` do `cachyos-boot-rescue/Cargo.toml` (usar `readme = false`) para não deixar metadado apontando para um arquivo que será movido.
- Consolidar os status divergentes preservando a Fase 2 como **em andamento**, conforme instrução do mantenedor. Não inferir encerramento da Fase 2.
- Preservar alterações locais existentes e não mover código, CI ou repositórios sob `Sources/`.
- Manter os arquivos não Markdown necessários ao crate (manifesto Cargo, lockfile e workflow); “somente códigos Rust” significa sem documentação Markdown dentro do crate.
- Não mover `.agents/skills/**.md`: esses arquivos compõem ferramentas de agente carregadas pelos caminhos atuais, não a documentação do produto.

## Abordagens consideradas

1. **`Docs/` canônica; `AGENTS.md` é a única exceção na raiz — escolhida.** Atende à centralização literal pedida, elimina conteúdo repetido e mantém o ponto de descoberta exigido por agentes.
2. **`cachyos-boot-rescue/docs/` canônica.** Mantém documentação junto ao crate, mas contradiz a escolha explícita do mantenedor de centralizar em `/Docs`.

## Decisões de implementação

- Usar subdiretórios `Docs/ai/` e `Docs/planning/`; preservar categorias já existentes em planos como `modules`, `domain`, `safety`, `testing` e `architecture` quando seus documentos forem criados.
- Tratar o plano de dez fases como referência do roadmap. O status canônico registrará Fase 1 conforme evidência documental existente e Fase 2 em andamento, sem marcar conclusão não autorizada.
- Mesclar conteúdo exclusivo de documentos duplicados no documento canônico correspondente antes de remover a cópia.
- Preservar os repositórios independentes dentro de `Sources/`; não reorganizar os seus READMEs ou documentos.
- Preservar as skills instaladas em `.agents/skills/`; mover seus Markdown quebraria os caminhos de carregamento das ferramentas.
- Preservar todo conteúdo local modificado/não rastreado ao consolidar e mover documentos; remover origem somente após confirmar que seu conteúdo está no destino canônico.

## Validação

Verificar links relativos, caminhos citados nas instruções, ausência de Markdown de documentação do produto fora de `Docs/` e do `AGENTS.md` raiz, consistência do status da Fase 2 e estado final do Git. Preservar Markdown de ferramentas sob `.agents/skills/` e repositórios independentes sob `Sources/`. Validar o metadado Cargo após retirar o README do crate. O baseline `cargo test --all-targets` falhou antes das mudanças por imports `crate::logging` não resolvidos em `src/app/mod.rs` e `src/cli/mod.rs`; o mantenedor autorizou seguir somente com documentação, sem corrigir Rust.
