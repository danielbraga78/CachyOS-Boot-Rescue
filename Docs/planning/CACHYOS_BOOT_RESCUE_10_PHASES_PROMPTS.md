# CachyOS Boot Rescue — Plano de implementação em 10 fases

Documento canônico do roadmap e dos prompts de implementação.

## Como usar este plano

Execute as fases em ordem. A partir da Fase 3, cada fase tem dois prompts: um para o Agente 1, líder, e outro para o Agente 2, responsável pela tarefa paralela. Eles podem trabalhar em paralelo depois que o Agente 1 definir e compartilhar os contratos necessários. A fase só termina após o Agente 1 integrar as entregas, validar os critérios de aceitação e apresentar o resumo. Não peça a nenhum agente para começar a fase seguinte.

### Regras globais para todos os agentes

1. Leia `AGENTS.md`, `Docs/ai/START_HERE.md`, `Docs/ai/PROJECT_CONTEXT.md`, `Docs/ai/CURRENT_STATUS.md` e os documentos indicados no mapa de documentação antes de alterar código.
2. Inspecione o estado real do repositório. Não presuma que arquivos, APIs, dependências ou funcionalidades descritas como planejadas já existem.
3. Implemente somente o escopo da fase atual. Não antecipe funcionalidades futuras.
4. Não deixe `TODO`, `todo!()`, `unimplemented!()`, stubs ou testes ignorados para satisfazer critérios de aceitação.
5. Não execute comandos destrutivos no host de desenvolvimento. Operações sobre discos reais só poderão ser implementadas nesta etapa como código isolado e testável; os testes devem usar mocks, fixtures ou VMs descartáveis.
6. Não execute `mkfs`, `wipefs`, `parted`, `fdisk` em modo de escrita, `sgdisk` em modo de escrita, `grub-install`, `limine-install`, `efibootmgr` em modo de escrita ou comandos equivalentes durante testes automatizados.
7. Use APIs Rust seguras sempre que possível. Para comandos externos, use `std::process::Command` com argumentos separados, nunca concatene entrada do usuário em uma shell.
8. Não registre senhas, chaves LUKS, tokens, conteúdo de arquivos privados, variáveis de ambiente completas ou dados sensíveis.
9. Operações privilegiadas devem ser explícitas, validadas e auditáveis. Não implemente elevação de privilégio implícita nesta fase.
10. Atualize documentação e `Docs/ai/CURRENT_STATUS.md` ao concluir cada fase. Registre decisões arquiteturais novas como ADR.
11. Rode `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` e `cargo test --all-targets` quando aplicável.
12. Se um pré-requisito estiver ausente, pare, descreva precisamente o bloqueio e não simule uma conclusão.
13. Ao final, apresente: arquivos criados/alterados, decisões tomadas, comandos executados, resultados dos testes, limitações conhecidas e confirmação de que não iniciou a próxima fase.

### Critério de passagem entre fases

Uma fase só está concluída quando:
- todos os critérios de aceitação da fase passaram;
- os testes relevantes passaram;
- não há warnings Clippy não justificados;
- documentação e status foram atualizados;
- não existem stubs ou pendências ocultas relacionadas ao escopo;
- o agente entregou um relatório verificável.

---

# Fase 1 — Fundação do projeto

**Resultado:** um projeto Rust mínimo, compilável, executável, testável e documentado. Ainda não detecta discos, não monta partições e não repara boot.

## Prompt 1

> Você está implementando a Fase 1 do CachyOS Boot Rescue.
>
> **Leia primeiro** os arquivos de contexto existentes. Se ainda não existirem, crie somente a estrutura mínima de documentação de IA especificada abaixo. Inspecione o repositório antes de alterar qualquer coisa.
>
> ### Objetivo
> Criar a fundação do projeto em Rust, com estrutura modular, configuração tipada, tratamento básico de erros, testes e CI. Não implemente funcionalidades de recuperação.
>
> ### Passo a passo
> 1. Inicialize ou valide um projeto Cargo chamado `cachyos-boot-rescue`, com binário `cachyos-boot-rescue`.
> 2. Defina uma política de MSRV (versão mínima de Rust) compatível com a versão estável atual disponível no ambiente de CI; documente a escolha. Não invente uma MSRV sem verificar a toolchain escolhida.
> 3. Crie módulos vazios apenas quando forem necessários para a fundação: `app`, `config`, `error` e `cli`. Não crie antecipadamente módulos de hardware ou reparo.
> 4. Implemente um tipo central de erro com contexto e conversão de erros de I/O. Mensagens apresentadas ao usuário devem ser compreensíveis e não conter stack traces por padrão.
> 5. Implemente configuração inicial com valores padrão e validação. Nesta fase, a configuração deve conter apenas opções não perigosas, como nível de log, diretório de dados e modo de interface. Não permita definir comandos arbitrários.
> 6. Adicione argumentos CLI mínimos: `--version`, `--help` e `--config` opcional. A execução normal deve mostrar uma mensagem de boas-vindas ou tela inicial simples, sem executar ações administrativas.
> 7. Defina uma estrutura de diretórios coerente para `src/`, `tests/`, `Docs/`, `packaging/` e `.github/workflows/`.
> 8. Crie `AGENTS.md` e os documentos `Docs/ai/START_HERE.md`, `Docs/ai/PROJECT_CONTEXT.md`, `Docs/ai/CURRENT_STATUS.md`, `Docs/ai/DOCUMENTATION_MAP.md`, `Docs/ai/DEVELOPMENT_RULES.md` e `Docs/ai/TASK_WORKFLOW.md`. Esses documentos devem registrar decisões confirmadas e marcar explicitamente o que ainda é planejado.
> 9. Adicione GitHub Actions para format, Clippy e testes em push e pull request.
> 10. Crie testes para configuração, argumentos CLI e tratamento de erros.
>
> ### Critérios de aceitação
> - `cargo build --locked` funciona.
> - `cargo run -- --help` e `cargo run -- --version` funcionam.
> - `cargo fmt --check` passa.
> - `cargo clippy --all-targets --all-features -- -D warnings` passa.
> - `cargo test --all-targets` passa.
> - CI executa os mesmos controles.
> - Nenhuma operação de disco, montagem, chroot ou bootloader é executada.
>
> ### Fora do escopo
> Logs em arquivo, descoberta de hardware, TUI completa, privilégios, montagem, chroot, diagnóstico e reparação.
>
> Ao terminar, atualize `Docs/ai/CURRENT_STATUS.md` e entregue o relatório padrão. Não inicie a Fase 2.

---

# Fase 2 — Logging, sessões e relatórios de diagnóstico

**Pré-requisito:** Fase 1 concluída e validada.

**Resultado:** logs discretos persistidos localmente, captura segura de eventos e exportação sanitizada. Nenhum comando de recuperação será executado.

