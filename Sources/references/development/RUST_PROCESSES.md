# Processos externos em Rust

## Fontes oficiais

- [Rust standard library: `std::process::Command`](https://doc.rust-lang.org/std/process/struct.Command.html)
- [Rust standard library: `std::process::Output`](https://doc.rust-lang.org/std/process/struct.Output.html)

BibTeX: `rust-command`, `rust-output`.

## Relevância para o projeto

- Construa comandos com `Command::new`, `.arg()` e `.args()`; mantenha cada
  argumento separado. Não componha uma linha de shell com entrada do usuário.
- Verifique `Output.status`, `stdout` e `stderr`; trate ausência do executável,
  falha de execução, timeout e status diferente de zero como resultados
  distintos.
- Escolha explicitamente como lidar com ambiente, diretório atual e streams.
  Evite herdar ou registrar dados sensíveis sem necessidade.
- Prefira adaptadores que recebam executável/configuração tipados e sejam
  testáveis com fixtures ou processos simulados.

## Cautela

`Command` não oferece, por si só, timeout nem validação semântica da saída.
Esses limites devem ser implementados e testados no adaptador do projeto.
