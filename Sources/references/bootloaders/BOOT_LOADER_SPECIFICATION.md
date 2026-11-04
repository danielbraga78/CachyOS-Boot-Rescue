# UAPI.1 — Boot Loader Specification

## Fonte primária

[UAPI.1 Boot Loader Specification](https://uapi-group.org/specifications/specs/boot_loader_specification/)

Define formatos e convenções de nomes para entradas de boot compartilhadas por
sistemas operacionais, bootloaders e ferramentas de userspace. A especificação
descreve diretórios compartilhados na ESP e, quando aplicável, na partição
XBOOTLDR; não define todo o conteúdo dessas partições.

BibTeX: `uapi-bls`.

## Aplicação ao projeto

- Não tratar `/loader/entries/` ou `/EFI/Linux/` como propriedade exclusiva de
  um único SO ou bootloader.
- Preservar entradas desconhecidas/externas durante diagnóstico e reparo.
- Em GPT, distinguir ESP de XBOOTLDR pelos tipos GUID, sem inferir a função
  somente pelo filesystem ou ponto de montagem.
- Considerar as restrições de case-insensitivity e de arquivos regulares/
  diretórios descritas pela especificação ao analisar entradas.

## Limite

BLS cobre formatos e convenções de boot; não é uma especificação de segurança
para executar reparos e não substitui a identificação do layout real da máquina.
