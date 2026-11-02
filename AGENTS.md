# AGENTS.md
# Regras obrigatórias para agentes de IA — Boot-repair

## 1. Objetivo

Este documento estabelece as regras que todos os agentes de IA devem seguir ao analisar, planejar, desenvolver, modificar, testar ou documentar o projeto Boot-repair.

Estas regras são obrigatórias e devem ser respeitadas independentemente do agente, modelo ou ferramenta utilizada.

O objetivo principal é garantir:
- Segurança na recuperação de sistemas Linux.
- Integridade dos dados do usuário.
- Previsibilidade das operações.
- Código modular e de fácil manutenção.
- Compatibilidade com diferentes configurações de inicialização.
- Documentação técnica atualizada.

## 2. Princípios fundamentais

Todo agente deve:

1. Analisar antes de modificar.
2. Nunca presumir a configuração do sistema.
3. Identificar a distribuição Linux e sua versão.
4. Identificar o tipo de firmware: UEFI ou BIOS Legacy.
5. Identificar o esquema de particionamento: GPT ou MBR.
6. Identificar o bootloader instalado.
7. Verificar os sistemas de arquivos e pontos de montagem.
8. Preservar os dados existentes.
9. Registrar operações importantes.
10. Validar cada etapa antes de prosseguir.

Nenhuma operação destrutiva deve ser executada sem justificativa técnica e autorização explícita.

## 3. Segurança operacional

### 3.1 Operações proibidas sem autorização

Nunca executar automaticamente:

- Formatação de partições.
- Exclusão de partições.
- Alterações na tabela de partições.
- Recriação de sistemas de arquivos.
- Exclusão de diretórios do sistema.
- Reinstalação completa do sistema operacional.
- Sobrescrita de configurações de boot.
- Alterações destrutivas no EFI System Partition (ESP).
- Remoção de kernels instalados.
- Exclusão de entradas EFI.
- Execução de comandos de escrita direta em dispositivos de armazenamento.

Exemplos de comandos que exigem atenção especial:

`mkfs`, `wipefs`, `dd`, `fdisk`, `parted`, `sgdisk`, `rm -rf` e operações de escrita com `efibootmgr`.

### 3.2 Operações de leitura

Sempre que possível, priorizar comandos de diagnóstico:

- `lsblk`
- `blkid`
- `findmnt`
- `fdisk -l`
- `parted -l`
- `efibootmgr -v`
- `ls /boot`
- `cat /etc/fstab`
- `uname -r`
- `chroot`
- `journalctl`

Comandos de leitura também devem ser avaliados quanto a privilégios e efeitos colaterais.

### 3.3 Backup

Antes de modificar configurações críticas:

1. Identificar o arquivo original.
2. Verificar se existe backup.
3. Criar uma cópia preservando permissões quando necessário.
4. Registrar o caminho do backup.
5. Validar a alteração realizada.

## 4. Regras para operações Chroot

O ambiente Chroot deve ser preparado corretamente antes de qualquer reparo.

O agente deve verificar:

- Partição raiz correta.
- Partição EFI, quando aplicável.
- Partições adicionais necessárias.
- Montagem de `/dev`.
- Montagem de `/proc`.
- Montagem de `/sys`.
- Montagem de `/run`.
- Disponibilidade de ferramentas necessárias.
- Acesso à rede, quando necessário.

A estrutura de montagem deve respeitar a configuração real do sistema.

Não presumir que todos os sistemas utilizam `/boot/efi`. A ESP pode estar montada em `/efi`, `/boot` ou outro ponto.

Nunca executar reparos em um Chroot cuja partição raiz não tenha sido identificada com segurança.

## 5. Bootloaders

O projeto deve considerar diferentes mecanismos de inicialização.

### GRUB
Verificar:
- Instalação do GRUB.
- Arquivos de configuração.
- Localização dos módulos.
- Configuração EFI ou BIOS.
- Detecção de kernels.
- Entradas de inicialização.

