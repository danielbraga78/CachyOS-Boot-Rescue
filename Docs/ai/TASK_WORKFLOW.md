# Fluxo de trabalho para tarefas

## Início

1. Leia o `AGENTS.md` da raiz, [`START_HERE.md`](START_HERE.md), [`CURRENT_STATUS.md`](CURRENT_STATUS.md) e [`DOCUMENTATION_MAP.md`](DOCUMENTATION_MAP.md).
2. Confirme a fase e os pré-requisitos no estado atual e no plano de dez fases.
3. Inspecione arquivos, APIs, testes e dependências existentes antes de propor alterações.
4. Leia apenas os contratos e decisões relevantes ao escopo.
5. Se o estado de uma operação puder afetar discos, EFI, NVRAM ou dados, pare diante de ambiguidades e peça autorização/informação.

## Implementação

1. Faça uma alteração pequena e mantenha diagnóstico separado de mutação.
2. Escreva testes compatíveis com o comportamento novo; use mocks/fixtures ou VMs descartáveis para interações com armazenamento.
3. Execute as verificações pertinentes, registre comandos e resultados reais e não esconda falhas.
4. Atualize os documentos canônicos em `Docs/` e o status com evidências.
5. Não inicie a fase seguinte na mesma tarefa.

## Entrega

Informe:

- arquivos criados, alterados ou movidos;
- decisões tomadas;
- comandos executados e resultados;
- limitações, bloqueios e cenários não testados;
- confirmação de que o escopo não avançou para outra fase.

## Branch, revisão e merge

- Use uma branch específica para a tarefa (`feature/*`, `fix/*` ou `docs/*`).
- Para issues, registre objetivo, critérios de aceitação e dependências; não presuma que a numeração ou labels sugeridas em documentos antigos estejam configuradas no repositório.
- Em pull requests, informe o que mudou, por que mudou, como validar e o estado do CI.
- Faça merge ou publicação somente quando solicitado/autorizado pelo mantenedor.
- Após merge autorizado, confirme o estado do branch principal e atualize o status e changelog se aplicável.

## Comandos do crate

Execute na pasta `cachyos-boot-rescue/`:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --locked
```
