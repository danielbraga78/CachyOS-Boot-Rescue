# UEFI e EFI System Partition

## Fontes

- [ArchWiki: Unified Extensible Firmware Interface](https://wiki.archlinux.org/title/Unified_Extensible_Firmware_Interface)
- [ArchWiki: EFI system partition](https://wiki.archlinux.org/title/EFI_system_partition)
- [CachyOS Wiki: Installation Desktop/Laptop](https://wiki.cachyos.org/installation/installation_on_root/)

BibTeX: `arch-uefi`, `arch-esp`, `cachyos-installation`.

## Relevância para o projeto

- A presença de `/sys/firmware/efi` diferencia uma inicialização UEFI observável
  de uma inicialização Legacy/BIOS; registre também quando a informação não
  estiver disponível.
- Em GPT, o tipo GUID da ESP é uma evidência mais forte que seu nome, tamanho ou
  filesystem isoladamente.
- A ESP pode conter bootloaders e arquivos de diferentes sistemas operacionais.
  Preservação e ownership são requisitos do reparo.
- As instruções atuais do instalador CachyOS variam o particionamento conforme
  o boot manager selecionado. Use-as para contexto de distribuição, não como
  prova de como uma instalação existente está montada.

## Cautela

Não criar, formatar, redimensionar ou limpar uma ESP como parte de diagnóstico.
Inspecione primeiro os mounts, conteúdo e entradas EFI existentes; qualquer
alteração precisa de autorização explícita e plano reversível.
