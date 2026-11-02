# CachyOS Boot Rescue — Plano de documentação orientada a agentes de IA

**Status:** proposta de documentação inicial  
**Projeto:** CachyOS Boot Rescue  
**Público principal:** agentes de programação (LLMs), mantenedores e contribuidores  
**Idioma:** documentação normativa em inglês; notas de planejamento podem começar em português  

> Este documento define como organizar a documentação para que um agente consiga entender o projeto, localizar contratos técnicos, implementar uma tarefa e validar o resultado sem precisar fazer perguntas básicas ao mantenedor.

---

## 1. Objetivo da documentação

A documentação não deve ser apenas explicativa. Ela deve funcionar como uma base de contexto técnico e operacional para agentes de IA.

Ao receber uma tarefa, um agente deve conseguir descobrir:

1. O que o projeto faz e o que explicitamente não faz.
2. Quais decisões já foram tomadas e quais ainda são propostas.
3. Como o código está organizado e onde cada responsabilidade pertence.
4. Quais estruturas de dados e contratos os módulos devem respeitar.
5. Como detectar discos, partições, ESPs, sistemas de arquivos e instalações Linux.
6. Como montar uma instalação e entrar/sair de um chroot com segurança.
7. Como diagnosticar e reparar GRUB e Limine sem apagar o caminho de boot anterior.
8. Como executar comandos privilegiados, registrar resultados e tratar falhas.
9. Como criar backups, validar alterações e tentar rollback.
10. Como executar testes, gerar o pacote Pacman e verificar a documentação atualizada.

**Resultado esperado:** uma tarefa deve poder ser iniciada com a leitura de um pequeno conjunto de arquivos de contexto, e o agente deve consultar os documentos especializados conforme a área alterada.

## 2. Princípios de documentação para IA

### 2.1 Uma fonte de verdade por assunto

Não repetir a mesma regra em cinco documentos. Cada assunto possui um documento normativo principal; outros documentos apontam para ele.

Exemplos:

- Política de segurança: `docs/safety/SECURITY_MODEL.md`.
- Contrato do módulo Storage: `docs/modules/storage.md`.
- Decisões aceitas: `docs/decisions/`.
- Estado real de implementação: `docs/ai/CURRENT_STATUS.md`.
- Matriz de suporte: `schemas/support-matrix.schema.json` e sua instância documentada em `docs/testing/HARDWARE_MATRIX.md`.

### 2.2 Diferenciar intenção, implementação e validação

Toda funcionalidade deve indicar seu estado. Usar estes rótulos de forma consistente:

- `PLANNED`: planejada, ainda sem implementação.
- `DESIGNED`: contrato/arquitetura definidos, código ainda não concluído.
- `IMPLEMENTED`: existe no código, mas pode não ter testes suficientes.
- `UNIT-TESTED`: possui testes unitários relevantes.
- `VM-TESTED`: validada em máquina virtual.
- `HARDWARE-TESTED`: validada em hardware real; registrar exatamente a configuração.
- `UNSUPPORTED`: não suportada deliberadamente.
- `UNKNOWN`: ainda não investigada.

Nunca descrever `IMPLEMENTED` como `SUPPORTED` sem uma evidência de teste apropriada. Toda alegação de compatibilidade deve indicar ambiente, data e nível de validação.

### 2.3 Escrever contratos, não apenas descrições

Cada módulo precisa declarar:

- Responsabilidade.
- Entradas e saídas.
- Tipos/estruturas utilizados.
- Pré-condições.
- Pós-condições.
- Efeitos colaterais.
- Comandos externos que pode executar.
- Erros que pode retornar.
- Recursos que não pode modificar.
- Testes obrigatórios.

### 2.4 Tornar as regras de segurança explícitas

Regras de segurança devem aparecer em um documento normativo e ser referenciadas no `AGENTS.md`. Não depender de inferência do agente.

### 2.5 Documentação próxima ao código

Documentação arquitetural fica em `docs/`; detalhes de tipos e funções ficam em comentários Rust (`///`, `//!`) e exemplos de uso. Mudanças de comportamento exigem atualização dos documentos afetados na mesma alteração de código.

### 2.6 Evitar contexto excessivo

O contexto inicial deve ser curto. Os arquivos especializados só são carregados quando a tarefa os exige. Não criar um único `PROJECT_CONTEXT.md` gigantesco com todos os detalhes.

---

## 3. Hierarquia de contexto para agentes

### Camada 0 — Instruções do repositório

`/AGENTS.md` é o arquivo de entrada para agentes que reconhecem instruções por repositório. Ele deve ser curto e encaminhar para os documentos corretos.

Criar arquivos-ponte para ferramentas que usam convenções próprias, sem duplicar as regras:

- `.github/copilot-instructions.md`
- `CLAUDE.md` (opcional, caso Claude Code seja usado)
- `.cursor/rules/project-context.mdc`

