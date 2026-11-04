# systemd-boot

## Fontes

- [ArchWiki: systemd-boot](https://wiki.archlinux.org/title/Systemd-boot) —
  requisitos UEFI, entradas e relação com ESP/XBOOTLDR.
- [systemd `bootctl`](https://www.freedesktop.org/software/systemd/man/latest/bootctl.html) —
  opções e comportamento da ferramenta upstream, incluindo descoberta de ESP.
- [UAPI.1 Boot Loader Specification](https://uapi-group.org/specifications/specs/boot_loader_specification/) —
  convenções de entradas compartilhadas entre sistemas e bootloaders.

BibTeX: `arch-systemd-boot`, `systemd-bootctl`, `uapi-bls`.

## Relevância para o Boot Rescue

- systemd-boot opera em UEFI e carrega executáveis EFI compatíveis; isso o
  diferencia de um detector genérico de arquivos em `/boot`.
- A documentação upstream de `bootctl` procura a ESP em `/efi`, `/boot` e
  `/boot/efi` quando nenhum caminho é indicado. Isso não autoriza assumir
  qualquer um desses pontos no projeto: a montagem real deve ser descoberta.
- Considere entradas Type #1 e imagens Type #2 conforme BLS e preserve entradas
  de outros sistemas no namespace compartilhado.
- Separe consultas somente leitura de ações de instalação/atualização e de
  alterações em variáveis EFI.

## Cautela

`bootctl install` pode gravar arquivos na ESP e atualizar a ordem de boot.
Qualquer rotina mutável requer plano explícito, validação do destino, backup ou
rollback e confirmação do usuário.