## Prompt 2

> Implemente a Fase 2 do CachyOS Boot Rescue. Leia o contexto e inspecione o código real da Fase 1.
>
> ### Objetivo
> Criar um sistema de logging interno, discreto por padrão, com sessões, níveis de severidade, rotação e exportação de relatório sanitizado. Não implemente ainda captura de comandos externos: isso será integrado quando existirem operações reais.
>
> ### Passo a passo
> 1. Escolha e configure `tracing`, `tracing-subscriber` e, se necessário, `tracing-appender`. Registre a decisão em ADR.
> 2. Crie `logging` com inicialização única, níveis `ERROR`, `WARN`, `INFO`, `DEBUG` e `TRACE`, e um identificador de sessão.
> 3. Defina caminhos de log configuráveis. Em instalação normal, use diretório apropriado sob `/var/log`; em execução sem permissão de escrita, use diretório de usuário ou diretório temporário seguro. Não falhe silenciosamente.
> 4. Implemente rotação por tamanho e/ou número de arquivos, com limites documentados. Evite manter logs ilimitados.
> 5. Defina eventos estruturados com timestamp, session_id, componente, operação, severidade e mensagem.
> 6. Implemente um coletor de relatório que reúna somente arquivos da sessão selecionada e metadados explicitamente permitidos.
> 7. Implemente sanitização antes da exportação. Remova ou masque padrões de senhas, tokens, chaves, material LUKS, conteúdo de variáveis de ambiente, nomes de usuário quando configurado e identificadores de máquina. Não alegue que regex garante anonimização perfeita.
> 8. Faça a exportação em arquivo compactado com nome previsível e sem sobrescrever arquivos existentes. Prefira formato aberto e documentado.
> 9. Adicione opção CLI para mostrar o caminho dos logs e exportar um relatório. Não implemente envio automático à internet.
> 10. Adicione testes com dados sintéticos contendo segredos conhecidos para garantir que o sanitizador os remova.
>
> ### Critérios de aceitação
> - Inicialização cria uma sessão identificável.
> - Eventos são gravados sem aparecer na interface normal.
> - Rotação respeita limites configurados.
> - Exportação não altera os logs originais.
> - Testes provam que os segredos sintéticos não aparecem no relatório exportado.
> - Falhas de escrita/exportação são reportadas claramente.
> - Não há upload, telemetria ou transmissão de dados.
>
> ### Fora do escopo
> Captura de stdout/stderr de comandos reais, descoberta de hardware, interface TUI e reparação.
>
> Atualize `Docs/modules/logging.md`, `Docs/modules/reporting.md`, `Docs/safety/PRIVACY.md`, schemas de log/relatório se já definidos, ADRs e `Docs/ai/CURRENT_STATUS.md`. Não inicie a Fase 3.

---

# Divisão de trabalho por agentes — da Fase 3 em diante

A Fase 2 continua **em andamento**: não a encerre, não a marque como concluída e não aplique a divisão abaixo a ela. A divisão começa somente na Fase 3.

Em cada fase a partir da Fase 3, o **Agente 1** lidera e executa aproximadamente **70% do esforço**, concentrando a implementação principal, as decisões de integração e a validação final. O **Agente 2** executa os **30% restantes** em tarefas delimitadas que possam avançar em paralelo, sem duplicar o trabalho do Agente 1.

Para evitar conflitos, trabalhem em branches ou worktrees separados. Antes do trabalho paralelo, o Agente 1 registra os contratos/interfaces necessários para a tarefa do Agente 2. Cada agente altera somente os arquivos atribuídos; se uma alteração compartilhada for inevitável, o Agente 1 a integra. O Agente 2 entrega sua parte ao Agente 1 e não encerra a fase, atualiza o status global nem inicia a fase seguinte. O Agente 1 integra as entregas, resolve incompatibilidades, executa os critérios de aceitação da fase e apresenta o relatório final. As porcentagens representam esforço estimado, não quantidade de itens.

---

# Fase 3 — Descoberta de hardware, discos e instalações

**Pré-requisito:** Fases 1 e 2 concluídas.

**Resultado:** inventário somente leitura de dispositivos, partições, filesystems, EFI e instalações Linux candidatas.

### Divisão entre agentes (70% / 30%)
- **Agente 1 — 70% (líder):** definir e implementar os modelos de descoberta, adaptadores `lsblk`/`blkid`, identificação UEFI, classificação de dispositivos/filesystems, limites, timeouts e logging. Integrar os resultados ao relatório de descoberta.
- **Agente 2 — 30% (paralelo):** criar fixtures JSON para os cenários solicitados e testes de contrato/parsing em arquivos de teste separados, usando os modelos e interfaces documentados pelo Agente 1. Não alterar os módulos de produção nem escolher dispositivos ou ESPs.
- **Integração:** o Agente 1 publica o contrato dos dados de entrada/saída antes do início dos testes paralelos e integra as fixtures/testes antes da validação final.

## Prompt 3A — Agente 1 (70%, líder)

> Você é o Agente 1 e lidera a Fase 3 do CachyOS Boot Rescue. Leia os documentos de arquitetura, segurança, logging e testes. Inspecione o código atual e implemente o escopo abaixo. Defina e compartilhe com o Agente 2 os modelos/interfaces necessários para as fixtures e testes. Integre a entrega dele antes da validação final; não encerre a fase antes disso.
>
> ### Objetivo
> Criar um mecanismo de descoberta **somente leitura**. Nesta fase, o programa identifica discos, partições, sistemas de arquivos, ESPs e possíveis instalações Linux, mas não monta, desmonta, desbloqueia nem modifica nada.
>
> ### Passo a passo
> 1. Defina modelos tipados para `BlockDevice`, `Partition`, `Filesystem`, `EspCandidate`, `LinuxInstallationCandidate` e `DiscoveryReport`.
> 2. Implemente um adaptador para consultar `lsblk` em JSON, com argumentos fixos e parsing robusto. Não analise saída humana/localizada.
> 3. Use `blkid` ou interfaces equivalentes somente para complementar dados que não estejam disponíveis. Trate ausência de ferramentas como diagnóstico, não como panic.
> 4. Identifique ESPs por tipo de partição GPT e filesystem FAT, sem assumir que toda partição FAT é EFI.
> 5. Identifique candidatos Linux usando marcadores como `/etc/os-release`, `etc/fstab` e diretórios de boot quando puderem ser inspecionados com segurança. Ainda não monte partições: limite-se a dispositivos já montados ou a fontes de metadados disponíveis.
> 6. Detecte o estado UEFI por `/sys/firmware/efi` e registre quando o programa foi iniciado em modo BIOS/Legacy.
> 7. Identifique Btrfs, ext4, XFS, FAT/VFAT, NTFS, swap e tipos desconhecidos sem tentar reparar filesystems.
> 8. Associe UUID, PARTUUID, label, major/minor e caminho do dispositivo quando disponíveis.
> 9. Implemente timeouts, limites de saída e tratamento de comando ausente/falha.
> 10. Registre operações e resultados usando o sistema de logging da Fase 2.
> 11. Integre e revise as fixtures JSON de `lsblk` e os testes criados pelo Agente 2 para discos SATA, NVMe, múltiplas ESPs, partições sem label, discos removíveis e dados incompletos.
>
> ### Critérios de aceitação
> - Nenhuma partição é montada ou modificada.
> - Parsing independe do idioma do sistema.
> - Múltiplas ESPs são apresentadas como candidatas, não escolhidas arbitrariamente.
> - Dados ausentes são representados como desconhecidos, não inventados.
> - Testes usam fixtures e não dependem do hardware do desenvolvedor.
> - Erros e comandos executados são registrados sem dados sensíveis.
>
> ### Fora do escopo
> Montagem, LUKS unlock, chroot, TUI de seleção e qualquer reparo.
>
> Atualize `Docs/modules/discovery.md`, `Docs/domain/UEFI.md`, `Docs/domain/EFI_PARTITIONS.md`, `Docs/domain/FILESYSTEMS.md`, `Docs/testing/HARDWARE_MATRIX.md` e `Docs/ai/CURRENT_STATUS.md`. Não inicie a Fase 4.

