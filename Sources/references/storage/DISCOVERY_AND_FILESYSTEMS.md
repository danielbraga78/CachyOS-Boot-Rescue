# Descoberta de dispositivos e filesystems

## Fontes

- [util-linux `lsblk(8)`](https://man7.org/linux/man-pages/man8/lsblk.8.html)
- [util-linux `blkid(8)`](https://man7.org/linux/man-pages/man8/blkid.8.html)
- [ArchWiki: EFI system partition](https://wiki.archlinux.org/title/EFI_system_partition)

BibTeX: `util-linux-lsblk`, `util-linux-blkid`, `arch-esp`.

## Pontos para implementação

- `lsblk` coleta informações de sysfs/udev e suporta saída estruturada. A
  saída padrão e as colunas padrão podem mudar; scripts devem escolher
  explicitamente as colunas e consumir JSON em vez de texto localizado.
- O relacionamento device/filesystem nem sempre é um-para-um. Um filesystem
  pode usar vários dispositivos ou ter múltiplos mounts; não reduza
  `MOUNTPOINTS` a um único caminho.
- `blkid` é complementar. Para usuários sem privilégios pode retornar dados
  cacheados e não verificados; resultados ambíguos podem ser ignorados.
  Explicite proveniência e incerteza no relatório.
- ESP é identificada por tipo de partição GPT, além de filesystem e contexto.
  Uma partição FAT isoladamente não comprova ser a ESP.
- UUID, PARTUUID, tipo de tabela, parent device, filesystem e mountpoint são
  dados distintos. Mantenha-os separados no modelo.

## Cautela

As páginas descrevem também ferramentas que escrevem em discos. Para a fase de
descoberta do projeto, restrinja os adaptadores a operações de leitura e teste
parsing com fixtures. Não rode sondagens destrutivas para "confirmar" um caso
ambíguo.
