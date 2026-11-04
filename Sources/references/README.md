# Biblioteca de referências técnicas

Esta biblioteca reúne documentação oficial e mantida por projetos upstream
relevante para o desenvolvimento do CachyOS Boot Rescue. Os documentos aqui
são resumos de consulta, não cópias integrais dos sites. Consulte sempre a
fonte original para obter instruções atuais e o contexto completo.

**Verificação dos links e conteúdo:** 2026-11-03.

## Organização

| Diretório | Assunto |
|---|---|
| [`bootloaders/`](bootloaders/) | Limine no CachyOS, GRUB, systemd-boot e formato compartilhado de entradas |
| [`storage/`](storage/) | Descoberta de dispositivos, ESP/UEFI e Btrfs |
| [`recovery/`](recovery/) | Chroot, initramfs e contexto específico do CachyOS |
| [`development/`](development/) | Processos externos Rust, testes Cargo e segurança Shell |

## Arquivos de índice

- [`catalog.json`](catalog.json): catálogo legível por máquina com URL, tópico,
  resumo, aplicabilidade e data de verificação.
- [`reading-map.csv`](reading-map.csv): mapa rápido de necessidades de
  desenvolvimento para referências e cautelas.
- [`bibliography.bib`](bibliography.bib): referências em BibTeX.
- [`bootloaders/README.md`](bootloaders/README.md): índice das referências
  específicas de bootloaders.

## Como aplicar ao projeto

1. Leia `AGENTS.md` e `Docs/ai/CURRENT_STATUS.md` antes de iniciar uma tarefa.
2. Use as notas para localizar a fonte upstream; não trate um resumo local como
   especificação mais recente que o documento original.
3. Separe evidência de proposta: as notas não afirmam que um recurso já esteja
   implementado ou validado no projeto.
4. Antes de qualquer operação de boot, armazenamento ou EFI, valide o sistema
   alvo, o firmware, a tabela de partições, os mounts e o bootloader instalado.
5. Os exemplos e procedimentos de instalação nos sites podem alterar ESP,
   NVRAM, kernels ou configuração. Não os execute em testes nem em uma máquina
   real sem escopo, revisão e autorização explícitos.

As fontes priorizam documentação upstream, manuais oficiais e a wiki do Arch
Linux para o foco inicial Arch/CachyOS. Distribuições e layouts além desse
foco permanecem sujeitos a validação específica.
