# Initramfs e mkinitcpio

## Fonte

[ArchWiki: mkinitcpio](https://wiki.archlinux.org/title/Mkinitcpio)

Descreve geração de initramfs, presets de kernel, hooks systemd/BusyBox e
execução automática por hooks de pacman.

BibTeX: `arch-mkinitcpio`.

## Relevância para o projeto

- A configuração em `/etc/mkinitcpio.conf`, os presets instalados e os kernels
  disponíveis determinam quais imagens são esperadas.
- O initramfs pode ser gerado por mais de uma configuração; ausência de um nome
  padrão não prova falha.
- Para a prioridade Arch/CachyOS, identificar a ferramenta instalada e a
  configuração efetiva antes de propor geração.
- Diferencie diagnóstico de regeneração; a segunda é uma operação de escrita.

## Cautela

Não executar `mkinitcpio` em testes usando o sistema do desenvolvedor e não
sobrescrever imagens para experimentar. Use fixtures e ambientes descartáveis.