## Prompt 3B — Agente 2 (30%, paralelo)

> Você é o Agente 2 da Fase 3 do CachyOS Boot Rescue. Leia as regras globais e os requisitos/critério de aceitação da Fase 3 neste documento; use os modelos/interfaces publicados pelo Agente 1.
>
> Crie fixtures JSON e testes de contrato/parsing de `lsblk` para SATA, NVMe, múltiplas ESPs, partições sem label, discos removíveis e dados incompletos. Use somente hardware fictício. Não altere módulos de produção, não consulte nem modifique discos reais, e não escolha uma ESP arbitrariamente.
>
> Se os contratos necessários ainda não estiverem definidos, solicite-os ao Agente 1 antes da parte dependente. Mantenha as alterações em arquivos de fixtures/testes e entregue os arquivos, comandos executados, resultados e limitações ao Agente 1. Não atualize o status global, não declare a fase concluída e não inicie a Fase 4.

---

# Fase 4 — Storage Manager e Chroot Manager

**Pré-requisito:** Fase 3 concluída e descoberta somente leitura testada.

**Resultado:** montagem controlada de uma instalação selecionada, com desmontagem garantida e entrada/saída de chroot. Ainda sem reparadores de boot.

### Divisão entre agentes (70% / 30%)
- **Agente 1 — 70% (líder):** implementar execução segura de mounts, `MountSession`, limpeza/desmontagem, `ChrootManager`, execução controlada de comandos e guardas contra alvos ambíguos. É responsável por toda operação privilegiada e pela integração.
- **Agente 2 — 30% (paralelo):** implementar o parser puro de `fstab` e os testes unitários de resolução de UUID/PARTUUID/LABEL e opções relevantes, em módulo e arquivos de teste separados. Não executar mounts nem alterar o `ChrootManager`.
- **Integração:** antes do trabalho paralelo, o Agente 1 define os tipos de entrada/saída do parser e o formato do `MountPlan`; depois integra o módulo do Agente 2 e valida o fluxo completo.

## Prompt 4A — Agente 1 (70%, líder)

> Você é o Agente 1 e lidera a Fase 4 do CachyOS Boot Rescue. Leia os contratos de discovery e segurança e verifique que os modelos da Fase 3 existem. Defina e compartilhe com o Agente 2 os tipos de entrada/saída do parser `fstab` e o contrato do `MountPlan`. Implemente o gerenciamento de mounts e chroot, integre a entrega dele e conduza a validação completa.
>
> ### Objetivo
> Implementar montagem segura de uma instalação Linux selecionada e gerenciamento de chroot. Não executar comandos de reparação de boot.
>
> ### Passo a passo
> 1. Defina um `MountPlan` explícito contendo dispositivos, filesystem, mountpoints, opções, subvolumes e dependências entre mounts.
> 2. Integre o parser puro de `/etc/fstab` entregue pelo Agente 2. Verifique suporte a UUID, PARTUUID, LABEL, caminhos de dispositivo, `nofail`, `noauto`, `subvol` e opções relevantes sem assumir que todas as entradas devem ser montadas.
> 3. Resolva UUID/PARTUUID para dispositivos e valide que o alvo corresponde à instalação selecionada.
> 4. Para Btrfs, identifique subvolumes existentes e monte o subvolume raiz correto conforme fstab e estrutura encontrada. Não assuma automaticamente `@`; confirme o que existe.
> 5. Monte primeiro a raiz e depois os pontos de montagem necessários, incluindo `/boot`, `/boot/efi` ou `/efi`, conforme fstab e evidências. Detecte conflitos e ambiguidades.
> 6. Implemente suporte a LUKS somente para detectar e solicitar desbloqueio interativo explícito. Não armazene senha, não a registre e não a passe em argumentos visíveis de processo. Se não for possível tratar a senha com segurança, documente a limitação e não implemente desbloqueio inseguro.
> 7. Crie um `MountSession` que mantenha o inventário de mounts realizados pelo aplicativo.
> 8. Implemente desmontagem em ordem inversa, inclusive em erros e encerramento normal. Nunca desmonte mounts que não foram criados pela sessão sem confirmação explícita.
> 9. Implemente `ChrootManager` para preparar pseudo-filesystems necessários (`/dev`, `/proc`, `/sys`, `/run`) com bind/rbind apropriados, rastrear o que foi montado e executar comandos em ambiente controlado.
> 10. Não use `chroot` com string de shell. Execute o binário e argumentos explicitamente dentro do ambiente.
> 11. Capture stdout, stderr, exit code, duração e timeout dos comandos usando o logging. A captura deve ter limites de tamanho e sanitização.
> 12. Implemente guardas para impedir chroot se a raiz ou os mounts de boot necessários estiverem ambíguos.
> 13. Teste a lógica de planejamento e desmontagem com mocks; testes de integração devem usar imagem/disco de VM descartável, nunca o host.
>
> ### Critérios de aceitação
> - O plano de montagem pode ser revisado antes de execução.
> - Montagem e desmontagem são rastreáveis e idempotentes quando possível.
> - Falhas parciais provocam limpeza dos mounts feitos pela sessão.
> - Não desmonta recursos preexistentes sem autorização.
> - Chroot não recebe segredos em argumentos ou logs.
> - Comandos são capturados com limite de saída e timeout.
> - Não existe chamada a `grub-install`, `limine-install`, `efibootmgr` em modo de escrita ou `mkinitcpio`.
>
> ### Fora do escopo
> Interface final, diagnóstico de boot e reparação.
>
> Atualize `Docs/modules/storage.md`, `Docs/modules/chroot.md`, `Docs/domain/BTRFS.md`, `Docs/domain/FSTAB_CRYPTTAB.md`, `Docs/domain/CHROOT.md`, `Docs/architecture/PRIVILEGE_MODEL.md` e `Docs/ai/CURRENT_STATUS.md`. Não inicie a Fase 5.