Cada arquivo-ponte deve dizer para ler `/AGENTS.md` e `docs/ai/START_HERE.md`, mantendo uma única fonte de verdade.

### Camada 1 — Contexto rápido obrigatório

`docs/ai/START_HERE.md` deve levar poucos minutos para ler e conter:

- Resumo do produto.
- Plataforma/ambiente-alvo.
- Stack tecnológica.
- Funcionalidades do MVP.
- Arquitetura em alto nível.
- Invariantes de segurança.
- Comandos de build/test.
- Links para os documentos especializados.

`docs/ai/PROJECT_CONTEXT.md` registra as decisões atualmente adotadas, sem misturar propostas com decisões confirmadas.

### Camada 2 — Contexto por tarefa

O `docs/ai/DOCUMENTATION_MAP.md` mapeia tarefas para documentos. Exemplo:

| Tarefa | Documentos obrigatórios |
|---|---|
| Detectar discos/partições | `docs/modules/discovery.md`, `docs/domain/FILESYSTEMS.md`, `docs/testing/TEST_STRATEGY.md` |
| Montar Btrfs | `docs/modules/storage.md`, `docs/domain/BTRFS.md`, `docs/domain/FSTAB_CRYPTTAB.md`, `docs/safety/SECURITY_MODEL.md` |
| Entrar no chroot | `docs/modules/chroot.md`, `docs/domain/CHROOT.md`, `docs/architecture/PRIVILEGE_MODEL.md` |
| Reparar GRUB | `docs/modules/diagnostics.md`, `docs/domain/BOOTLOADERS/GRUB.md`, `docs/safety/DESTRUCTIVE_OPERATIONS.md`, `docs/safety/ROLLBACK.md` |
| Reparar Limine | `docs/modules/repair_engine.md`, `docs/domain/BOOTLOADERS/LIMINE.md`, `docs/domain/EFI_PARTITIONS.md`, `docs/safety/ROLLBACK.md` |
| Alterar logs | `docs/modules/logging.md`, `docs/modules/reporting.md`, `docs/safety/PRIVACY.md` |
| Alterar pacote | `docs/development/PACKAGING.md`, `docs/development/DEPENDENCIES.md` |
| Adicionar testes | `docs/testing/TEST_STRATEGY.md`, `docs/testing/VM_TESTING.md`, `docs/testing/HARDWARE_MATRIX.md` |

### Camada 3 — Histórico e evidências

- ADRs explicam por que uma decisão foi tomada.
- Changelog registra alterações por versão.
- Relatórios de teste registram evidências e limitações.
- Issues e relatórios de bugs documentam falhas observadas, sem substituir contratos normativos.

---

## 4. Conteúdo esperado dos documentos

### 4.1 `AGENTS.md` — instruções curtas e obrigatórias

Deve conter:

1. Ler `docs/ai/START_HERE.md` antes de editar código.
2. Ler `docs/ai/DOCUMENTATION_MAP.md` para localizar documentos da tarefa.
3. Ler os ADRs relevantes antes de alterar arquitetura ou dependências.
4. Nunca presumir que uma função planejada já existe; conferir `CURRENT_STATUS.md` e o código.
5. Não executar operações reais de reparo em discos do desenvolvedor durante testes.
6. Não introduzir chamadas destrutivas sem plano explícito, confirmação e testes.
7. Não usar `shell=true`, `sh -c`, `bash -c`, `eval` ou concatenação de comandos com dados externos; usar `Command` com argumentos separados.
8. Não gravar segredos, senhas, chaves LUKS ou tokens nos logs.
9. Preservar entradas/arquivos do Windows e de outros bootloaders.
10. Adicionar/atualizar testes e documentação na mesma alteração.
11. Rodar format, lint e testes apropriados antes de concluir.
12. Informar arquivos alterados, testes executados, limitações e riscos remanescentes.

O `AGENTS.md` não deve conter longas explicações sobre GRUB, Btrfs ou UEFI; deve apontar para a documentação especializada.

### 4.2 `docs/ai/START_HERE.md` — briefing de primeira leitura

Seções obrigatórias:

- O que é o CachyOS Boot Rescue.
- Problema que resolve.
- O que não é: não é um instalador de sistema, particionador, migrador de bootloader no MVP nem substituto de backup.
- Público-alvo e modo de operação (Live USB/ambiente de recuperação).
- Stack: Rust, Ratatui/Crossterm, pacote Arch/Pacman.
- Bootloaders iniciais: GRUB e Limine.
- Sistemas/layouts suportados e não suportados no MVP.
- Arquitetura resumida.
- Regras de segurança mais importantes.
- Comandos locais de build/test.
- Links de navegação para o restante da documentação.

### 4.3 `docs/ai/PROJECT_CONTEXT.md` — decisões e pressupostos atuais

Registrar explicitamente:

