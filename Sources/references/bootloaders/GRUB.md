# GRUB

## Fontes

- [ArchWiki: GRUB](https://wiki.archlinux.org/title/GRUB) — orientação para
  instalação e configuração no Arch; distingue BIOS e UEFI e alerta para
  dependências da ESP e do sistema de arquivos.
- [GNU GRUB Manual](https://www.gnu.org/software/grub/manual/grub/grub.html) —
  referência upstream para conceitos, configuração e ferramentas do GRUB.
- [GNU GRUB: Installing GRUB](https://www.gnu.org/software/grub/manual/grub/html_node/Installing-GRUB-using-grub_002dinstall.html) —
  opções de instalação, incluindo o diretório explícito da ESP em UEFI.

BibTeX: `arch-grub`, `gnu-grub-manual`, `gnu-grub-install`.

## Relevância para o Boot Rescue

- Detectar e reportar GRUB como hipótese baseada em evidências; sua presença
  não prova que seja o bootloader atualmente usado.
- Tratar os caminhos de ESP e `/boot` como dados descobertos, não como
  constantes.
- Modelar BIOS e UEFI como fluxos distintos. Não inferir que opções de
  instalação ou NVRAM de um modo se aplicam ao outro.
- Configuração gerada pode depender dos módulos, kernels, filesystem e
  configuração local; não reconstruir estado somente a partir de defaults.

## Cautela

`grub-install` altera arquivos de boot e pode gravar variáveis EFI. Os exemplos
upstream são documentação, não comandos para executar durante descoberta ou
testes automatizados. Os procedimentos devem ser validados para a instalação
alvo e submetidos às confirmações definidas no projeto.