## Prompt 4B — Agente 2 (30%, paralelo)

> Você é o Agente 2 da Fase 4 do CachyOS Boot Rescue. Leia as regras globais e os requisitos/critério de aceitação da Fase 4 neste documento; use o contrato do parser e do `MountPlan` fornecido pelo Agente 1.
>
> Implemente, em módulo isolado, o parser puro de `/etc/fstab` e testes unitários para UUID, PARTUUID, LABEL, caminhos de dispositivo, `nofail`, `noauto`, `subvol` e opções relevantes. Cubra entradas inválidas, dados ausentes e ambiguidades. Não execute mounts, não desbloqueie LUKS e não altere `MountSession` ou `ChrootManager`. Não teste em partições reais.
>
> Se o contrato ainda não estiver definido, solicite-o ao Agente 1 antes da parte dependente. Entregue arquivos, comandos, resultados e limitações ao Agente 1. Não atualize o status global, não declare a fase concluída e não inicie a Fase 5.

---

# Fase 5 — Interface TUI e fluxo de usuário

**Pré-requisito:** discovery, logging, storage e chroot funcionais em testes.

**Resultado:** interface de terminal navegável que permite selecionar uma instalação, visualizar plano de montagem, abrir chroot e consultar logs. Ainda não executa reparos.

### Divisão entre agentes (70% / 30%)
- **Agente 1 — 70% (líder):** implementar navegação e estado central da aplicação, fluxo de descoberta/revisão/confirmação de montagem e chroot, operações demoradas e restauração confiável do terminal. Integrar a TUI aos módulos existentes.
- **Agente 2 — 30% (paralelo):** implementar telas isoladas de informações, ajuda/teclas e logs/relatórios, além dos testes de reducer/estado correspondentes, sem alterar o fluxo central de navegação.
- **Integração:** o Agente 1 define previamente a interface de estado/renderização das telas; integra os módulos e testes do Agente 2 e valida que nenhuma ação administrativa ocorra sem confirmação.

## Prompt 5A — Agente 1 (70%, líder)

> Você é o Agente 1 e lidera a Fase 5 do CachyOS Boot Rescue. Leia o fluxo de aplicação, modelos de discovery, MountPlan, MountSession e ChrootManager. Não presuma APIs: use as implementações existentes. Defina e compartilhe com o Agente 2 as interfaces de estado/renderização das telas isoladas. Implemente o fluxo central, integre a entrega dele e valide o fluxo completo.
>
> ### Objetivo
> Criar a interface TUI com Ratatui e Crossterm, conectando somente as operações já implementadas. Não adicione diagnóstico nem reparação.
>
> ### Passo a passo
> 1. Defina o modelo de estado da aplicação e navegação entre telas.
> 2. Crie a tela inicial e o fluxo central de navegação; integre as telas isoladas de logs/relatórios e informações do programa entregues pelo Agente 2.
> 3. Implemente tela de descoberta com lista de instalações e detalhes de disco, root, filesystem e ESPs candidatas.
> 4. Permita selecionar uma instalação e abrir uma tela de revisão do plano de montagem.
> 5. Exiba claramente mounts já existentes, mounts que o programa pretende criar e ambiguidades.
> 6. Implemente confirmação explícita antes de montar.
> 7. Após montagem bem-sucedida, ofereça abrir chroot ou desmontar. O chroot é uma operação administrativa e deve ser claramente identificado.
> 8. Implemente tela de progresso para operações demoradas sem bloquear o terminal.
> 9. Implemente tela de erro com mensagem curta, identificador da sessão e opção de exportar relatório sanitizado.
> 10. Integre a tela de logs/relatórios entregue pelo Agente 2 e garanta que detalhes técnicos permaneçam opt-in.
> 11. Garanta restauração do terminal ao sair, inclusive em panic e erro.
> 12. Adicione navegação por teclado, foco visível, ajuda de teclas e comportamento utilizável em terminais pequenos.
> 13. Crie testes do reducer/estado de UI sem depender de terminal real.
>
> ### Critérios de aceitação
> - Usuário navega pelas telas sem executar operações não confirmadas.
> - Discovery é acionado pela interface e resultados são apresentados.
> - Plano de montagem é revisado antes da execução.
> - Erros oferecem exportação local do relatório.
> - Terminal é restaurado ao sair.
> - Não existem opções fictícias para diagnóstico ou reparação.
>
> ### Fora do escopo
> Diagnóstico de bootloader, reparação e conversão de layout EFI.
>
> Atualize `Docs/architecture/OVERVIEW.md`, `Docs/architecture/DATA_FLOW.md`, `Docs/architecture/PROCESS_LIFECYCLE.md`, módulos pertinentes em `Docs/modules/`, manual de teclas e `Docs/ai/CURRENT_STATUS.md`. Não inicie a Fase 6.

## Prompt 5B — Agente 2 (30%, paralelo)

> Você é o Agente 2 da Fase 5 do CachyOS Boot Rescue. Leia as regras globais e os requisitos/critério de aceitação da Fase 5 neste documento; use as interfaces de estado/renderização acordadas com o Agente 1.
>
> Implemente telas isoladas de informações do programa, ajuda/teclas e consulta de logs/relatórios, além dos testes de reducer/estado correspondentes. Não altere a navegação central, o fluxo de montagem/chroot, a execução de operações ou o ciclo de vida do terminal. Não crie ações fictícias de diagnóstico ou reparo.
>
> Se as interfaces ainda não estiverem definidas, solicite-as ao Agente 1 antes da integração. Mantenha alterações isoladas, execute os testes pertinentes e entregue arquivos, comandos, resultados e limitações ao Agente 1. Não atualize o status global, não declare a fase concluída e não inicie a Fase 6.

---

# Fase 6 — Motor de diagnóstico somente leitura

**Pré-requisito:** TUI, discovery, montagem e chroot concluídos.

**Resultado:** diagnóstico real de GRUB e Limine, layout de boot, EFI, kernels, initramfs e entradas UEFI. Não modifica o sistema.

### Divisão entre agentes (70% / 30%)
- **Agente 1 — 70% (líder):** definir os modelos comuns de diagnóstico e implementar a composição dos achados, detecção do ambiente/layout EFI, verificações de GRUB, UUIDs, kernels/initramfs e integração com TUI, logging e relatório.
- **Agente 2 — 30% (paralelo):** implementar o adaptador isolado de diagnóstico do Limine e seus fixtures/testes, sem modificar os tipos comuns nem a composição do relatório.
- **Integração:** o Agente 1 publica primeiro os contratos de `Finding`, evidências e interface do adaptador; integra o diagnóstico do Limine e executa a validação conjunta somente leitura.