- Nome: CachyOS Boot Rescue.
- Projeto independente, não fork do Cachy-Chroot.
- Linguagem: Rust.
- Interface: TUI com Ratatui/Crossterm.
- Distribuição principal: pacote `.pkg.tar.zst` instalável com Pacman.
- Distribuição futura: AUR, após estabilizar PKGBUILD e testes.
- Público inicial: CachyOS; compatibilidade Arch Linux como objetivo.
- MVP: detecção, montagem, chroot, diagnóstico e reparo de GRUB/Limine, initramfs, logs e exportação de relatório.
- Logs discretos por padrão; exportação manual e consentida.
- Migração entre bootloaders não pertence ao MVP.
- Secure Boot, BIOS/Legacy, RAID/LVM e topologias complexas devem ser marcados como fora do escopo até terem projeto/testes específicos.

Marcar explicitamente os pontos ainda sujeitos a decisão do mantenedor, em vez de apresentá-los como fatos:

- Versão mínima do Rust (MSRV).
- Licença do projeto após auditoria de reutilização de código.
- Limite inicial de sistemas de arquivos.
- Política de suporte a LUKS no MVP.
- Política para múltiplas ESPs.
- Política de persistência dos logs no Live USB.

Na ausência de decisão, o agente deve escolher a opção mais conservadora, registrar uma ADR proposta e não executar uma operação destrutiva.

### 4.4 `docs/ai/CURRENT_STATUS.md` — estado real do software

Tabela por recurso com as colunas:

- Recurso.
- Estado (`PLANNED`, `DESIGNED`, `IMPLEMENTED`, etc.).
- Módulo/caminhos do código.
- Testes existentes.
- Limitações conhecidas.
- Data/commit da última verificação.

Este documento é especialmente importante para evitar que o agente confunda o plano futuro com funcionalidades já implementadas.

### 4.5 `docs/ai/TASK_WORKFLOW.md` — procedimento de trabalho do agente

Fluxo obrigatório:

1. Ler `AGENTS.md` e `START_HERE.md`.
2. Identificar a área afetada em `DOCUMENTATION_MAP.md`.
3. Ler os contratos dos módulos e ADRs relevantes.
4. Inspecionar o código atual e testes antes de propor mudanças.
5. Escrever um plano curto de alteração.
6. Implementar a menor mudança segura possível.
7. Criar/atualizar testes.
8. Atualizar documentação e `CURRENT_STATUS.md`.
9. Executar verificações locais.
10. Entregar resumo, testes, riscos e itens não cobertos.

O agente não deve perguntar sobre decisões já registradas. Se faltar um detalhe não destrutivo, deve adotar o padrão documentado. Se uma ambiguidade puder modificar discos, EFI, NVRAM ou dados do usuário, deve parar antes dessa operação e solicitar confirmação explícita.

---

## 5. Documentação de arquitetura

### `docs/architecture/OVERVIEW.md`

Descrever camadas, dependências permitidas e direção das chamadas. Recomenda-se a seguinte organização lógica:

```text
TUI / CLI
   │
   ▼
Application Orchestrator
   ├── Discovery
   ├── Diagnostic Engine
   ├── Repair Planner
   └── Session Manager
          │
          ├── Storage Manager
          ├── Chroot Manager
          ├── Bootloader Adapters
          ├── Backup / Rollback
          ├── Command Executor
          └── Logging / Reporting
```

A UI não deve executar comandos de sistema diretamente. Ela chama casos de uso da camada de aplicação. Os adaptadores não devem depender da TUI.

### `docs/architecture/MODULES.md`

Definir cada módulo e as dependências permitidas. Exemplo de contratos esperados:

- `discovery`: encontra dispositivos, partições, sistemas de arquivos, instalações e ESPs; não monta nem escreve.
- `storage`: monta/desmonta volumes conforme plano aprovado; não escolhe o bootloader.
- `chroot`: prepara o ambiente-alvo e fornece execução contextualizada; garante desmontagem em saída normal e tratamento de falha.
- `diagnostics`: observa e produz achados; não modifica o sistema.
- `repair-planner`: converte achados em plano legível; não executa.
- `repair-engine`: executa um plano confirmado através de adaptadores.
- `bootloader-adapters`: implementam comportamento específico de cada bootloader.
- `backup`: guarda estado anterior e manifesto de propriedade.
- `rollback`: reverte somente alterações próprias e verificáveis.
- `command-executor`: executa binários com argumentos separados, timeout, captura de saída e resultado estruturado.
- `logging`: registra eventos técnicos sanitizados.
- `reporting`: reúne dados, remove informações sensíveis e exporta arquivo de diagnóstico.

### `docs/architecture/DATA_FLOW.md`

Documentar o fluxo completo:

`Start → Privilege check → Discovery → Select installation → Mount plan → Mount → Inspect → Diagnose → Repair plan → Backup → User confirmation → Apply → Post-validation → Report → Cleanup`.

Incluir diagramas Mermaid e identificar em quais etapas o programa pode escrever em disco ou NVRAM.

### `docs/architecture/PROCESS_LIFECYCLE.md`

