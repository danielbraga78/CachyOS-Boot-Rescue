# Segurança e manutenção de scripts Shell

## Fontes

- [ShellCheck](https://www.shellcheck.net/) — ferramenta de análise estática
  para scripts Shell.
- [ShellCheck SC2086](https://www.shellcheck.net/wiki/SC2086) — explica os
  riscos de word splitting e globbing em expansões sem aspas.

BibTeX: `shellcheck`, `shellcheck-sc2086`.

## Aplicação ao projeto

- Faça quoting de expansões e passe listas como arrays Bash, evitando
  word splitting e expansão de glob acidentais.
- Evite `eval`, `source` de configurações não confiáveis e concatenação de
  valores em comandos.
- Verifique códigos de saída, stdout e stderr das ferramentas chamadas.
- Execute ShellCheck nos scripts do repositório, mantendo os limites de teste
  declarados em `AGENTS.md`.

## Cautela

Um linter estático não prova que uma operação de armazenamento é segura.
Revisão do fluxo, validação de destino e testes com dados fictícios continuam
necessários.
