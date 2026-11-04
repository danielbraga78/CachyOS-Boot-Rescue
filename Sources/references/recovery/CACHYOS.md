# Contexto específico do CachyOS

## Fonte primária

[CachyOS Wiki: Installation Desktop/Laptop](https://wiki.cachyos.org/installation/installation_on_root/)

A página apresenta fluxos de instalação e opções de boot manager, incluindo
Limine, systemd-boot, rEFInd e GRUB em cenários descritos pelo instalador.

BibTeX: `cachyos-installation`.

## Relevância para o projeto

- Use nomes de pacotes, ferramentas e convenções específicas do CachyOS apenas
  após confirmar sua presença na instalação de destino.
- As escolhas feitas pelo instalador variam conforme boot manager e firmware;
  não inferir que todo sistema CachyOS tem o mesmo layout.
- O projeto prioriza Arch/CachyOS, mas deve separar adaptadores de distribuição
  e reportar quando uma dependência ou procedimento não estiver disponível.

## Limites

Este material de instalação não é um guia de reparo nem comprova compatibilidade
de hardware. O suporte de cada bootloader e topologia precisa ser validado por
testes próprios e documentado com evidências.