Documentar ciclo da sessão, estado temporário, sinais/encerramento, desmontagem e recuperação após interrupção. Diferenciar encerramento normal, erro de comando, cancelamento do usuário e encerramento abrupto.

### `docs/architecture/ERROR_HANDLING.md`

Definir enumerações de erro, contexto, severidade e códigos de saída. Não usar mensagens textuais como única forma de decidir o fluxo do programa.

### `docs/architecture/PRIVILEGE_MODEL.md`

Definir quando e por que o programa exige root, quais comandos são privilegiados e como evita ampliar privilégios desnecessariamente. O MVP pode exigir root desde o início, mas deve validar isso e explicar claramente ao usuário.

---

## 6. Documentação do domínio Linux

Esses documentos evitam que agentes implementem suposições perigosas ou incompatíveis com instalações reais.

### `UEFI.md` e `EFI_PARTITIONS.md`

Documentar:

- Diferença entre EFI System Partition (ESP), diretório de montagem e entrada NVRAM.
- ESP montada em `/boot`, `/efi` ou `/boot/efi`.
- Diferença entre arquivos EFI e entradas de firmware.
- `BootOrder`, `BootNext`, `BootCurrent`.
- Várias ESPs e critérios para detectar ambiguidade.
- Preservação de `EFI/Microsoft` e diretórios de fornecedores.
- Nunca assumir que `/dev/sda` é o disco principal; usar UUID, PARTUUID e propriedades estáveis.

### `FILESYSTEMS.md`, `BTRFS.md` e `FSTAB_CRYPTTAB.md`

Documentar:

- Detecção de filesystem sem depender apenas do nome do dispositivo.
- Btrfs: subvolumes, subvolume padrão, nested subvolumes e opções de montagem.
- Diferença entre montar o top-level subvolume (`subvolid=5`) e montar a raiz instalada.
- Parsing de `/etc/fstab` e `/etc/crypttab` sem executar/sourcing esses arquivos como shell.
- UUID, PARTUUID, LABEL, caminhos e opções relevantes.
- Montagens bind de `/dev`, `/proc`, `/sys`, `/run` necessárias ao chroot.
- Como detectar entradas inconsistentes sem “corrigir” automaticamente.
- Política de read-only discovery e read-write somente após confirmação.

### `CHROOT.md`

Documentar:

- Como preparar root e montagens adicionais.
- Ordem de montagem e desmontagem.
- Uso de `arch-chroot` ou mecanismo próprio, com decisão registrada.
- Variáveis de ambiente que podem ser propagadas.
- Como executar comandos no sistema-alvo sem interpolação de shell.
- Limpeza após falhas e prevenção de montagens duplicadas.

### `INITRAMFS.md`

Documentar quando regenerar initramfs, como detectar kernels instalados, caminhos de imagens, uso de `mkinitcpio`, interpretação de falhas e pré-condições. Não regenerar automaticamente se a raiz/chroot não estiver corretamente montada.

### `BOOTLOADERS/GRUB.md` e `BOOTLOADERS/LIMINE.md`

Cada documento deve ter a mesma estrutura:

1. Como detectar instalação existente.
2. Arquivos e diretórios relevantes.
3. Configurações e fontes de verdade.
4. Layouts de ESP suportados.
5. Pacotes e comandos necessários.
6. Procedimento de diagnóstico somente leitura.
7. Plano de reparo.
8. Arquivos que podem ser criados/modificados.
9. Arquivos que nunca podem ser apagados sem autorização específica.
10. Validação pós-reparo.
11. Backup e rollback.
12. Casos de teste e limitações conhecidas.

Não copiar comandos de um tutorial sem documentar as pré-condições e versões testadas.

---

## 7. Segurança, reparos e transações

### `docs/safety/SECURITY_MODEL.md`

Deve ser uma especificação normativa. Invariantes mínimos:

- Descoberta e diagnóstico são somente leitura.
- Toda escrita deriva de um `RepairPlan` explícito.
- O plano contém dispositivo, partição, caminhos, comandos, efeitos esperados e riscos.
- O usuário vê o plano antes de confirmar.
- Nenhuma partição será formatada pelo MVP.
- Não apagar nem sobrescrever arquivos do Windows ou de bootloaders não gerenciados.
- Não modificar `BootOrder`/`BootNext` sem caso de uso documentado e confirmação.
- Não escolher ESP arbitrariamente quando houver mais de uma candidata.
- Não modificar `/etc/fstab` em reparos comuns.
- Falhas devem encerrar o fluxo de forma conservadora.

### `docs/safety/DESTRUCTIVE_OPERATIONS.md`

Definir classificação:

- Classe 0: leitura.
- Classe 1: criação de relatório/backup fora do sistema-alvo.
- Classe 2: criação/alteração de arquivos de configuração do bootloader.
- Classe 3: gravação de binários EFI ou alteração de NVRAM.
- Classe 4: operações potencialmente destrutivas (formatação, remoção de arquivos ou limpeza de entradas).

