# CURRENT_STATUS.md
# Estado atual do projeto Boot-repair

**Última atualização:** 10/11/2026

## 1. Situação geral

O projeto tem as **Fases 1, 2, 3, 4 e 5 implementadas e validadas**. Todas as verificações de qualidade passam (`cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets`, `cargo build --locked`).

<badge>Estado: Fases 1-5 concluídas com estabilização da TUI e fluxo de operações</badge>

## 2. Fases

| Fase | Escopo canônico | Estado | Evidência/observação |
|---|---|---|---|
| 1 | Fundação do crate Rust | **Concluída** | Cargo package, módulos `error`, `config`, `cli`, `app`, configuração TOML, CLI, tratamento de erros, logging `tracing`. 33 testes unitários. `cargo test --all-targets` passa. |
| 2 | Logging, sessões e relatórios sanitizados | **Concluída** | `tracing` + `tracing-subscriber` + `tracing-appender`, sessões com UUID, rotação diária + limite de arquivos, sanitização de segredos (senhas, tokens, chaves, LUKS, env vars, usernames, MACs, UUIDs), exportação tar.gz sanitizada. Testes com dados sintéticos contendo segredos. |
| 3 | Descoberta somente leitura de hardware e instalações | **Concluída** | `lsblk --json` parsing robusto, enriquecimento `blkid`, identificação ESP (múltiplas candidatas, sem seleção arbitrária), detecção Linux (montadas + metadados, swap excluído), detecção UEFI/BIOS, classificação filesystems, valores ausentes = `None`, serial não serializado, timeouts/limites de saída, child parsing errors logged not discarded. 144 testes passam. |
| 4 | Montagem e gerenciamento de Chroot | **Concluída com correções de segurança** | Parser puro fstab, MountPlan, MountSession, DeviceResolver, Btrfs subvolumes, LUKS detection, ChrootManager com pseudo-filesystems, execução comandos com timeout/sanitização, validação ambiente. **Correções de segurança aplicadas:** confinamento de montagens, validação fstab estrita, MountSession transacional, ChrootManager não destrutivo, sanitização de saída, LUKS unlock não implementado. |
| 5 | TUI e fluxo do usuário | **Concluída** | Implementação da app centralizada, reducer puro, canais typed, TaskManager, wrappers de operations (discovery/mount/chroot), telas da TUI (welcome/discovery/install_select/mount_review/mount_progress/chroot/error), fluxo de navegação e cancelamento com validação por testes. |
| 6 | Diagnóstico somente leitura de bootloaders | Planejada | Depende das Fases 3–5. |
| 7 | Backup, transações e rollback | Planejada | Testes somente com arquivos temporários/fixtures. |
| 8 | Reparadores GRUB, Limine e initramfs | Planejada | Operações mutáveis somente com salvaguardas e ambientes descartáveis. |
| 9 | Testes de integração, segurança e compatibilidade | Planejada | Cenários não executados devem permanecer explicitamente não testados. |
| 10 | Empacotamento Pacman e releases | Planejada | Publicação no AUR permanece manual. |

## 3. Evidências de Validação (Fase 5)

### Código Implementado (`cachyos-boot-rescue/src/app/`, `src/runtime/`, `src/operations/`, `src/ui/`)
- `src/app/app.rs` - app loop TUI com terminal raw mode, evento de teclado/resize, reducer + runtime, setup/shutdown
- `src/app/reducer.rs` - transições puras de estado para discovery, mount, chroot, erro e progresso
- `src/app/state.rs` e `src/app/model.rs` - `AppState`, `SharedData`, `OperationState`, `MountStage`, `OpId`, `OpKind`
- `src/runtime/task_manager.rs` - gestão de tarefas assíncronas, concorrência e cancelamento
- `src/runtime/channels.rs` - canais `mpsc` tipados para `AppCommand`/`AppEvent`
- `src/operations/discovery.rs`, `mount.rs`, `chroot.rs` - wrappers assíncronos com eventos do pipeline
- `src/ui/screens/*` - telas de welcome, discovery, install select, mount review, progress, chroot e error
- `src/ui/layout.rs` - render principal com header/body/footer e delegação de telas

### Validação da Fase 5
- `cargo fmt` - PASS
- `cargo clippy --all-targets --all-features -- -D warnings` - PASS
- `cargo test --all-targets` - PASS
- `cargo build --locked` - PASS

