# Regras de desenvolvimento

## Princípios

1. Segurança e integridade dos dados têm prioridade.
2. Inspecionar o código, as dependências e o estado do projeto antes de alterar.
3. Separar diagnóstico de execução e preservar o comportamento existente.
4. Preferir módulos focados, tipos explícitos, tratamento de erros e dependências necessárias.
5. Não afirmar compatibilidade ou conclusão sem evidências.

As regras operacionais obrigatórias estão no [`AGENTS.md`](../../AGENTS.md); este documento complementa essas regras para o crate Rust.

## Código e testes

- Respeitar a edição Rust e a MSRV declaradas no manifesto e CI, após verificá-las.
- O contexto existente documenta edition 2024; confirme o valor atual no `Cargo.toml`.
- Preferir APIs seguras e argumentos de processo separados; não executar comandos via shell com dados externos.
- Cobrir novos comportamentos e falhas com testes que usem fixtures, mocks ou VMs descartáveis.
- Não usar discos reais do desenvolvedor em testes automatizados.
- Documentar itens públicos Rust com `///`; evitar warnings e respeitar `cargo clippy -D warnings`.
- Adicionar dependências somente quando necessárias; manter `Cargo.lock` coerente com o manifesto.
- Antes de concluir alterações Rust, executar na raiz do crate:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --locked
```

Se uma verificação falhar, reportar a falha e distinguir problemas preexistentes dos introduzidos pela alteração.

## Fases e documentação

- Trabalhar somente no escopo autorizado da fase atual.
- Atualizar [`CURRENT_STATUS.md`](CURRENT_STATUS.md) com evidências, testes e limitações.
- Registrar decisões arquiteturais relevantes em ADRs sob `Docs/` quando estabelecida a estrutura apropriada.
- Manter os documentos especializados em `Docs/`; não criar cópias Markdown no crate.
- Não avançar de fase por intenção: cumprir pré-requisitos e critérios definidos em [`../planning/CACHYOS_BOOT_RESCUE_10_PHASES_PROMPTS.md`](../planning/CACHYOS_BOOT_RESCUE_10_PHASES_PROMPTS.md).

## Estilo de manutenção

- Evitar mudanças não relacionadas.
- Respeitar alterações locais existentes; não revertê-las sem autorização.
- Não adicionar TODOs, stubs ou testes ignorados para aparentar conclusão.
- Usar commits pequenos e mensagens descritivas quando o mantenedor solicitar commits.
- Convenção documentada para commits: `<tipo>(<escopo>): <descrição curta>`; exemplos de tipos incluem `feat`, `fix`, `docs`, `refactor`, `test`, `chore` e `ci`.
- Fazer pull request com objetivo, alterações, como testar e resultado do CI; atualizar o status/documentação correspondente.

As tabelas de módulos por fase em versões anteriores divergiam do plano de dez fases e foram descartadas; consulte o roadmap canônico antes de escolher um módulo ou antecipar trabalho.