Classe 4 deve ficar fora do MVP. Classes 2 e 3 exigem plano, backup e confirmação inequívoca.

### `docs/safety/BACKUP_POLICY.md` e `ROLLBACK.md`

Especificar:

- O que entra no backup.
- Localização, permissões e retenção.
- Manifesto de arquivos, hashes, proprietários e timestamps relevantes.
- Estado NVRAM registrado separadamente.
- Limitações de restaurar variáveis de firmware.
- Regras para restaurar somente arquivos cuja propriedade foi registrada pelo Rescue.
- Como lidar com backup incompleto/corrompido.
- Como o sistema registra o estado da transação após queda de energia.

Um backup de arquivos da ESP não deve ser apresentado como garantia de recuperação de toda a configuração do firmware.

### Modelo de transação recomendado

```text
DISCOVERED
  → INSPECTED
  → PLAN_READY
  → BACKUP_CREATED
  → AWAITING_CONFIRMATION
  → APPLYING
  → POST_VALIDATION
  → COMPLETED

Falhas possíveis:
  → ABORTED_BEFORE_WRITE
  → APPLY_FAILED
  → ROLLBACK_REQUIRED
  → ROLLBACK_COMPLETED
  → RECOVERY_MANUAL_REQUIRED
```

O código deve persistir um journal de transação antes da primeira escrita. O estado precisa permitir determinar o que foi tentado e o que foi realmente concluído.

---

## 8. Logging e relatórios de diagnóstico

### `docs/modules/logging.md`

Definir eventos estruturados com:

- timestamp;
- session_id;
- transaction_id, quando existir;
- módulo e operação;
- nível (`ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE`);
- identificador estável do erro;
- duração;
- código de saída de processo externo;
- referência aos artefatos capturados.

A interface deve ser discreta: sem despejar logs técnicos no fluxo normal. O usuário vê mensagens simples e tem uma tela secundária de diagnóstico.

### `docs/modules/reporting.md` e `docs/safety/PRIVACY.md`

O relatório exportado deve conter apenas dados necessários para diagnosticar:

- versão do Rescue e versão do sistema Live;
- arquitetura e modo de boot (UEFI/BIOS);
- resumo de discos/partições com identificadores sanitizados;
- layout de montagem;
- bootloader detectado;
- comandos executados, argumentos sanitizados, stdout/stderr sanitizados e exit code;
- sequência de eventos e erro principal;
- resultados de validação;
- resumo do backup, sem incluir chaves ou conteúdo secreto.

Nunca exportar:

- senha de usuário ou de LUKS;
- tokens, cookies ou credenciais;
- conteúdo de chaves privadas;
- conteúdo de `/etc/shadow`;
- variáveis de ambiente completas;
- arquivos de configuração inteiros sem revisão de sensibilidade.

A exportação é manual e exige consentimento. Não enviar logs automaticamente para GitHub ou qualquer serviço remoto.

No Live USB, definir claramente que os logs podem ser voláteis e devem ser exportados antes do reboot. O local de armazenamento temporário e o local de exportação devem ser diferentes. Usar permissões restritas (`0700` para diretórios e `0600` para arquivos, quando aplicável).

### `schemas/diagnostic-report.schema.json`

Criar schema versionado para o relatório, permitindo que ferramentas futuras validem e processem relatórios sem depender de texto livre.

---

## 9. Contratos e schemas de dados

A documentação deve descrever tipos importantes antes ou junto à implementação.

Tipos conceituais sugeridos:

- `BlockDevice`: dispositivo de bloco e propriedades estáveis.
- `Partition`: partição, filesystem, UUID/PARTUUID, flags e tamanho.
- `LinuxInstallation`: raiz, distribuição identificada, fstab, kernels e montagens relevantes.
- `EspCandidate`: partição ESP, ponto de montagem e confiança da detecção.
- `MountPlan`: plano de montagem com origem, destino, opções e modo read-only/read-write.
- `BootloaderKind`: GRUB, Limine e futuros adaptadores.
- `DiagnosticFinding`: código, severidade, descrição e evidência.
- `RepairPlan`: ações ordenadas, efeitos, riscos, backup necessário e pré-condições.
- `CommandSpec`: programa, argumentos separados, diretório de trabalho, ambiente permitido e timeout.
- `CommandResult`: exit code, stdout/stderr capturados, duração e status.
- `BackupManifest`: caminhos, hashes, origem, timestamp e versão do schema.
- `SessionEvent`: evento de log estruturado.
- `DiagnosticReport`: pacote de dados exportável.

Criar schemas JSON versionados para `RepairPlan`, `DiagnosticReport`, `SessionLog` e matriz de suporte. Os schemas devem ter exemplos válidos em `examples/` e testes de validação.

---

## 10. Documentação por módulo

Cada `docs/modules/<module>.md` deve usar um template comum:

