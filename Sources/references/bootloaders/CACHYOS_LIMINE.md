# Limine no CachyOS

## Conclusão documentada

Para a configuração atual do instalador CachyOS consultada em 2026-11-03,
Limine é a seleção padrão do Calamares. Isso foi confirmado em duas
configurações do instalador: `packagechooser_bootloader.conf` define
`default: limine`, e `bootloader.conf` define `efiBootLoader: "limine"`.

Isso sustenta a descrição de Limine como bootloader padrão do instalador
CachyOS atual; **não** significa que Limine seja obrigatório, que toda versão
histórica ou spin o use, ou que toda máquina CachyOS instalada esteja
atualmente inicializando por Limine. A tela do instalador permite selecionar
outros bootloaders e a configuração usa uma variável de seleção. Uma instalação
existente deve ser diagnosticada pelo estado dos discos e firmware, não pelo
default upstream.

## Evidência upstream consultada

Os arquivos abaixo pertencem ao repositório oficial
[`CachyOS/cachyos-calamares`](https://github.com/CachyOS/cachyos-calamares).
Os links usam o commit
[`e6c22095c3ed4e51725a7cbf9870d453462eb1fb`](https://github.com/CachyOS/cachyos-calamares/commit/e6c22095c3ed4e51725a7cbf9870d453462eb1fb),
para que a evidência não dependa de mudanças futuras na branch:

- [`packagechooser_bootloader.conf`](https://github.com/CachyOS/cachyos-calamares/blob/e6c22095c3ed4e51725a7cbf9870d453462eb1fb/src/modules/packagechooser/packagechooser_bootloader.conf):
  default `limine`; choices também incluem GRUB, rEFInd, systemd-boot e variantes.
- [`bootloader.conf`](https://github.com/CachyOS/cachyos-calamares/blob/e6c22095c3ed4e51725a7cbf9870d453462eb1fb/src/modules/bootloader/bootloader.conf):
  `efiBootLoader: "limine"` e seleção por `packagechooser_bootloader`.
- [`partition.conf`](https://github.com/CachyOS/cachyos-calamares/blob/e6c22095c3ed4e51725a7cbf9870d453462eb1fb/src/modules/partition/partition.conf):
  configuração base de ESP e ajustes por bootloader; para a opção Limine,
  declara `/boot` e tamanho configurado de 4096 MiB.
- [`bootloader/main.py`](https://github.com/CachyOS/cachyos-calamares/blob/e6c22095c3ed4e51725a7cbf9870d453462eb1fb/src/modules/bootloader/main.py):
  integração de bootloader do instalador e geração/ajuste de configuração
  específica do Limine.
- [`pacstrap/main.py`](https://github.com/CachyOS/cachyos-calamares/blob/e6c22095c3ed4e51725a7cbf9870d453462eb1fb/src/modules/pacstrap/main.py):
  mapeia a opção Limine para os pacotes `limine` e `limine-entry-tool`; no
  caminho Btrfs adiciona integração relacionada a Snapper e
  `limine-snapper-sync`.
- [PKGBUILD Limine do CachyOS](https://github.com/CachyOS/CachyOS-PKGBUILDS/tree/master/limine):
  receita do pacote mantida pela organização CachyOS.

O wiki do CachyOS também descreve Limine como escolha nas seções de
[instalação manual](https://wiki.cachyos.org/installation/installation_on_root/)
e [dual boot](https://wiki.cachyos.org/installation/installation_on_root/#dual-booting-windows-and-cachyos).
O wiki é vivo e não substitui a inspeção da configuração instalada.

## Implicações de layout

O instalador tem valores dependentes da opção escolhida. No snapshot citado:

- o ponto configurado para ESP na opção Limine é `/boot`;
- o tamanho configurado para a partição EFI é 4096 MiB;
- o instalador identifica e configura a raiz de acordo com filesystem e
  parâmetros detectados, incluindo casos de Btrfs, ZFS e criptografia.

Esses são valores do fluxo de instalação, não requisitos universais de Limine
nem prova do layout de uma instalação existente. O projeto deve descobrir o
mountpoint real da ESP (por exemplo `/boot`, `/efi` ou `/boot/efi`), a raiz,
subvolumes e parâmetros de criptografia antes de sugerir qualquer ação.

## Integração CachyOS versus upstream Limine

O upstream fornece o formato de `limine.conf` e o bootloader. Ferramentas do
CachyOS podem manter a configuração de alto nível, integrar kernels e
snapshots, ou regenerar entradas. No commit consultado:

- o pacote escolhido com Limine inclui `limine-entry-tool`;
- para raiz Btrfs, o instalador associa os pacotes Snapper e
  `limine-snapper-sync`;
- a integração de configuração é conduzida pelo código do Calamares e não
  deve ser confundida com a sintaxe do arquivo upstream `limine.conf`.

Ao diagnosticar:

1. identifique os pacotes e ferramentas que realmente estão instalados na raiz
   de destino;
2. examine todos os arquivos de configuração relevantes sem executá-los como
   shell;
3. correlacione entradas geradas a kernels/initramfs presentes e mounts;
4. preserve configurações desconhecidas e entradas de outros sistemas;
5. não conclua que a integração Snapper torna todo snapshot inicializável.

## Regra de produto

Limine deve ser tratado como caso prioritário de compatibilidade e
diagnóstico para CachyOS. A seleção padrão do instalador é um prior de
investigação, **não um detector**: nunca pule a confirmação do bootloader
efetivamente instalado/selecionado no firmware.

Nenhum trecho desta nota autoriza instalação, reconfiguração, escrita na ESP,
alteração de BootOrder/BootNext ou criação/exclusão de entradas EFI.