### Testes relevantes da fase
- `tests/app_model_tests.rs`
- `tests/app_reducer_tests.rs`
- `tests/runtime_channels_tests.rs`
- `tests/task_manager_tests.rs`
- `tests/operations_discovery_tests.rs`
- `tests/operations_mount_tests.rs`
- `tests/operations_chroot_tests.rs`

## 4. Evidências de Validação (Fase 3)

### Código Implementado (`cachyos-boot-rescue/src/discovery/`)
- `types.rs` - Modelos tipados: `BlockDevice`, `Partition`, `Filesystem`, `EspCandidate`, `LinuxInstallationCandidate`, `DiscoveryReport`, `DeviceId`, `PartitionType`, `FilesystemType`, `UefiState`
- `lsblk.rs` - Adaptador `lsblk --json` com timeout 30s, limite 1MB, locale C, parsing recursivo, child errors logged
- `blkid.rs` - Adaptador `blkid -o json -p` para enriquecimento, não sobrescreve lsblk
- `esp.rs` - Identificação ESP por 7 critérios, confiança 0-100, múltiplas candidatas ordenadas, `mountpoints` field
- `linux.rs` - `identify_linux_installations()` (montadas) + `identify_from_metadata()` (não montadas), swap excluído, Btrfs subvolume detection
- `uefi.rs` - `detect_uefi_state()` via `/sys/firmware/efi`, 3 estados
- `mod.rs` - `run_discovery()` orquestra fluxo completo, retorna `DiscoveryReport`

### Testes (144 total)
- **Unitários (84)**: Parsing lsblk/blkid, classificação tipos, ESP confidence, Linux detection, UEFI states, logging sanitizer
- **Integração (54)**: Fluxo completo com 7 fixtures JSON (SATA, NVMe, múltiplas ESPs, sem labels, removíveis, incompletos, LUKS)
- **Contrato (6)**: `lsblk_contract.rs` - parsing exato, preservação de `None`, múltiplas ESPs, dispositivos removíveis

### Fixtures JSON (Hardware Fictício)
| Fixture | Cenário |
|---------|---------|
| `sata_single_disk.json` | SATA SSD, ESP `/boot/efi`, root ext4, home ext4 |
| `nvme_single_disk.json` | NVMe, ESP `/efi`, root btrfs `@`, home btrfs `@home` |
| `multiple_esps.json` | 2 discos, 3 ESPs (2 Linux + 1 Windows), NTFS |
| `no_labels.json` | Partições sem label, UUIDs presentes |
| `removable_disks.json` | Interno + USB GPT + SD card MBR |
| `incomplete_data.json` | Dados parciais (fstype=null), loop device squashfs |
| `luks_encrypted.json` | LUKS root, dm-0 descriptografado btrfs |

### Correções Implementadas Durante Fase 3
1. **Swap não é instalação candidata** - `PartitionType::is_linux_installation()` exclui `LinuxSwap`
2. **Valores ausentes = `None`** - `device_id`, `size`, `sector_size`, `logical_sector_size` são `Option`
3. **Child parsing errors logged** - `filter_map(...ok())` → coleta errors + `tracing::warn`
4. **ESP mountpoints corretos** - Usa `mountpoints` (lista lsblk) não `mount_options` (opções de montagem)
5. **Serial privado** - `#[serde(skip_serializing)]` em `BlockDevice.serial`
6. **Free space sem comando externo** - `estimate_free_space()` retorna `None` na fase read-only
7. **Imports limpos** - Removidos `unused_imports`, `unused_mut`, clippy warnings corrigidos
8. **Timeout e limite de saída** - `execute_with_timeout` mata processos no timeout e drena pipes
9. **Busca recursiva de dispositivos** - `identify_from_metadata` e `identify_linux_installations` recursivos
10. **Valores ausentes de removable/read_only** - `Option<bool>` para representar estado desconhecido
11. **fstab_content protegido** - `#[serde(skip_serializing)]` em `LinuxInstallationCandidate.fstab_content`

## 4. Evidências de Validação (Fase 4)