```markdown
# Nome do módulo

## Status
PLANNED | DESIGNED | IMPLEMENTED | ...

## Responsabilidade

## Não-responsabilidades

## Dependências permitidas

## Entradas e saídas

## Tipos principais

## Pré-condições

## Pós-condições

## Efeitos colaterais

## Comandos externos

## Tratamento de erros

## Regras de segurança

## Testes obrigatórios

## Arquivos de código relacionados

## Documentos relacionados

## Limitações conhecidas
```

Isso facilita a recuperação de contexto e reduz a chance de um agente criar uma segunda implementação para uma função já existente.

---

## 11. ADRs — registro de decisões de arquitetura

Toda decisão com impacto duradouro deve ter um ADR. Usar `docs/decisions/TEMPLATE.md` com:

- ID e título.
- Estado: `PROPOSED`, `ACCEPTED`, `REJECTED`, `SUPERSEDED`.
- Contexto/problema.
- Decisão.
- Alternativas consideradas.
- Consequências positivas e negativas.
- Segurança/compatibilidade afetadas.
- Documentos e código que precisam mudar.

ADRs iniciais sugeridos:

- `ADR-0001`: Rust como linguagem principal.
- `ADR-0002`: Ratatui/Crossterm para TUI.
- `ADR-0003`: pacote Pacman como distribuição principal e AUR como objetivo futuro.
- `ADR-0004`: sistema de logs estruturados e exportação manual.
- `ADR-0005`: diagnóstico separado de execução de reparos.
- `ADR-0006`: arquitetura de adaptadores de bootloader.
- `ADR-0007`: modelo de privilégio e execução de comandos.
- `ADR-0008`: política de suporte UEFI e layouts de ESP do MVP.
- `ADR-0009`: licença e reutilização de código upstream.

Não criar ADR para cada detalhe interno sem consequência arquitetural.

---

## 12. Referências aos projetos upstream

Criar `docs/references/UPSTREAM_PROJECTS.md` contendo URL, licença, linguagem, componentes úteis, versão/commit auditado e o que foi efetivamente reutilizado.

### Cachy-Chroot

- Repositório: https://github.com/CachyOS/cachy-chroot
- O projeto é Rust e tem licença BSD-3-Clause.
- A documentação oficial descreve descoberta de partições, subvolumes Btrfs, suporte a LUKS e montagem/chroot/desmontagem em ambiente Live.
- Auditar o código e a API antes de decidir se alguma lógica será extraída, adaptada ou simplesmente usada como referência.
- Não presumir que o binário interativo `cachy-chroot` seja uma biblioteca Rust importável.

### CachyOS Bootloader Switcher

- Repositório: https://github.com/seba970423/cachyos-bootloader-switcher
- O repositório consultado apresenta o Switcher r47, implementação baseada em Bash e licença GPL-3.0.
- Usar como referência de fluxos, validação, backups, ownership e rollback; não presumir que seja dependência Rust.
- A documentação r47 declara validação real em uma plataforma UEFI específica: uma ESP VFAT montada em `/boot`, root F2FS, sem LUKS, LVM, RAID, múltiplas ESPs ou Secure Boot. Esses resultados não provam compatibilidade com outras topologias.
- Se houver cópia/adaptação direta de código, realizar auditoria de licença e preservar avisos/licenças. A decisão de licença do Boot Rescue precisa ser registrada antes da incorporação de código GPL.

### Regra de atualização

Registrar a URL e o commit/tag exatos usados na auditoria. Revisar upstream periodicamente e nunca atualizar silenciosamente a arquitetura com base em comportamento não verificado.

---

## 13. Plano de testes documentado

### Camadas de teste

1. **Unitários:** parsing de fstab/crypttab, detecção, classificação, geração de plano e sanitização.
2. **Integração sem escrita real:** comandos simulados e fixtures; validar argumentos, ordem e tratamento de erros.
3. **Filesystem sandbox:** imagens de disco temporárias com partições de teste, Btrfs e FAT, sem tocar discos do host.
4. **VM UEFI:** QEMU/OVMF com imagens descartáveis, snapshots e várias topologias.
5. **Hardware real:** somente após testes anteriores, com backup e mídia Live de recuperação disponível.

### Matriz de ambientes

A matriz deve ter colunas para:

- UEFI/BIOS;
- Secure Boot;
- quantidade de ESPs;
- ESP em `/boot`, `/efi` ou `/boot/efi`;
- filesystem raiz;
- Btrfs com subvolumes;
- LUKS;
- LVM/RAID;
- Windows dual boot;
- bootloader;
- resultado por etapa;
- evidência/log;
- data e commit.

Nenhuma combinação deve ser chamada de suportada apenas porque o programa conseguiu detectar ou montar a instalação.

### Failure injection

Testar falhas em cada limite de escrita:

- comando ausente;
- timeout;
- exit code diferente de zero;
- filesystem read-only;
- ESP cheia;
- múltiplas ESPs ambíguas;
- UUID ausente/inconsistente;
- falha depois do backup;
- falha durante escrita;
- falha durante pós-validação;
- interrupção antes/depois de persistir o journal;
- relatório com conteúdo sensível para validar redaction.