## Prompt 6A — Agente 1 (70%, líder)

> Você é o Agente 1 e lidera a Fase 6 do CachyOS Boot Rescue. Use somente as APIs existentes de discovery, storage, chroot, UI e logging. Defina e compartilhe com o Agente 2 os tipos comuns de achado/evidência e a interface do adaptador. Implemente os modelos e verificações centrais, integre a entrega dele e valide que o diagnóstico permanece somente leitura.
>
> ### Objetivo
> Criar diagnóstico somente leitura. O programa deve produzir achados estruturados, evidências, severidade e recomendações. Nenhuma correção será executada.
>
> ### Passo a passo
> 1. Defina tipos `DiagnosticRun`, `Finding`, `Evidence`, `Severity`, `Recommendation` e `BootEnvironment`.
> 2. Defina identificadores estáveis de diagnóstico, por exemplo `EFI_NOT_MOUNTED`, `GRUB_CONFIG_MISSING`, `LIMINE_CONFIG_MISSING`, `KERNEL_NOT_FOUND`, `INITRAMFS_NOT_FOUND`, `FSTAB_UUID_MISMATCH`.
> 3. Detecte o bootloader instalado por evidências em arquivos, diretórios, pacotes e entradas EFI. Não conclua somente pelo nome de uma entrada NVRAM.
> 4. Para GRUB, verifique presença de binário EFI, configuração `grub.cfg`, diretórios esperados, referência a UUIDs e existência de kernels/initramfs.
> 5. Integre e revise o adaptador de diagnóstico Limine e seus fixtures/testes entregues pelo Agente 2; verifique binário EFI, `limine.conf`, caminhos de kernel/initramfs e coerência com o layout da ESP.
> 6. Detecte `/boot`, `/boot/efi` e `/efi` sem assumir um layout universal. Compare fstab, mounts reais e conteúdo dos diretórios.
> 7. Consulte `efibootmgr -v` somente em modo leitura quando UEFI estiver disponível. Preserve todas as entradas e sinalize ambiguidades.
> 8. Verifique kernels e initramfs existentes, sem regenerá-los.
> 9. Verifique referências UUID/PARTUUID em fstab e configurações relevantes, sem modificá-las.
> 10. Produza recomendações textuais que expliquem evidência e risco. Não apresente uma recomendação como certeza se os dados forem insuficientes.
> 11. Conecte resultados à TUI e permita salvar relatório diagnóstico sanitizado.
> 12. Integre e revise as fixtures de sistemas saudáveis e cenários quebrados entregues pelo Agente 2; cubra os achados definidos nesta fase.
>
> ### Critérios de aceitação
> - Diagnóstico não altera arquivos, NVRAM ou partições.
> - Cada achado contém evidência reproduzível.
> - Falsos positivos e estados desconhecidos são tratados explicitamente.
> - Layout EFI separado e EFI em `/boot` são reconhecidos como configurações diferentes.
> - Testes cobrem GRUB, Limine, dual boot, múltiplas ESPs e ausência de UEFI.
> - Relatório é exportável e sanitizado.
>
> ### Fora do escopo
> Qualquer reparação, backup de alterações ou rollback.
>
> Atualize todos os documentos de diagnóstico e domínio, schemas de relatório, `Docs/testing/HARDWARE_MATRIX.md` e `Docs/ai/CURRENT_STATUS.md`. Não inicie a Fase 7.

## Prompt 6B — Agente 2 (30%, paralelo)

> Você é o Agente 2 da Fase 6 do CachyOS Boot Rescue. Leia as regras globais e os requisitos/critério de aceitação da Fase 6 neste documento; use os tipos comuns e a interface do adaptador publicados pelo Agente 1.
>
> Implemente o adaptador isolado de diagnóstico somente leitura do Limine e seus fixtures/testes. Verifique evidências de configuração, binário EFI e caminhos de kernel/initramfs conforme os requisitos da fase; represente incertezas sem inferir estado. Não altere os tipos comuns, a composição do relatório, a TUI nem arquivos do sistema de destino.
>
> Se o contrato ainda não estiver disponível, solicite-o ao Agente 1 antes da integração. Mantenha o adaptador e testes isolados, execute os testes pertinentes e entregue arquivos, comandos, resultados e limitações ao Agente 1. Não atualize o status global, não declare a fase concluída e não inicie a Fase 7.

---

# Fase 7 — Backup, transações e rollback

**Pré-requisito:** diagnóstico confiável e storage/chroot estáveis.

**Resultado:** mecanismo genérico para planejar mudanças, criar backup, executar transação, validar e restaurar arquivos alterados. Ainda não implementar comandos específicos de instalação de bootloader.

### Divisão entre agentes (70% / 30%)
- **Agente 1 — 70% (líder):** implementar a máquina de estados, preflight, confirmação, backup/manifests, aplicação transacional, journaling e rollback seguro. É responsável por garantir que nenhuma mutação ocorra sem autorização.
- **Agente 2 — 30% (paralelo):** criar testes de falhas injetadas em diretórios temporários, cobrindo interrupções em cada estado, integridade de backup e resultados de rollback, em arquivos de teste separados. Não implementar operações de produção nem testar em caminhos reais do sistema.
- **Integração:** o Agente 1 define os estados e pontos de injeção de falhas antes do trabalho paralelo; incorpora os testes e corrige a implementação antes da aceitação.

## Prompt 7A — Agente 1 (70%, líder)

