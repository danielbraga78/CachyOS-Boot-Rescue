# Btrfs e subvolumes

## Fonte primária

[Btrfs documentation: Subvolumes](https://btrfs.readthedocs.io/en/latest/Subvolumes.html)

Explica subvolumes, subvolumes padrão, snapshots, mounts com `subvol`/`subvolid`
e a natureza não recursiva de snapshots.

BibTeX: `btrfs-subvolumes`.

## Relevância para o projeto

- Um subvolume não é uma partição nem um filesystem independente em nível de
  bloco. Modelar dispositivo, filesystem, subvolume e mount separadamente.
- O subvolume default pode mudar o conteúdo apresentado no mount sem mudar o
  device; interpretar a raiz apenas pelo device é insuficiente.
- Layouts de raiz com subvolumes precisam respeitar mounts definidos pelo
  sistema, especialmente antes de preparar um chroot.
- Snapshot não equivale a backup: snapshots compartilham blocos e não protegem
  contra todas as falhas do armazenamento.

## Cautela

Não alterar subvolume default, flags, snapshots ou mounts para "corrigir" uma
detecção. A estrutura real deve ser reportada e a intervenção precisa de
escopo e aprovação próprios.