---

## 14. Empacotamento e desenvolvimento

### `docs/development/PACKAGING.md`

Documentar:

- PKGBUILD como origem da compilação do pacote.
- Geração do `.pkg.tar.zst` com `makepkg`.
- Instalação de teste com `sudo pacman -U`.
- Verificação do pacote com ferramentas como `namcap`.
- Separação entre `depends`, `makedepends` e `optdepends`.
- Não exigir todos os bootloaders como dependências obrigatórias.
- Localização padrão dos arquivos (`/usr/bin`, `/usr/share/doc`, `/usr/share/licenses`, man page e configurações em `/etc` quando necessárias).
- Preparação futura de pacote AUR, sem publicar antes de existir release versionada e PKGBUILD reproduzível.

### `docs/development/BUILD.md`

Documentar versões mínimas, comandos de desenvolvimento e verificação. Exemplo a ajustar quando o projeto real for criado:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --release
makepkg -s
```

Os comandos exatos devem acompanhar o workspace e os recursos efetivamente implementados.

### `docs/development/DEPENDENCIES.md`

Manter tabela com crate/binário externo, versão mínima, finalidade, obrigatório/opcional, pacote Arch correspondente, motivo e alternativa. Não confiar em dependências transitivas para ferramentas externas.

---

## 15. Fluxo de trabalho de agentes sem perguntas básicas

Cada issue deve oferecer contexto suficiente. Um agente deverá seguir este roteiro:

### Antes de programar

- Ler `AGENTS.md`.
- Ler `docs/ai/START_HERE.md`.
- Ler `docs/ai/CURRENT_STATUS.md`.
- Consultar `DOCUMENTATION_MAP.md`.
- Ler contrato do módulo e ADRs relevantes.
- Conferir o código existente e os testes atuais.

### Durante a implementação

- Não criar APIs públicas sem atualizar o contrato.
- Não introduzir dependências sem ADR ou justificativa técnica.
- Não duplicar lógica de parsing, execução de comandos ou detecção.
- Não usar comandos shell montados por concatenação.
- Não executar ações de escrita durante testes comuns.
- Registrar decisões novas como ADR `PROPOSED`.

### Antes de concluir

- Rodar format, clippy e testes relevantes.
- Atualizar documentação, status e exemplos/schema.
- Verificar links internos.
- Declarar o que foi testado e o que continua sem validação.
- Listar limitações e riscos residuais.

### Quando pode perguntar ao mantenedor

Somente quando houver uma decisão de produto/licença não registrada ou uma ambiguidade que possa causar perda de dados, escrita no disco errado, alteração de NVRAM, exposição de dados sensíveis ou mudança de escopo. O agente deve primeiro consultar ADRs e contexto existentes.

---

## 16. Modelo de issue para tarefas implementáveis por IA

Toda issue de desenvolvimento deve usar este formato:

```markdown
## Objetivo
Uma frase clara descrevendo o resultado esperado.

## Contexto
Por que isso é necessário e qual fluxo afeta.

## Documentos obrigatórios
Links para contratos, arquitetura e ADRs relevantes.

## Escopo
O que deve mudar.

## Fora de escopo
O que não deve ser alterado.

## Critérios de aceitação
- [ ] Resultado observável 1
- [ ] Resultado observável 2
- [ ] Teste para caminho de sucesso
- [ ] Teste para falha
- [ ] Documentação atualizada

## Segurança
Quais dados/dispositivos/arquivos podem ser modificados?