> Você é o Agente 1 e lidera a Fase 7 do CachyOS Boot Rescue. Leia o modelo de segurança e o diagnóstico. Inspecione os pontos reais de integração. Defina e compartilhe com o Agente 2 os estados da transação e os pontos de injeção de falhas. Implemente o mecanismo transacional, integre a entrega dele e valide que nenhuma mutação ocorre sem autorização.
>
> ### Objetivo
> Criar um mecanismo genérico e seguro de transações para operações de reparação futuras. Nesta fase, teste o mecanismo somente com arquivos temporários e fixtures.
>
> ### Passo a passo
> 1. Defina `RepairPlan`, `RepairStep`, `Precondition`, `BackupManifest`, `Transaction`, `TransactionState` e `RollbackResult`.
> 2. Estabeleça estados explícitos: `Planned`, `PreflightPassed`, `BackedUp`, `AwaitingConfirmation`, `Applying`, `Validating`, `Committed`, `RollingBack`, `RolledBack` e `Failed`.
> 3. Cada passo deve declarar arquivos que poderá criar, alterar ou remover, comandos que pretende executar, pré-condições, riscos e validação posterior.
> 4. Implemente preflight que confirme instalação selecionada, mounts, espaço disponível, permissões, destino e ausência de conflito.
> 5. Implemente backup preservando conteúdo, permissões, ownership quando possível, timestamps e caminhos relativos. Não copie symlinks de forma insegura nem siga symlinks para fora da raiz autorizada.
> 6. Armazene manifest com hashes, tamanho, metadados e identificador da sessão. Não inclua segredos.
> 7. Implemente restauração atômica sempre que possível, verificando que o destino ainda corresponde ao arquivo que a transação modificou.
> 8. Implemente confirmação humana obrigatória antes de qualquer operação classificada como mutável.
> 9. Implemente journaling para detectar transações interrompidas e oferecer recuperação após reinicialização.
> 10. Defina política de retenção e limpeza de backups; nunca apague backup automaticamente durante uma recuperação em andamento.
> 11. Integre e revise os testes de rollback com falhas injetadas entregues pelo Agente 2; verifique que o estado anterior é restaurado.
> 12. Implemente uma tela de revisão do plano na TUI, sem conectar ainda comandos reais de bootloader.
>
> ### Critérios de aceitação
> - Um plano mostra exatamente os caminhos e operações que pretende alterar.
> - Nenhuma transação mutável ocorre sem confirmação.
> - Backup é criado e verificado antes da aplicação.
> - Falha simulada em qualquer etapa permite rollback ou apresenta claramente o que não pôde ser revertido.
> - Testes usam diretórios temporários, nunca `/boot`, `/etc` ou EFI reais.
> - Logs registram transição de estado e resultado.
>
> ### Fora do escopo
> Instalação de GRUB/Limine, alterações NVRAM e reparação real.
>
> Atualize `Docs/modules/backup.md`, `Docs/safety/BACKUP_POLICY.md`, `Docs/safety/ROLLBACK.md`, `Docs/safety/DESTRUCTIVE_OPERATIONS.md`, `Docs/architecture/ERROR_HANDLING.md`, schemas e `Docs/ai/CURRENT_STATUS.md`. Não inicie a Fase 8.

## Prompt 7B — Agente 2 (30%, paralelo)

> Você é o Agente 2 da Fase 7 do CachyOS Boot Rescue. Leia as regras globais e os requisitos/critério de aceitação da Fase 7 neste documento; use os estados e pontos de injeção de falhas definidos pelo Agente 1.
>
> Crie testes com falhas injetadas em diretórios temporários, cobrindo interrupções nos estados da transação, integridade do backup e resultados de rollback. Não implemente operações de produção, não use `/boot`, `/etc`, ESPs ou discos reais e não enfraqueça pré-condições para fazer testes passarem.
>
> Se os pontos de injeção ainda não estiverem definidos, solicite-os ao Agente 1 antes de codificar os testes dependentes. Entregue arquivos, comandos, resultados e limitações ao Agente 1. Não atualize o status global, não declare a fase concluída e não inicie a Fase 8.

---

# Fase 8 — Motor de reparação: GRUB, Limine e initramfs

**Pré-requisito:** todas as fases anteriores concluídas, especialmente diagnóstico e transações.

**Resultado:** primeiros reparadores reais, com plano, backup, confirmação, execução e validação. Esta é a primeira fase que implementa mutações reais, mas os testes automatizados continuam em VMs descartáveis.

### Divisão entre agentes (70% / 30%)
- **Agente 1 — 70% (líder):** implementar a autorização transacional, o reparador GRUB, a reconstrução de initramfs, a validação de mounts/layout e a integração dos reparadores com a TUI. Conduzir a validação de segurança e o fluxo de rollback.
- **Agente 2 — 30% (paralelo):** implementar o adaptador isolado de reparação Limine e seus testes em VM descartável/fixtures, sem alterar o adaptador GRUB, o motor de transações ou a interface comum.
- **Integração:** o Agente 1 define e revisa a trait/contrato comum antes do trabalho paralelo. O Agente 2 não executa reparos em discos do host; o Agente 1 revisa e integra o adaptador, confirma backups e autorização por operação e executa a validação final.

## Prompt 8A — Agente 1 (70%, líder)

> Você é o Agente 1 e lidera a Fase 8 do CachyOS Boot Rescue. Esta fase pode modificar a instalação selecionada; siga estritamente o modelo de segurança e use `RepairPlan` e `Transaction` da Fase 7. Defina e revise com o Agente 2 a trait/contrato comum dos reparadores. Implemente autorização transacional, reparadores GRUB/initramfs e integração; revise e integre a entrega dele. Nunca execute testes mutáveis em discos do host.
>
> ### Objetivo
> Implementar adaptadores independentes para GRUB, Limine e reconstrução de initramfs. Não implemente migração de bootloader nem conversão automática de layout.
>
> ### Passo a passo
> 1. Defina uma trait comum `BootloaderRepair` com métodos para detectar suporte, construir plano, validar pré-condições, aplicar e validar resultado. Não permita que o adaptador execute sem uma transação autorizada.
> 2. Implemente primeiro o reparador GRUB. Use o layout detectado e validado; não presuma `/boot/efi`. Gere plano para `grub-install` com argumentos explícitos e `grub-mkconfig` para o destino correto.
> 3. Antes de executar GRUB, confirme que a ESP selecionada está montada no local esperado, que o diretório de destino é inequívoco, que o pacote/ferramentas estão presentes e que o modo UEFI/BIOS é compatível.
> 4. Preserve entradas e arquivos do Windows. Não remova nem sobrescreva `EFI/Microsoft`. Trate `EFI/BOOT/BOOTX64.EFI` como caminho compartilhado e de alto risco; não o substitua por padrão.
> 5. Integre e revise o reparador Limine isolado e seus testes entregues pelo Agente 2. Confirme que validam topologia suportada, localização da ESP, caminhos de kernel/initramfs e sintaxe/configuração antes da instalação. Não converta `/boot/efi` para `/boot` automaticamente.
> 6. Revise a validação de sintaxe e comandos Limine realizada pelo Agente 2 contra a versão instalada e a documentação upstream atual. Não aceite flags ou caminhos não verificados.
> 7. Implemente reparação de initramfs usando `mkinitcpio -P` dentro do chroot, após verificar configuração, kernels e espaço disponível. Não execute se os mounts estiverem ambíguos.
> 8. Faça cada reparador construir um plano detalhado e mostrar ao usuário comandos, arquivos afetados, risco e backup que será criado.
> 9. Exija confirmação explícita por operação; confirmação genérica da sessão não autoriza operações futuras.
> 10. Execute comandos sem shell, capture stdout/stderr/exit code/timeout e registre no log. Redija possíveis segredos.
> 11. Valide os arquivos e configurações após execução. Não afirme que o computador inicializará; informe que a validação de boot real depende de reinicialização.
> 12. Se a validação falhar, ofereça rollback com resumo do que será restaurado.
> 13. Integre os reparadores à TUI e mostre resultado, relatório e próximos passos.
> 14. Teste em VMs UEFI descartáveis com snapshots, incluindo Windows Boot Manager e layouts de EFI distintos.
>
> ### Critérios de aceitação
> - GRUB e Limine possuem adaptadores separados.
> - Cada operação gera plano e backup antes de alterar arquivos.
> - Layout EFI não é alterado implicitamente.
> - Windows Boot Manager e diretórios Microsoft são preservados.
> - Não existe execução de comando mutável por diagnóstico.
> - Falhas de comando e validação disparam tratamento transacional.
> - Testes reais de instalação ocorrem somente em VM descartável.
>
> ### Fora do escopo
> Migração GRUB↔Limine, conversão de layout, Secure Boot e suporte a bootloaders adicionais.
>
> Atualize `Docs/modules/repair_engine.md`, `Docs/domain/BOOTLOADERS/GRUB.md`, `Docs/domain/BOOTLOADERS/LIMINE.md`, `Docs/domain/INITRAMFS.md`, `Docs/safety/`, exemplos de RepairPlan e `Docs/ai/CURRENT_STATUS.md`. Não inicie a Fase 9.

