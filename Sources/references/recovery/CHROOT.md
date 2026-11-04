# Chroot em sistemas Arch/CachyOS

## Fontes

- [ArchWiki: Chroot](https://wiki.archlinux.org/title/Chroot)
- [Arch Linux man page: `arch-chroot(8)`](https://man.archlinux.org/man/arch-chroot.8)

BibTeX: `arch-chroot-wiki`, `arch-chroot-man`.

## Relevância para o projeto

- O diretório alvo deve ser a raiz da instalação correta e estar montado com
  seus filesystems adicionais conforme a configuração local, inclusive ESP,
  `/boot`, Btrfs subvolumes e volumes desbloqueados quando aplicável.
- `arch-chroot` prepara filesystems de API como `/dev` e `/proc` e trata
  `/etc/resolv.conf`; o projeto não deve presumir que um `chroot` cru ofereça
  o mesmo ambiente.
- A existência de um diretório montado não é evidência suficiente de que seja a
  raiz da instalação pretendida.
- Verifique ferramentas, mounts, rede e requisitos de privilégio antes de
  qualquer procedimento dentro do chroot.

## Cautela

Entrar em chroot não torna seguro qualquer comando subsequente. Reparos ainda
podem alterar kernels, ESP, NVRAM ou configuração. Não execute comandos
privilegiados no host durante os testes; use mocks, fixtures ou VMs descartáveis.
