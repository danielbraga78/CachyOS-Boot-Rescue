# Phase 4 Security and Reliability Fixes - Implementation Plan

## Overview
Fix critical security and reliability issues in storage and chroot modules without advancing to Phase 5.

## Issues to Fix (in order)

### 1. Mount Confinement (storage/mount_plan.rs, storage/mount_session.rs, storage/mod.rs)
- **Problem**: fstab mount points are logical paths passed directly to `mount` as absolute destinations
- **Fix**: Resolve all mount points under `MountSession.root_mount` (e.g., `/boot/efi` → `<root_mount>/boot/efi`)
- **Validate**: Reject relative paths, `..`, paths escaping root, ambiguous destinations
- **Check**: Root mount and required mounts match selected installation candidate
- **Don't unmount**: Pre-existing mounts not created by session

### 2. Fstab Options and Ambiguities (storage/fstab.rs, storage/mount_plan.rs)
- **noauto**: Don't auto-mount; record as ignored/optional in plan visibly
- **nofail**: Preserve meaning without treating resolution failure as device found
- **Ambiguity**: Block entry/plan on ambiguous identifiers, never choose arbitrarily
- **Root fstab entry**: Verify matches selected installation device; block on conflict
- **Unknown filesystems**: Reject/flag before execution
- **Duplicate/conflicting mountpoints**: Detect and reject
- **Topological ordering**: Detect missing deps/cycles, fail explicitly

### 3. MountSession Reliability (storage/mount_session.rs)
- **unmount_all**: Return error if any unmount fails; preserve inventory of active mounts
- **finalize**: Only mark finalized when cleanup completed
- **Rollback**: Clean up session mounts only; don't unmount pre-existing; log rollback failures without hiding original error
- **Idempotency**: Avoid inconsistent states in repeated finalize/unmount/rollback
- **No force unmount**: Don't use `umount -f` in Drop/rollback without explicit auth

### 4. ChrootManager Transactional & Non-Destructive (chroot/mod.rs)
- **prepare failure**: Unmount only pseudo-fs mounted by this call (reverse order); preserve pre-existing
- **Pre-existing detection**: Detect before mounting; never register as manager's
- **resolv.conf**: Remove copy from host to target; use non-destructive approach if needed
- **cleanup**: Report failures; maintain coherent state; allow retry
- **Tests**: Inject/mock external calls; no real pseudo-fs mounts

### 5. Timeout, Capture, Sanitization (chroot/mod.rs)
- **Timeout flow**: Process must remain accessible for kill; no indefinite blocking on wait/drain
- **Drain pipes**: Continue draining even when truncating; independent limits; unambiguous timeout reporting
- **Sanitization**: Don't log args, env vars, output with secrets; apply existing sanitizer before logging
- **No shell**: Execute binary/args directly
- **Tests**: Timeout termination, output over limit, spawn/wait failures, sanitization with mocks

### 6. LUKS Flow Review (storage/luks.rs)
- **Password handling**: Don't ignore write_all failures; don't proceed as if unlock succeeded
- **Password hygiene**: Not logged, not in visible args, not in logs, not kept longer than needed; explicit failure/cancellation handling
- **cryptsetup options**: Verify compatibility; if not provably safe, keep only detection, document unlock as not implemented
- **No real devices**: Don't execute cryptsetup against real devices in tests

### 7. Sensitive Data Protection (storage/fstab.rs, storage/mount_plan.rs, storage/mount_session.rs)
- **FstabEntry.raw_line**: Don't serialize by default (already has serde but check)
- **Sensitive options**: Don't serialize raw options with potential secrets
- **MountPlan serialization**: Use safe explicit projection or omit sensitive fields
- **Tests**: Verify raw_line, sensitive options, passwords, args don't appear in JSON/logs/reports

## Implementation Order

1. **storage/fstab.rs** - Fix fstab parsing, options handling, device resolution
2. **storage/mount_plan.rs** - Fix mount confinement, topological ordering, validation
3. **storage/mount_session.rs** - Fix reliability, rollback, finalize
4. **storage/mod.rs** - Integration fixes
5. **chroot/mod.rs** - Transactional prepare, timeout, sanitization, no resolv.conf copy
6. **storage/luks.rs** - LUKS flow review, password handling
7. **Tests** - Add failing tests for each fix before implementing

## Testing Strategy
- All tests use mocks/temp dirs - no real mounts, chroot, LUKS, or host disks
- Add tests that FAIL before fixes, PASS after
- Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets`, `cargo build --locked`

## Documentation Updates (after verified behavior)
- Docs/modules/storage.md
- Docs/modules/chroot.md
- Docs/domain/BTRFS.md
- Docs/domain/FSTAB_CRYPTTAB.md
- Docs/domain/CHROOT.md
- Docs/architecture/PRIVILEGE_MODEL.md
- CURRENT_STATUS.md and Docs/ai/CURRENT_STATUS.md

## Success Criteria
- All 198+ existing tests still pass
- New tests for each fix pass
- Quality checks pass
- No Phase 5 work started
- No commits made