## Prompt 8B — Agente 2 (30%, paralelo)

> Você é o Agente 2 da Fase 8 do CachyOS Boot Rescue. Esta fase contém operações mutáveis: siga as regras globais e os requisitos da Fase 8 neste documento. Use somente o contrato do reparador comum revisado pelo Agente 1.
>
> Implemente o adaptador isolado de reparação Limine e seus testes em fixtures/VM descartável. Valide sintaxe, comandos e caminhos com a versão instalada e documentação upstream; não invente opções. Exija plano, pré-condições, backup e confirmação transacional por operação. Não altere o reparador GRUB, o motor de transações, a interface comum ou o fluxo da TUI. Nunca execute reparos em discos do host nem altere NVRAM durante testes.
>
> Se o contrato ou ambiente descartável seguro não estiver disponível, pare a parte bloqueada e reporte o motivo ao Agente 1; não substitua por execução no host. Entregue arquivos, comandos, resultados e limitações. Não atualize o status global, não declare a fase concluída e não inicie a Fase 9.

---

# Fase 9 — Testes de integração, segurança e compatibilidade

**Pré-requisito:** todos os fluxos de reparação da Fase 8 implementados.

**Resultado:** evidência automatizada de que o aplicativo funciona em cenários controlados e falha de forma segura em cenários problemáticos.

### Divisão entre agentes (70% / 30%)
- **Agente 1 — 70% (líder):** conduzir a suíte de regressão e segurança, verificar diagnóstico somente leitura, confirmação/backup/rollback, sanitização e limpeza de recursos; corrigir falhas encontradas no produto e consolidar os resultados.
- **Agente 2 — 30% (paralelo):** criar e executar cenários isolados de compatibilidade em fixtures/VMs descartáveis, registrar evidências por layout (GRUB/Limine, EFI, filesystems e dual boot) e atualizar somente os artefatos de matriz de compatibilidade atribuídos.
- **Integração:** o Agente 1 define os cenários e formato de evidência; integra os resultados do Agente 2, distingue testes não executados de aprovados e valida os critérios de aceitação da fase.

## Prompt 9A — Agente 1 (70%, líder)

> Você é o Agente 1 e lidera a Fase 9 do CachyOS Boot Rescue. Não adicione funcionalidades novas de produto, exceto correções necessárias para passar os testes. Defina e compartilhe com o Agente 2 os cenários e formato de evidência. Conduza os testes de regressão/segurança, integre os resultados e artefatos dele, corrija falhas e marque explicitamente cenários não executados.
>
> ### Objetivo
> Construir uma suíte de integração e segurança com VMs descartáveis e fixtures, validar cenários de recuperação e documentar precisamente a matriz de compatibilidade.
>
> ### Passo a passo
> 1. Crie estratégia de testes com camadas: unitário, integração com mocks, integração em imagem de filesystem e VM UEFI.
> 2. Prepare a infraestrutura compartilhada de testes com discos virtuais e snapshots usando ferramentas disponíveis no CI ou scripts reproduzíveis. Nunca use discos físicos do host.
> 3. Integre e revise os cenários de compatibilidade criados pelo Agente 2: GRUB com EFI separada em `/boot/efi` e ESP em `/boot`, Limine com ESP em `/boot`, Btrfs, ext4 e múltiplas ESPs.
> 4. Integre e revise os cenários dual boot entregues pelo Agente 2, verificando que entradas e arquivos Microsoft não são removidos.
> 5. Injete falhas: espaço insuficiente na ESP, UUID ausente, mountpoint ocupado, EFI não montada, configuração inválida, comando ausente, timeout, interrupção durante transação e falha de rollback.
> 6. Verifique que diagnóstico nunca altera o sistema.
> 7. Verifique que reparo nunca inicia sem confirmação e backup.
> 8. Verifique sanitização dos relatórios com padrões de segredo e dados pessoais sintéticos.
> 9. Verifique limpeza de mounts, processos filhos e restauração do terminal após falha.
> 10. Execute testes de instalação e boot em VM quando o ambiente permitir. Se não permitir, marque o cenário como não executado, não como aprovado.
> 11. Faça análise de dependências e vulnerabilidades usando ferramentas adequadas ao ecossistema Rust/Arch.
> 12. Gere relatório de testes e atualize a matriz de compatibilidade com status `tested`, `partially tested`, `untested` ou `unsupported`.
>
> ### Critérios de aceitação
> - Todos os testes unitários e de integração não destrutivos passam.
> - Cenários de VM são reportados com evidência e logs.
> - Nenhum cenário não executado aparece como aprovado.
> - Dual boot preserva artefatos Windows.
> - Falhas simuladas não deixam mounts ou transações órfãos sem diagnóstico.
> - Documentação de compatibilidade reflete exatamente os testes executados.
>
> ### Fora do escopo
> Novos bootloaders, migração de bootloader, lançamento público.
>
> Atualize `Docs/testing/`, `Docs/testing/HARDWARE_MATRIX.md`, `Docs/testing/FAILURE_INJECTION.md`, `Docs/testing/TEST_FIXTURES.md`, threat model e `Docs/ai/CURRENT_STATUS.md`. Não inicie a Fase 10.

## Prompt 9B — Agente 2 (30%, paralelo)