### systemd-boot
Verificar:
- Presença do systemd-boot.
- Localização da ESP ou XBOOTLDR.
- Arquivos de entrada.
- Arquivos de kernel e initramfs.
- Configuração do loader.

### Outros mecanismos

Não presumir que GRUB ou systemd-boot sejam utilizados em todos os sistemas.

O diagnóstico deve identificar o mecanismo existente antes de propor uma intervenção.

## 6. Compatibilidade com distribuições

O foco inicial do projeto é o ecossistema Arch Linux e CachyOS.

Entretanto, o desenvolvimento deve considerar a possibilidade de expansão para outras distribuições.

Não utilizar comandos específicos de uma distribuição sem verificar sua disponibilidade.

Exemplos:

- Arch Linux / CachyOS: `pacman`, `mkinitcpio`.
- Fedora: `dnf`, `dracut`.
- Debian / Ubuntu: `apt`, `update-initramfs`.

Não misturar procedimentos de distribuições diferentes sem justificativa.

## 7. Arquitetura e qualidade do código

Todo código novo deve:

- Ter responsabilidade bem definida.
- Ser modular.
- Evitar duplicação.
- Possuir tratamento de erros.
- Validar entradas.
- Evitar dependências desnecessárias.
- Ser legível e documentado.
- Separar diagnóstico de execução.
- Separar interface de lógica operacional.

Não criar funcionalidades grandes em arquivos monolíticos.

Não introduzir abstrações desnecessárias.

## 8. Execução de comandos

Toda execução de comandos deve considerar:

- Código de retorno.
- Saída padrão.
- Saída de erro.
- Tempo de execução.
- Necessidade de privilégios.
- Possíveis efeitos colaterais.

Não utilizar `sudo` indiscriminadamente.

Não ocultar erros com redirecionamentos que impeçam o diagnóstico.

Comandos críticos devem ser apresentados ao usuário antes da execução.

## 9. Testes

Toda funcionalidade nova deve possuir testes compatíveis com sua finalidade.

Priorizar:

- Testes unitários.
- Testes de integração.
- Simulação de cenários.
- Testes com dispositivos fictícios.
- Testes de falhas.
- Validação de códigos de retorno.

Nunca utilizar o disco real do usuário como ambiente de testes automatizados.

Sempre que possível, simular dispositivos e partições.

## 10. Interface e experiência do usuário

A interface deve:

- Apresentar informações compreensíveis.
- Mostrar o progresso das operações.
- Informar erros com detalhes úteis.
- Solicitar confirmação antes de operações críticas.
- Diferenciar diagnóstico de reparo.
- Exibir quais alterações serão realizadas.

Evitar mensagens genéricas como "Erro ao reparar".

Preferir mensagens que identifiquem a etapa e a causa provável.

## 11. Documentação

Ao adicionar ou modificar funcionalidades:

- Atualizar a documentação correspondente.
- Atualizar `CURRENT_STATUS.md`.
- Registrar decisões arquiteturais relevantes.
- Documentar dependências.
- Documentar limitações conhecidas.

Não afirmar que uma funcionalidade está implementada sem verificar o código.

## 12. Fluxo obrigatório de trabalho

Antes de implementar qualquer funcionalidade:

1. Ler este documento.
2. Ler `START_HERE.md`.
3. Ler `CURRENT_STATUS.md`.
4. Inspecionar a estrutura existente.
5. Identificar dependências.
6. Compreender o comportamento esperado.
7. Elaborar um plano de implementação.
8. Implementar em etapas pequenas.
9. Executar testes.
10. Atualizar a documentação.

## 13. Regra de integridade

A segurança dos dados do usuário tem prioridade sobre a conclusão de qualquer reparo.

Se houver incerteza sobre uma operação, o agente deve interromper a execução, explicar o risco e solicitar informações adicionais.

Nunca inventar resultados de testes, funcionalidades, configurações ou diagnósticos.

**Regra final: nenhum agente deve sacrificar a integridade do sistema para concluir uma tarefa mais rapidamente.**