## Evidência
Testes, fixtures, logs ou capturas necessários.
```

---

## 17. Validação automática da documentação

Criar uma tarefa CI que:

- verifique links relativos quebrados;
- valide JSON contra os schemas;
- valide exemplos JSON;
- verifique que cada módulo público listado possui documentação;
- procure referências a funcionalidades que ainda estejam `PLANNED` mas sejam descritas como existentes;
- exija atualização de `CURRENT_STATUS.md` quando arquivos em módulos críticos forem alterados (quando viável);
- valide diagramas Mermaid ou ao menos garanta que estejam em arquivos Markdown legíveis;
- execute verificações de licença e presença dos avisos upstream relevantes.

Adicionar um checklist de revisão documental no Pull Request Template.

---

## 18. Plano de criação da documentação por etapas

### Etapa 0 — Congelar decisões e pressupostos

- Criar `PROJECT_CONTEXT.md`.
- Registrar o que já foi decidido.
- Separar decisões confirmadas das propostas.
- Auditar licenças e formato dos projetos upstream.

**Entrega:** agentes sabem o que não devem rediscutir e o que ainda não está definido.

### Etapa 1 — Contexto imediato para IA

- Criar `AGENTS.md`.
- Criar `START_HERE.md`.
- Criar `CURRENT_STATUS.md`.
- Criar `DOCUMENTATION_MAP.md`.
- Criar `TASK_WORKFLOW.md`.

**Entrega:** agente consegue se orientar sem ler toda a documentação.

### Etapa 2 — Arquitetura e contratos

- Criar visão geral e grafo de módulos.
- Definir tipos e contratos.
- Documentar ciclo de sessão, executor de comandos e privilégios.
- Definir fluxo diagnóstico → plano → backup → confirmação → execução → validação.

**Entrega:** agente sabe onde implementar cada responsabilidade.

### Etapa 3 — Domínio Linux e bootloaders

- Documentar UEFI, ESP, Btrfs, fstab, crypttab e chroot.
- Documentar GRUB e Limine com layouts e pré-condições.
- Definir claramente suporte, não suporte e comportamento para ambiguidade.

**Entrega:** agente não precisa adivinhar como uma instalação Linux é organizada.

### Etapa 4 — Segurança, logs e recuperação

- Escrever modelo de ameaças.
- Definir operações por nível de risco.
- Definir backup, journal e rollback.
- Definir logging estruturado, sanitização e exportação manual.

**Entrega:** agente entende o limite de segurança de cada alteração.

### Etapa 5 — Testes e empacotamento

- Documentar testes unitários, sandbox e VM.
- Criar matriz de compatibilidade.
- Documentar build, PKGBUILD, dependências e release.

**Entrega:** agente consegue testar e entregar alterações reproduzíveis.

### Etapa 6 — Automatizar consistência

- Validar links, schemas e exemplos na CI.
- Adicionar templates de issue/PR.
- Fazer uma tarefa piloto implementada por agente usando somente a documentação.
- Registrar lacunas encontradas e corrigir os documentos.

**Entrega:** documentação validada por uso real, não apenas escrita.

---

## 19. Critério de sucesso: teste de onboarding de um agente

A documentação estará pronta para o objetivo quando um agente novo, sem histórico da conversa, conseguir realizar esta tarefa de avaliação:

> Adicione um diagnóstico somente leitura que detecte se a ESP está montada em `/boot`, `/efi` ou `/boot/efi`, produza um `DiagnosticFinding` estruturado, registre eventos sem expor dados sensíveis e adicione testes.

Sem perguntar ao mantenedor, o agente deverá conseguir:

1. Encontrar o módulo correto.
2. Identificar tipos existentes.
3. Respeitar o modelo de privilégios.
4. Implementar sem executar escrita no disco.
5. Adicionar testes com fixtures.
6. Atualizar documentação/status.
7. Rodar os comandos de validação.
8. Explicar claramente o que não foi testado em hardware.

Se falhar, a documentação ainda tem lacunas. A tarefa de onboarding deve ser repetida após cada grande mudança arquitetural.

---

## 20. Decisões iniciais que devem orientar o agente

| Área | Diretriz inicial | Estado |
|---|---|---|
| Nome | CachyOS Boot Rescue | Decidido |
| Tipo | Projeto independente | Decidido |
| Linguagem | Rust | Decidido |
| Interface | TUI com Ratatui/Crossterm | Decidido como direção |
| Distribuição | Pacote Pacman `.pkg.tar.zst` | Decidido |
| AUR | Objetivo futuro | Decidido |
| Plataforma inicial | CachyOS/Arch x86_64 em ambiente Live/recuperação | Proposta de escopo |
| Bootloaders iniciais | GRUB e Limine | Direção definida |
| Diagnóstico | Somente leitura antes do plano de reparo | Regra de segurança proposta |
| Logs | Discretos, estruturados e exportados manualmente | Decidido como requisito |
| Envio remoto | Nunca automático | Regra de privacidade proposta |
| Migração de bootloader | Fora do MVP | Proposta de escopo |
| Secure Boot | Fora do MVP até ter desenho e testes | Proposta de escopo |
| BIOS/Legacy | Fora do MVP inicial | Proposta de escopo |
| Destruição/formatação | Não oferecida pelo MVP | Regra de segurança proposta |

---

## 21. Próxima ação recomendada

A primeira entrega documental não deve tentar escrever todos os manuais de uma vez. Criar primeiro os sete arquivos que habilitam o onboarding:

1. `AGENTS.md`
2. `docs/ai/START_HERE.md`
3. `docs/ai/PROJECT_CONTEXT.md`
4. `docs/ai/CURRENT_STATUS.md`
5. `docs/ai/DOCUMENTATION_MAP.md`
6. `docs/ai/TASK_WORKFLOW.md`
7. `docs/decisions/TEMPLATE.md`

Em seguida, documentar arquitetura, segurança e contratos dos módulos antes de implementar rotinas que escrevem na ESP ou na NVRAM.

**Regra final:** o código e os testes descrevem o comportamento realmente implementado; os documentos de arquitetura definem o comportamento pretendido; `CURRENT_STATUS.md` liga os dois e informa o que já foi comprovado. Um agente nunca deve tratar uma proposta como funcionalidade pronta.