### Código Implementado (`cachyos-boot-rescue/src/storage/` e `src/chroot/`)
- `storage/fstab.rs` - Parser puro `/etc/fstab` com suporte a UUID, PARTUUID, LABEL, path, opções `nofail`, `noauto`, `subvol`/`subvolid`, **raw_line e raw options não serializados**, `device_spec_to_string()` para logging seguro
- `storage/mount_plan.rs` - `MountPlan` com validação estrita: dependências ausentes/ciclos, mount points duplicados, filesystem desconhecido, path relativo/`..`, root fstab mismatch (path + UUID), noauto respeitado independentemente de device, nofail preservado, ambiguidade bloqueia entrada, **confinamento via `resolved_mount_point(root_mount)`**, projeção segura `MountPlanSafe`
- `storage/mount_session.rs` - `MountSession` transacional: `unmount_all` retorna `UnmountResult` detalhado com falhas, `finalize` só marca se limpeza OK, `rollback` preserva erro original, **não usa `umount -f` automático**, idempotente, **confinamento via `resolved_mount_point`**
- `storage/device_resolver.rs` - Resolução UUID/PARTUUID/LABEL/path recursivamente, retorna `DeviceResolution` detalhado
- `storage/btrfs.rs` - Detecção subvolumes Btrfs, subvolume padrão (`@`, `@root`), opções `subvol=`/`subvolid=`
- `storage/luks.rs` - **Apenas detecção LUKS**; `unlock_luks_interactive` retorna erro explícito "não implementado de forma segura", `close_luks` seguro
- `storage/mod.rs` - `build_mount_plan()` integra fstab, dispositivos, candidato, ESPs com `root_mount` obrigatório
- `chroot/mod.rs` - `ChrootManager` **não destrutivo**: `prepare` com rollback em falha, detecta pseudo-fs pré-existentes, **não copia `/etc/resolv.conf`**, `execute` com timeout/drenagem/sanitização, `cleanup` preserva pré-existentes, validação sem chroot real

### Testes (221 total)
- **Unitários (155)**: testes das bibliotecas e módulos
- **Integração e contrato (66)**: 6 testes fstab, 54 testes de integração e 6 testes de contrato `lsblk`
- **Total: 221 testes passando**, confirmado por `cargo test --all-targets`

### Correções de Segurança Implementadas (Esta Sessão)

#### 1. Confinamento de Montagens
- Mount points fstab (ex: `/boot/efi`) resolvidos via `resolved_mount_point(root_mount)` → `<root_mount>/boot/efi`
- Validação: path absoluto, sem `..`, não escapa da raiz
- `execute_mount` usa ponto resolvido; `MountSession` registra path lógico

#### 2. Validação Estrita fstab
- `noauto`: respeitado **antes** de resolver device → `SkippedNoauto` sempre
- `nofail`: device ausente → `SkippedNofailMissing` (entrada no plano, não erro)
- Ambiguidade: `SkippedAmbiguous` (entrada no plano, bloqueia montagem)
- Root fstab mismatch: compara path **e** UUID (warning se UUID difere)
- Filesystem desconhecido: erro bloqueante
- Mount point duplicado/relativo/`..`: erro bloqueante
- Dependências ausentes/ciclos: erro bloqueante (topological sort)

#### 3. MountSession Transacional
- `unmount_all` → `Result<UnmountResult>`: `unmounted`, `failed`, `skipped`
- `finalize`: só `finalized=true` se `unmount_all` OK
- `rollback(original_error)`: limpa sessão, loga falhas, não oculta erro original
- **Drop**: NÃO chama `unmount_all` automaticamente (evita `umount -f` silencioso)
- Idempotente: registro duplicado ignorado

#### 4. ChrootManager Não Destrutivo
- `prepare`: rollback automático se falha em qualquer pseudo-fs
- Detecta mounts pré-existentes via `findmnt`; não registra como próprios
- **Não copia `/etc/resolv.conf`** do host (pode sobrescrever dados usuário)
- `execute`: timeout com drenagem completa de pipes, limites independentes stdout/stderr, sanitização via `logging::sanitize_output`, retorna `truncated` flag
- `cleanup`: desmonta apenas próprios, preserva pré-existentes, retorna erro se falha
- `validate_chroot_environment`: verifica ferramentas no filesystem target, **sem chroot real**

#### 5. Sanitização de Saída
- `logging::sanitize_output()` aplicado em `ChrootManager::execute` antes de log
- Remove: senhas, tokens, chaves SSH, material LUKS, env vars, URLs com credenciais, MACs, UUIDs, usernames
- Logs de execução mostram apenas `binary` + `arg_count`

