# START_HERE.md
# Boot-repair — Guia de entrada do projeto

## 1. Visão geral

Boot-repair é um projeto voltado ao diagnóstico e à recuperação de problemas de inicialização de sistemas Linux.

Seu foco inicial é oferecer ferramentas para trabalhar com sistemas Arch Linux e CachyOS, utilizando ambientes Live e Chroot.

A proposta é tornar os procedimentos de recuperação mais organizados, seguros e reproduzíveis.

O projeto deve ajudar a identificar problemas relacionados a:

- GRUB.
- systemd-boot.
- EFI System Partition.
- Kernels Linux.
- Initramfs.
- Configuração de inicialização.
- Montagem de partições.
- Configuração de `/etc/fstab`.
- Problemas de inicialização após atualizações.

## 2. Objetivo principal

Desenvolver uma solução que permita diagnosticar e executar procedimentos de recuperação de boot em sistemas Linux sem exigir que o usuário conheça todos os comandos envolvidos.

A ferramenta deve orientar o usuário desde a identificação do sistema instalado até a validação do procedimento de recuperação.

O processo deve ser dividido em etapas independentes.

## 3. Filosofia do projeto

O Boot-repair deve seguir cinco princípios:

1. Diagnosticar antes de reparar.
2. Identificar antes de modificar.
3. Preservar antes de substituir.
4. Validar antes de concluir.
5. Registrar antes de encerrar.

O projeto não deve funcionar como um conjunto de comandos executados indiscriminadamente.

Cada procedimento precisa considerar a configuração real da máquina.

## 4. Ambiente técnico

### Distribuições prioritárias

- CachyOS.
- Arch Linux.

### Tecnologias e componentes do sistema

- Linux.
- GNU Coreutils.
- Utilitários de particionamento.
- Utilitários de sistemas de arquivos.
- Chroot.
- GRUB.
- systemd-boot.
- UEFI.
- BIOS Legacy.
- GPT.
- MBR.
- systemd.
- mkinitcpio.

### Ferramentas de diagnóstico

- lsblk.
- blkid.
- findmnt.
- efibootmgr.
- mount.
- journalctl.
- pacman.
- grub-install.
- grub-mkconfig.
- bootctl.

A disponibilidade de cada ferramenta deve ser verificada antes da utilização.

## 5. Fluxo geral de funcionamento

O fluxo esperado é:

1. Inicializar o ambiente de recuperação.
2. Detectar o hardware e o firmware.
3. Identificar os dispositivos de armazenamento.
4. Identificar partições e sistemas de arquivos.
5. Identificar a instalação Linux.
6. Detectar o bootloader.
7. Montar o sistema de destino.
8. Preparar o ambiente Chroot.
9. Executar diagnósticos.
10. Apresentar opções de recuperação.
11. Executar o reparo autorizado.
12. Validar o resultado.
13. Gerar um relatório da operação.

Cada etapa deve possuir critérios de sucesso e condições de interrupção.

## 6. Arquitetura conceitual

A arquitetura deve separar as seguintes responsabilidades:

### Diagnóstico
Identificação do sistema instalado, discos, partições, firmware e bootloader.

### Gerenciamento de partições
Leitura das informações e identificação dos pontos de montagem necessários.

### Gerenciamento Chroot
Preparação, entrada, execução e encerramento do ambiente de recuperação.

### Recuperação do boot
Execução de procedimentos específicos para cada bootloader.

### Segurança
Validação de operações, confirmações, backups e controle de riscos.

### Logs
Registro das operações, comandos executados, resultados e falhas.

### Interface
Apresentação das informações e interação com o usuário.

Esses módulos representam uma arquitetura proposta. Sua implementação deve ser confirmada no código antes de serem considerados existentes.

## 7. Organização do desenvolvimento

O projeto foi planejado para ser desenvolvido em dez fases.

As fases devem ser implementadas progressivamente, evitando desenvolver funcionalidades dependentes antes de estabelecer suas bases.

A sequência detalhada de implementação deve ser consultada no planejamento de dez fases do projeto.

## 8. Documentos fundamentais

| Arquivo | Responsabilidade |
|---|---|
| `AGENTS.md` | Regras para agentes de IA |
| `START_HERE.md` | Contexto e orientação inicial |
| `CURRENT_STATUS.md` | Estado atual do desenvolvimento |

Todo agente deve consultar os três arquivos antes de modificar o projeto.

## 9. Orientações para começar

Ao iniciar uma sessão de desenvolvimento:

1. Leia `AGENTS.md`.
2. Leia este arquivo.
3. Leia `CURRENT_STATUS.md`.
4. Inspecione a estrutura do repositório.
5. Identifique a fase atual.
6. Consulte o planejamento correspondente.
7. Trabalhe exclusivamente no escopo autorizado daquela fase.

Não implemente funcionalidades futuras antecipadamente sem necessidade técnica justificada.

## 10. Critério de conclusão

Uma funcionalidade só deve ser considerada concluída quando:

- O código estiver implementado.
- Os testes pertinentes forem executados.
- Os erros conhecidos estiverem documentados.
- A documentação estiver atualizada.
- Os critérios de aceitação forem atendidos.

O objetivo é construir uma ferramenta de recuperação Linux confiável, segura e tecnicamente verificável.
