# Limine

## Resumo

Limine é um bootloader e boot manager portátil e multiprotocolo. O upstream o
descreve como implementação de referência do protocolo Limine; também carrega
Linux, Multiboot 1/2 e pode encadear executáveis EFI ou outros bootloaders.

Limine é uma referência central para este projeto: a configuração atual do
instalador CachyOS o seleciona como padrão, embora o usuário possa escolher
outro bootloader. Consulte [`CACHYOS_LIMINE.md`](CACHYOS_LIMINE.md) para as
evidências e limites dessa afirmação.

## Fontes primárias

- [ArchWiki: Limine](https://wiki.archlinux.org/title/Limine) — instalação e
  configuração no Arch, diferenças UEFI/BIOS e exemplos.
- [Upstream: README](https://github.com/Limine-Bootloader/Limine) — propósito,
  arquiteturas, protocolos e filesystems suportados.
- [Upstream: CONFIG.md](https://github.com/Limine-Bootloader/Limine/blob/trunk/CONFIG.md) —
  descoberta e sintaxe de `limine.conf`.
- [Upstream: USAGE.md](https://github.com/Limine-Bootloader/Limine/blob/trunk/USAGE.md) —
  operação UEFI e políticas de Secure Boot/Measured Boot.
- [Upstream: FAQ.md](https://github.com/Limine-Bootloader/Limine/blob/trunk/FAQ.md) —
  escolhas de design, filesystems e verificação de configuração.
- [Upstream: INSTALL.md](https://github.com/Limine-Bootloader/Limine/blob/trunk/INSTALL.md) —
  build, checks com QEMU e instalação dos arquivos de software.
- [Pacote Limine do CachyOS](https://github.com/CachyOS/CachyOS-PKGBUILDS/tree/master/limine) —
  definição do pacote distribuído pela equipe CachyOS.

As páginas upstream de GitHub com `trunk` são documentos vivos e podem mudar.
Elas foram consultadas em 2026-11-03. Para o snapshot CachyOS usado como
evidência do padrão atual, a documentação do pacote usa o commit fixo
registrado em [`CACHYOS_LIMINE.md`](CACHYOS_LIMINE.md).

## Modos de implantação

Limine possui caminhos de implantação diferentes; o diagnóstico não deve
misturá-los:

| Ambiente | Evidências e requisitos |
|---|---|
| UEFI | Aplicativo EFI, normalmente `BOOTX64.EFI` em x64 (ou nome adequado à arquitetura), armazenado em uma ESP FAT. Pode ser inicializado por entrada NVRAM ou pelo caminho removível/fallback. |
| BIOS com MBR | Código de estágio inicial instalado no disco e arquivo `limine-bios.sys` em filesystem que Limine consiga ler. |
| BIOS com GPT | Além dos arquivos de boot, requer partição BIOS boot para o código inicial. A implantação depende do disco e da partição corretos. |

O upstream informa suporte a FAT12/16/32 e ISO9660 para acesso aos seus
arquivos. Isso não significa que a raiz Linux precise ser FAT: significa que
kernel, initramfs e demais recursos lidos diretamente pelo bootloader devem
estar num volume que ele consiga ler. O kernel é quem acessa o filesystem raiz
durante o boot.

Um arquivo EFI do Limine pode coexistir com a instalação BIOS, mas isso não
torna a configuração automaticamente híbrida nem demonstra que ambos os
caminhos foram testados.

## `limine.conf`

O arquivo contém opções globais e entradas de menu. A sintaxe upstream usa
linhas `nome: valor`; entradas começam por `/` e as opções específicas de uma
entrada seguem o título até a próxima entrada. Entre as informações
importantes para diagnosticar uma entrada Linux estão:

- protocolo (`protocol: linux`);
- caminho do kernel (`path:` ou `kernel_path:`);
- um ou mais módulos/initramfs (`module_path:`);
- linha de comando do kernel (`cmdline:`);
- caminhos e identificadores do volume onde os arquivos estão.

Os caminhos podem usar a partição que contém a configuração (`boot():/`) ou
identificação explícita do volume, como UUID/PARTUUID, conforme o formato e o
contexto. Um `limine.conf` na ESP pode apontar para arquivos em outro volume;
por isso, caminhos da configuração precisam ser correlacionados ao inventário
real de devices e mounts.

### Localização não trivial

Limine procura a configuração no contexto do dispositivo de boot. Em UEFI,
primeiro considera o volume do executável EFI iniciado. A localização do
executável e os diretórios candidatos de `limine.conf` afetam qual configuração
vence. Uma cópia antiga numa posição de prioridade maior pode ocultar a
configuração que alguém espera estar ativa.

Consequências para o Boot Rescue:

- não assumir que `/boot/limine/limine.conf` é o único caminho possível;
- não concluir que a configuração encontrada é a efetivamente consumida sem
  considerar a localização e precedência do executável/volumes;
- registrar cada arquivo e volume encontrado e sinalizar múltiplos candidatos;
- não editar, renomear ou remover entradas durante uma etapa de diagnóstico.

Para o detalhe da ordem de busca e a sintaxe completa, consulte o
[documento upstream de configuração](https://github.com/Limine-Bootloader/Limine/blob/trunk/CONFIG.md).

## Integrações do CachyOS

O instalador CachyOS atual pode gerar e manter configurações além dos arquivos
do Limine upstream. Em particular, o código do Calamares integra seleção de
kernel e parâmetros de boot, e o conjunto de pacotes pode incluir ferramentas
de entradas e sincronização de snapshots. Ver
[`CACHYOS_LIMINE.md`](CACHYOS_LIMINE.md).

Não trate um arquivo de configuração específico do CachyOS como sintaxe nativa
do `limine.conf`. Registre sua origem e deixe claro se ele é consumido por uma
ferramenta da distribuição que gera ou atualiza a configuração usada pelo
bootloader.

## Secure Boot e integridade

Segundo o upstream, a inicialização do binário EFI assinado, por si só, não
habilita toda a política de verificação de Limine. A configuração requer
inscrever um hash da configuração no executável EFI. Com Secure Boot ativo e
hash inscrito, Limine verifica a configuração e exige hashes nos recursos
carregados, com as exceções descritas pelo upstream; o editor de configuração
é desabilitado. Sem o hash inscrito, a política adicional descrita não é
aplicada, mesmo que o binário esteja assinado.

Portanto:

- estado Secure Boot ativo não comprova que `limine.conf`, kernel e initramfs
  estão verificados pelo Limine;
- presença de assinatura EFI não comprova que a cadeia inteira está protegida;
- não modificar, assinar ou inscrever hashes automaticamente;
- registrar como desconhecido quando a ferramenta não puder confirmar a
  cadeia de confiança.

O comportamento é específico à versão instalada; consulte
[USAGE.md upstream](https://github.com/Limine-Bootloader/Limine/blob/trunk/USAGE.md)
e não extrapole para outra versão.

## Matriz de diagnóstico somente leitura

| Pergunta | Evidências possíveis | O que não se deve inferir isoladamente |
|---|---|---|
| Limine está instalado no sistema? | Pacotes/arquivos do sistema instalado | Que Limine foi usado no último boot |
| Existe uma implantação EFI? | Arquivos EFI candidatos na ESP, mounts e caminhos | Que o firmware iniciou aquele arquivo |
| Existe uma entrada de firmware? | Saída de leitura de `efibootmgr -v`, quando disponível | Que ela foi usada ou que seu caminho ainda existe |
| Limine iniciou o sistema atual? | Dados de boot corrente disponíveis, caminho do loader quando suportado e evidências coerentes | Nome de diretório EFI ou `limine.conf` isolado |
| Há uma entrada Linux utilizável? | Configuração, kernel/initramfs referenciados, filesystem acessível e argumentos de root | Que a entrada necessariamente inicializará ou que os argumentos estão corretos |
| Há suporte a snapshots Btrfs? | Pacotes e configuração de integração detectados | Que todo snapshot é inicializável ou que foi validado |

As fontes dessas evidências podem não existir ou não estar acessíveis em um
ambiente Live. Nesse caso, reporte a limitação em vez de substituir a evidência
ausente por uma suposição.

## Testes e limites

- Teste a descoberta e análise de configuração usando arquivos temporários e
  fixtures sintéticas; nunca consulte nem modifique a ESP do desenvolvedor.
- Inclua casos com múltiplos ESPs, vários `limine.conf`, entradas incompletas,
  kernels/initramfs ausentes, caminhos por UUID e boot BIOS/UEFI.
- Não use `limine bios-install`, `efibootmgr` em modo de escrita, `cp` para ESP
  ou qualquer rotina de instalação nos testes automatizados.
- A documentação do upstream lista QEMU e checks automatizados do Limine; isso
  é contexto para uma futura validação isolada, não evidência de que o projeto
  Boot Rescue tenha sido validado em firmware real.
