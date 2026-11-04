# CURRENT_STATUS.md
# Estado atual do projeto Boot-repair (versão IA)

**Última atualização:** 03/11/2026

## 1. Situação geral

O projeto tem as **Fases 1, 2, 3 e 4 implementadas e testadas**. Todas as verificações de qualidade passam (`cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets`, `cargo build --locked`).

<badge>Estado: Fases 1-4 concluídas com correções de segurança</badge>

## 2. Fases

| Fase | Escopo canônico | Estado | Evidência/observação |
|---|---|---|---|
| 1 | Fundação do crate Rust | **Concluída** | Cargo package, módulos `error`, `config`, `cli`, `app`, configuração TOML, CLI, tratamento de erros, logging `tracing`. 33 testes unitários. `cargo test --all-targets` passa. |
| 2 | Logging, sessões e relatórios sanitizados | **Concluída** | `tracing` + `tracing-subscriber` + `tracing-appender`, sessões com UUID, rotação diária + limite de arquivos, sanitização de segredos (senhas, tokens, chaves, LUKS, env vars, usernames, MACs, UUIDs), exportação tar.gz sanitizada. Testes com dados sintéticos contendo segredos. |
| 3 | Descoberta somente leitura de hardware e instalações | **Concluída** | `lsblk --json` parsing robusto, enriquecimento `blkid`, identificação ESP (múltiplas candidatas, sem seleção arbitrária), detecção Linux (montadas + metadados, swap excluído), detecção UEFI/BIOS, classificação filesystems, valores ausentes = `None`, serial não serializado, timeouts/limites de saída, child parsing errors logged not discarded. 144 testes passam. |
| 4 | Montagem e gerenciamento de Chroot | **Concluída com correções de segurança** | Parser puro fstab, MountPlan, MountSession, DeviceResolver, Btrfs subvolumes, LUKS detection, ChrootManager com pseudo-filesystems, execução comandos com timeout/sanitização, validação ambiente. **Correções de segurança aplicadas:** confinamento de montagens, validação fstab estrita, MountSession transacional, ChrootManager não destrutivo, sanitização de saída, LUKS unlock não implementado. |
| 5 | TUI e fluxo do usuário | Planejada | Depende dos módulos anteriores. |
| 6 | Diagnóstico somente leitura de bootloaders | Planejada | Depende das Fases 3–5. |
| 7 | Backup, transações e rollback | Planejada | Testes somente com arquivos temporários/fixtures. |
| 8 | Reparadores GRUB, Limine e initramfs | Planejada | Operações mutáveis somente com salvaguardas e ambientes descartáveis. |
| 9 | Testes de integração, segurança e compatibilidade | Planejada | Cenários não executados devem permanecer explicitamente não testados. |
| 10 | Empacotamento Pacman e releases | Planejada | Publicação no AUR permanece manual. |

## 3. Correções de Segurança Fase 4 (Esta Sessão)

### 1. Confinamento de Montagens
- Mount points fstab (ex: `/boot/efi`) resolvidos via `resolved_mount_point(root_mount)` → `<root_mount>/boot/efi`
- Validação: path absoluto, sem `..`, não escapa da raiz

### 2. Validação Estrita fstab
- `noauto`: respeitado **antes** de resolver device → `SkippedNoauto` sempre
- `nofail`: device ausente → `SkippedNofailMissing` (entrada no plano, não erro)
- Ambiguidade: `SkippedAmbiguous` (entrada no plano, bloqueia montagem)
- Root fstab mismatch: compara path **e** UUID (warning se UUID difere)
- Filesystem desconhecido: erro bloqueante
- Mount point duplicado/relativo/`..`: erro bloqueante
- Dependências ausentes/ciclos: erro bloqueante (topological sort)

### 3. MountSession Transacional
- `unmount_all` → `Result<UnmountResult>`: `unmounted`, `failed`, `skipped`
- `finalize`: só `finalized=true` se `unmount_all` OK
- `rollback(original_error)`: limpa sessão, loga falhas, não oculta erro original
- **Drop**: NÃO chama `unmount_all` automaticamente (evita `umount -f` silencioso)

### 4. ChrootManager Não Destrutivo
- `prepare`: rollback automático se falha em qualquer pseudo-fs
- Detecta mounts pré-existentes via `findmnt`; não registra como próprios
- **Não copia `/etc/resolv.conf`** do host (pode sobrescrever dados usuário)
- `execute`: timeout com drenagem completa de pipes, limites independentes stdout/stderr, sanitização via `logging::sanitize_output`
- `cleanup`: desmonta apenas próprios, preserva pré-existentes, retorna erro se falha
- `validate_chroot_environment`: verifica ferramentas no filesystem target, **sem chroot real**

### 5. Sanitização de Saída
- `logging::sanitize_output()` aplicado em `ChrootManager::execute` antes de log
- Remove: senhas, tokens, chaves SSH, material LUKS, env vars, URLs com credenciais, MACs, UUIDs, usernames

### 6. LUKS Unlock Não Implementado
- `unlock_luks_interactive` retorna `Err("não implementado de forma segura")`
- Documenta requisitos: zeroização (`zeroize` crate), stdin seguro (`rpassword`), tratamento `write_all` falha, timeout/cancelamento, sem logs/args/env
- Apenas detecção disponível: `detect_luks_devices`, `probe_luks_device`, `close_luks`

### 7. Proteção de Dados Sensíveis
- `FstabEntry.raw_line`: `#[serde(skip_serializing)]`
- `FstabOptions.raw`: `#[serde(skip_serializing)]`
- `FstabEntry.to_safe_projection()` → `FstabEntrySafe` (sem raw_line)
- `MountPlan.to_safe_projection()` → `MountPlanSafe` (HashMaps convertidos para Vec<(String,...)> para serialização JSON)
- Testes verificam ausência de `raw_line`, `raw`, senhas em JSON serializado

## 4. Testes
- **Total: 221 testes passando**, confirmado por `cargo test --all-targets`
- testes unitários: 155
- testes de integração e contrato: 66
- **quality checks**: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets`, `cargo build --locked` - **TODOS PASSAM**

## 5. Documentação Nova/Atualizada
- `Docs/modules/storage.md` - Documentação completa do módulo storage
- `Docs/modules/chroot.md` - Documentação completa do módulo chroot
- `Docs/domain/BTRFS.md` - Subvolumes, detecção, montagem
- `Docs/domain/FSTAB_CRYPTTAB.md` - Parser fstab, LUKS detection/design
- `Docs/domain/CHROOT.md` - Preparação, execução, limpeza, segurança
- `Docs/architecture/PRIVILEGE_MODEL.md` - Modelo de privilégio, confirmações, auditoria

## 6. Próximos Passos
Fase 5 (TUI e fluxo do usuário) - aguarda autorização. Não iniciada.

---

**Diretriz:** este arquivo deve representar o estado real do projeto, não o estado desejado.