#### 6. LUKS Unlock Não Implementado
- `unlock_luks_interactive` retorna `Err("não implementado de forma segura")`
- Documenta requisitos: zeroização (`zeroize` crate), stdin seguro (`rpassword`), tratamento `write_all` falha, timeout/cancelamento, sem logs/args/env
- Apenas detecção disponível: `detect_luks_devices`, `probe_luks_device`, `close_luks`

#### 7. Proteção de Dados Sensíveis
- `FstabEntry.raw_line`: `#[serde(skip_serializing)]`
- `FstabOptions.raw`: `#[serde(skip_serializing)]`
- `FstabEntry.to_safe_projection()` → `FstabEntrySafe` (sem raw_line)
- `MountPlan.to_safe_projection()` → `MountPlanSafe` (HashMaps convertidos para Vec<(String,...)> para serialização JSON)
- Testes verificam ausência de `raw_line`, `raw`, senhas em JSON serializado

## 5. Documentação Atualizada
- `Docs/modules/discovery.md` - Documentação completa do módulo
- `Docs/modules/storage.md` - **NOVA**: Documentação completa do módulo storage
- `Docs/modules/chroot.md` - **NOVA**: Documentação completa do módulo chroot
- `Docs/domain/UEFI.md` - Detecção UEFI/BIOS, variáveis EFI, boot entries, Secure Boot
- `Docs/domain/EFI_PARTITIONS.md` - Critérios ESP, múltiplas ESPs, layouts, tamanho, segurança
- `Docs/domain/FILESYSTEMS.md` - Classificação, propriedades, LUKS/LVM, compatibilidade boot
- `Docs/domain/BTRFS.md` - **NOVA**: Subvolumes, detecção, montagem
- `Docs/domain/FSTAB_CRYPTTAB.md` - **NOVA**: Parser fstab, LUKS detection/design
- `Docs/domain/CHROOT.md` - **NOVA**: Preparação, execução, limpeza, segurança
- `Docs/architecture/PRIVILEGE_MODEL.md` - **NOVA**: Modelo de privilégio, confirmações, auditoria
- `Docs/testing/HARDWARE_MATRIX.md` - Matriz de cobertura, cenários testados/não testados, rastreabilidade

## 6. Baseline de Qualidade
```bash
cargo fmt --check                    # PASS
cargo clippy --all-targets --all-features -- -D warnings  # PASS
cargo test --all-targets             # PASS (221 testes)
cargo build --locked                 # PASS
```

## 7. Bloqueios Atuais
Nenhum. Fase 4 concluída com todos os critérios de aceitação validados **incluindo correções de segurança**.

## 8. Próximos Passos
Iniciar Fase 6 (Diagnóstico somente leitura de bootloaders) quando autorizado. A Fase 5 já foi concluída com:
- App centralizada + reducer + runtime
- Fluxo de discovery → install select → mount review → progress → chroot → error
- Navegação por telas e cancelamento operacional
- Base de UI e widgets estáveis para extensão futura

## 9. Registro de alterações

| Data | Alteração |
|---|---|
| 02/11/2026 | Estruturação inicial dos documentos de governança e acompanhamento. |
| 03/11/2026 | Correções da Fase 3: timeout/output limit, busca recursiva, removable/read_only Option<bool>, fstab_content proteção, documentação alinhada. |
| 03/11/2026 | Implementação Fase 4: storage (fstab, mount_plan, mount_session, device_resolver, btrfs, luks), chroot (manager, command, validation), testes, CI. |
| 03/11/2026 | **Correções de Segurança Fase 4**: confinamento montagens, validação fstab estrita, MountSession transacional, ChrootManager não destrutivo, sanitização saída, LUKS unlock não implementado, proteção dados sensíveis. Total verificado: 221 testes. |
| 10/11/2026 | **Conclusão da Fase 5**: app centralizada, reducer, channels tipados, TaskManager, wrappers de operations, TUI com telas e fluxo discovery/mount/chroot/error validado com `cargo test --all-targets` + `cargo clippy --all-targets --all-features -- -D warnings`. |

---

**Diretriz:** este arquivo deve representar o estado real do projeto, não o estado desejado. Toda atualização deve ser baseada em alterações verificáveis no repositório.