> Você é o Agente 2 da Fase 9 do CachyOS Boot Rescue. Leia as regras globais e os requisitos/critério de aceitação da Fase 9 neste documento; use os cenários e formato de evidência fornecidos pelo Agente 1.
>
> Crie e execute cenários isolados de compatibilidade em fixtures/VMs descartáveis para layouts GRUB/Limine, EFI, filesystems, Btrfs e dual boot. Registre ambiente, evidências, comandos e limitações. Nunca use discos físicos do host. Atualize somente os artefatos de matriz de compatibilidade atribuídos; não altere funcionalidades do produto e não marque cenários não executados como aprovados.
>
> Se não houver ambiente seguro disponível, registre o cenário como não executado e informe o motivo. Entregue artefatos, evidências, comandos e resultados ao Agente 1. Não atualize o status global, não declare a fase concluída e não inicie a Fase 10.

---

# Fase 10 — Empacotamento Pacman, releases e preparação para AUR

**Pré-requisito:** fases 1–9 concluídas e suíte de testes aprovada.

**Resultado:** pacote Arch instalável com `pacman -U`, build reproduzível, release no GitHub e estrutura preparada para submissão futura ao AUR.

### Divisão entre agentes (70% / 30%)
- **Agente 1 — 70% (líder):** implementar PKGBUILD e empacotamento, auditar dependências/caminhos/permissões, validar build reproduzível e instalação/atualização/remoção em ambiente descartável, e integrar o workflow de release.
- **Agente 2 — 30% (paralelo):** preparar a documentação de empacotamento e release, instruções de instalação/remoção e checklist de revisão AUR, sem publicar no AUR nem alterar o PKGBUILD/workflow.
- **Integração:** o Agente 1 fornece os metadados e caminhos finais do pacote; revisa a documentação do Agente 2, integra os artefatos e executa a validação final antes de declarar a fase concluída.

## Prompt 10A — Agente 1 (70%, líder)

> Você é o Agente 1 e lidera a Fase 10 do CachyOS Boot Rescue. Antes de começar, confirme que as fases anteriores estão concluídas e que a matriz de compatibilidade e testes foi atualizada. Implemente e valide o empacotamento, PKGBUILD e workflow de release; forneça ao Agente 2 os metadados e caminhos finais. Integre a documentação entregue por ele. A publicação no AUR permanece manual e não está autorizada.
>
> ### Objetivo
> Preparar distribuição oficial como pacote Arch Linux `.pkg.tar.zst`, instalável via `sudo pacman -U`, e preparar um PKGBUILD adequado para futura publicação no AUR.
>
> ### Passo a passo
> 1. Defina metadados finais: nome, versão, release, arquitetura, licença, mantenedor e URLs.
> 2. Audite dependências runtime e build. Declare somente dependências necessárias; bootloaders opcionais não devem ser dependências obrigatórias se o programa puder detectar sua ausência.
> 3. Crie PKGBUILD seguindo padrões atuais do Arch, com fontes verificadas por checksum ou commit/tag imutável.
> 4. Use build reproduzível quando possível e `cargo build --release --locked`; não baixe dependências dinamicamente durante a compilação sem declarar e verificar as fontes.
> 5. Instale binário, documentação, licença, exemplos e arquivos de configuração em caminhos FHS apropriados.
> 6. Defina diretórios de log e dados com permissões corretas. Não crie arquivos de log no momento da instalação se não forem necessários.
> 7. Crie arquivos `.install` somente se houver uma tarefa real de instalação/atualização que não possa ser resolvida pelos metadados do pacote.
> 8. Verifique que instalação e remoção não executam reparos, não montam discos e não alteram entradas UEFI.
> 9. Teste instalação, atualização e remoção em rootfs/VM descartável usando pacman.
> 10. Execute verificações de pacote, como `namcap`, quando disponíveis, e corrija problemas relevantes.
> 11. Crie workflow de release que compile, teste, gere pacote e publique checksums e notas de versão.
> 12. Revise e integre a documentação de instalação com `sudo pacman -U arquivo.pkg.tar.zst`, remoção com `pacman -R`, atualização e dependências entregue pelo Agente 2.
> 13. Prepare diretório/artefatos necessários para AUR, mas não publique no AUR sem decisão explícita do mantenedor.
> 14. Faça revisão final de README, documentação de usuário, política de privacidade, limitações e matriz de compatibilidade.
>
> ### Critérios de aceitação
> - Pacote `.pkg.tar.zst` é gerado e instalado em ambiente Arch/CachyOS descartável.
> - Binário funciona após instalação.
> - Dependências são corretas e mínimas.
> - Instalar/remover o pacote não altera discos, EFI ou NVRAM.
> - Build e testes são executados no CI.
> - Release inclui versão, checksum, instruções e limitações.
> - PKGBUILD está pronto para revisão AUR, mas publicação permanece manual.
>
> ### Fora do escopo
> Publicação automática no AUR, repositório próprio com assinatura de pacotes e suporte a distribuições não Arch.
>
> Atualize `Docs/ai/CURRENT_STATUS.md` e o changelog do projeto; revise e integre `Docs/development/PACKAGING.md`, `Docs/development/RELEASES.md` e `Docs/README.md` entregues pelo Agente 2. Entregue relatório final do projeto.

## Prompt 10B — Agente 2 (30%, paralelo)

> Você é o Agente 2 da Fase 10 do CachyOS Boot Rescue. Leia as regras globais e os requisitos/critério de aceitação da Fase 10 neste documento. Use os metadados e caminhos finais do pacote fornecidos pelo Agente 1.
>
> Prepare `Docs/development/PACKAGING.md`, `Docs/development/RELEASES.md` e as instruções de instalação/atualização/remoção com pacman e checklist de revisão AUR em `Docs/README.md`. Não altere PKGBUILD, workflow, dependências ou arquivos instalados pelo pacote. Não publique no AUR nem execute instalação em sistema real.
>
> Entregue os documentos, comandos de validação e resultados ao Agente 1 para revisão e integração. Não atualize o status global, não declare a fase concluída e não inicie trabalho fora desta fase.

---

# Checklist de conclusão por fase

Copie esta lista para o acompanhamento do projeto:

- [ ] Fase 1 — Fundação
- [ ] Fase 2 — Logging e relatórios
- [ ] Fase 3 — Discovery
- [ ] Fase 4 — Storage e Chroot
- [ ] Fase 5 — Interface TUI
- [ ] Fase 6 — Diagnóstico
- [ ] Fase 7 — Backup e rollback
- [ ] Fase 8 — Reparadores GRUB e Limine
- [ ] Fase 9 — Testes e segurança
- [ ] Fase 10 — Pacman e releases

## Regra de ouro do projeto

**Não avançar por intenção; avançar por evidência.**

Uma fase não está pronta porque o agente escreveu o código ou afirmou que terminou. Ela está pronta quando compila, passa nos testes definidos, atualiza a documentação e demonstra os critérios de aceitação.

O agente seguinte deve conseguir continuar somente lendo o repositório, sem depender da conversa anterior.
