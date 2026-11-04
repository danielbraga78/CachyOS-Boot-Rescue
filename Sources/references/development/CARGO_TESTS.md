# Testes Rust com Cargo

## Fonte oficial

[The Cargo Book: `cargo test`](https://doc.rust-lang.org/cargo/commands/cargo-test.html)

Compila e executa testes unitários, de integração e de documentação; descreve
targets, filtros e diretórios de execução.

BibTeX: `cargo-test`.

## Aplicação ao projeto

- Para verificar o crate completo, use o comando prescrito no plano/status,
  como `cargo test --all-targets`; um teste selecionado não substitui a suíte.
- Separe testes unitários dos contratos de adaptadores e integrações.
- A descoberta de discos deve usar JSON de fixtures. Nenhum teste deve depender
  de partições, ESP ou NVRAM reais.
- Registre comandos, status e falhas; não descreva um baseline antigo como
  validação do checkout atual.
