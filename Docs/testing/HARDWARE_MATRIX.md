# Matriz de Hardware e Compatibilidade

## Visão Geral

Esta documenta os cenários de hardware testados, suportados e conhecidos para o CachyOS Boot Rescue. A Fase 3 foca em **descoberta somente leitura** - todos os testes usam fixtures JSON sintéticos, **nunca hardware real**.

## Cenários Testados (Fixtures)

| Fixture | Descrição | Discos | Partições | ESPs | Linux Install | Notas |
|---------|-----------|--------|-----------|------|---------------|-------|
| `sata_single_disk.json` | SATA SSD único | 1 (sda) | 3 | 1 | 1 (ext4) | Layout padrão Fedora/Ubuntu |
| `nvme_single_disk.json` | NVMe único | 1 (nvme0n1) | 3 | 1 | 1 (btrfs) | systemd-boot, subvolumes |
| `multiple_esps.json` | Dual boot + backup | 2 (sda, sdb) | 6 | 3 | 1 (ext4) | 2 Linux ESPs + 1 Windows ESP |
| `no_labels.json` | Sem labels | 1 (sda) | 2 | 1 | 1 (ext4) | UUIDs presentes, labels None |
| `removable_disks.json` | USB + SD card | 3 (sda, sdb, sdc) | 4 | 3 | 1 (ext4) | Interno + USB GPT + SD MBR |
| `incomplete_data.json` | Dados parciais | 2 (sda, loop0) | 3 | 1 | 1 (ext4) | sda1 sem fstype/uuid, loop device |
| `luks_encrypted.json` | LUKS + dm-crypt | 2 (nvme0n1, dm-0) | 3 | 1 | 1 (btrfs) | LUKS root, device mapper |

## Cobertura de Tipos de Disco

| Tipo | Testado | Fixture | Notas |
|------|---------|---------|-------|
| SATA SSD | ✅ | `sata_single_disk` | Rotação 0 |
| NVMe | ✅ | `nvme_single_disk`, `luks_encrypted` | Namespace nvme0n1pX |
| USB Flash | ✅ | `removable_disks` | Removível=true, GPT |
| SD Card | ✅ | `removable_disks` | Removível=true, MBR (dos) |
| Loop Device | ✅ | `incomplete_data` | squashfs, read-only |
| Device Mapper (dm-crypt) | ✅ | `luks_encrypted` | Tipo `crypt`, parent=nvme0n1p2 |
| LVM | ❌ | - | Não testado na Fase 3 |
| RAID (mdadm) | ❌ | - | Não testado na Fase 3 |

## Cobertura de Tabelas de Partições

| Tipo | Testado | Fixture | Notas |
|------|---------|---------|-------|
| GPT | ✅ | Todas exceto `removable_disks` (sdc) | `pttype: "gpt"` |
| MBR (DOS) | ✅ | `removable_disks` (sdc) | `pttype: "dos"`, parttype hex (ex: `0x0c`) |

## Cobertura de Filesystems

| Filesystem | Testado | Como | Notas |
|------------|---------|------|-------|
| ext4 | ✅ | Múltiplas | Root, home, data |
| btrfs | ✅ | `nvme_single_disk`, `luks_encrypted` | Com subvolumes (`subvol=@`) |
| xfs | ❌ | - | Não nas fixtures |
| ext3/ext2 | ❌ | - | Não nas fixtures |
| vfat/fat32 | ✅ | Todas (ESPs) | ESPs, USB |
| fat16/fat12 | ✅ | Testes unitários | `test_identify_esp_fat16_fat32_vfat` |
| ntfs | ✅ | `multiple_esps` | Windows partition |
| swap | ✅ | Testes unitários | `test_identify_from_metadata` |
| crypto_LUKS | ✅ | `luks_encrypted` | Com subsystem ext4/btrfs |
| lvm2_member | ❌ | - | Não testado |
| squashfs | ✅ | `incomplete_data` (loop0) | Classificado como Unknown |

## Cobertura de Partições Especiais

| Tipo | Testado | Fixture | GUID/Type |
|------|---------|---------|-----------|
| ESP (EFI System) | ✅ | Todas | `c12a7328-f81f-11d2-ba4b-00a0c93ec93b` |
| Linux Filesystem | ✅ | Múltiplas | `0fc63daf-8483-4772-8e79-3d69d8477de4` |
| Linux Swap | ✅ | Testes unitários | `0657fd6d-a4ab-43c4-84e5-0933c84b4f4f` |
| Microsoft Basic Data | ✅ | `multiple_esps` | `ebd0a0a2-b9e5-4433-87c0-68b6b72699c7` |
| Windows Recovery | ❌ | - | `de94bba4-06d1-4d40-a16a-bfd50179d6ac` |
| BIOS Boot | ❌ | - | `21686148-6449-6e6f-744e-656564454649` |
| Linux RAID | ❌ | - | `a19d880f-05fc-4d3b-a006-743f0f84911e` |
| Linux LVM | ❌ | - | `e6d6d379-f507-44c2-a23c-238f2a3df928` |
| MBR 0x0c (FAT32 LBA) | ✅ | `removable_disks` (sdc1) | `parttype: "0x0c"` |

## Cobertura de Cenários de Boot

| Cenário | Testado | Fixture | Notas |
|---------|---------|---------|-------|
| UEFI + ESP em `/boot/efi` | ✅ | `sata_single_disk` | Padrão Fedora/Ubuntu |
| UEFI + ESP em `/efi` | ✅ | `nvme_single_disk` | Padrão Arch/CachyOS |
| UEFI + ESP em `/boot` | ✅ | Testes unitários | systemd-boot unificado |
| UEFI + Múltiplas ESPs | ✅ | `multiple_esps` | 3 ESPs, ordenadas por confiança |
| BIOS Legacy (MBR) | ✅ | `removable_disks` (sdc) | MBR, sem ESP |
| LUKS Full Disk Encryption | ✅ | `luks_encrypted` | ESP clara + root criptografado |
| Btrfs Subvolumes | ✅ | `nvme_single_disk` | `@`, `@home` |
| Disco Removível (USB) | ✅ | `removable_disks` | `removable: true` |
| Sem Labels | ✅ | `no_labels` | UUIDs apenas |
| Dados Incompletos | ✅ | `incomplete_data` | fstype=null, uuid=null |

## Matriz de Confiança ESP

| Combinação | Confiança | Classificação |
|------------|-----------|---------------|
| Tipo EFI + FAT + `/boot/efi` + label EFI + tamanho OK | 100 | Certeza alta |
| Tipo EFI + FAT + `/efi` | 95 | Certeza alta |
| Tipo EFI + FAT (não montada) | 75 | Provável |
| Tipo EFI + não-FAT | 45 | Baixa (filesystem errado) |
| Não-EFI + FAT + label EFI + tamanho | 45 | Possível |
| Apenas tipo EFI | 40 | Baixa |
| Apenas FAT + tamanho | 35 | Muito baixa |

## Cenários NÃO Testados (Fase 3)

| Cenário | Status | Fase Prevista |
|---------|--------|---------------|
| LVM sobre LUKS | ❌ Não testado | Fase 4+ (montagem) |
| RAID mdadm (root) | ❌ Não testado | Fase 4+ |
| ZFS root | ❌ Não testado | Fora de escopo inicial |
| Secure Boot ativo | ❌ Não testado | Fase 6 (diagnóstico) |
| Hybrid MBR/GPT | ❌ Não testado | Raro |
| Apple APFS | ❌ Não testado | Fora de escopo |
| Múltiplos discos NVMe | ❌ Não testado | Fase 4+ |
| Disco com setor 4K nativo | ❌ Não testado | Phase 4+ |
| eMMC / SD card boot | ⚠️ Parcial | `removable_disks` tem SD card |
| Raspberry Pi (no ESP padrão) | ❌ Não testado | Fora de escopo |

## Estratégia de Testes

### Fixtures Sintéticos
- **Nunca** usa hardware real do desenvolvedor
- JSONs baseados em saídas reais de `lsblk --json` e `blkid -o json`
- Cobrem casos de borda: dados ausentes, múltiplas ESPs, criptografia

### Testes Unitários
- Parsing de JSON (lsblk, blkid)
- Classificação de tipos (filesystem, partição, device)
- Identificação ESP (confiança, ordenação)
- Detecção Linux (montado, metadados, swap exclusion)
- Detecção UEFI (estados válidos)
- **Timeout e limite de saída** (`test_execute_with_timeout_*`)
- **Valores ausentes removable/read_only** (`test_parse_removable_*`)
- **Proteção fstab_content serialização** (`test_*_fstab_content_not_serialized`)
- **Busca recursiva de partições** (`test_identify_*_recursive`)

### Testes de Integração
- Fluxo completo `run_discovery()` com fixtures
- Enriquecimento blkid → lsblk
- Deduplicação Linux candidates (montado > metadados)
- Serialização `DiscoveryReport`

### Testes de Contrato (`lsblk_contract.rs`)
- Parsing exato de campos obrigatórios
- Preservação de `None` para dados ausentes
- Múltiplas ESPs preservadas (não selecionadas)
- Dispositivos removíveis detectados

## Execução de Testes

```bash
# Todos os testes
cargo test --all-targets

# Apenas descoberta
cargo test discovery

# Com output detalhado
cargo test -- --nocapture

# Testes específicos
cargo test test_identify_esp
cargo test test_identify_from_metadata
cargo test test_execute_with_timeout
cargo test test_parse_removable
cargo test test_.*_recursive
cargo test test_.*_fstab_content
```

## Limitações Conhecidas dos Testes

1. **Não testa** execução real de `lsblk`/`blkid` (mockado via fixtures)
2. **Testa** timeouts de comandos via `sleep` em testes unitários (`test_execute_with_timeout_kills_on_timeout`)
3. **Testa** limites de saída reais via `printf` em testes unitários (`test_execute_with_timeout_output_limit`)
4. **Não testa** detecção UEFI em hardware real (mockado via `/sys/firmware/efi` check)
5. **Não testa** espaço livre em ESP (retorna `None` na fase read-only)
6. **Não valida** contra `efibootmgr` real (fase futura)

## Rastreabilidade para Fases Futuras

| Fase | Dependência da Fase 3 | Validação Necessária |
|------|----------------------|---------------------|
| 4 (Storage/Chroot) | `DiscoveryReport` com devices, ESPs, Linux candidates | Montagem baseada em `BlockDevice.path` e `mountpoint` |
| 5 (TUI) | `format_discovery_summary()`, `EspCandidate.confidence` | Seleção visual de candidato |
| 6 (Diagnóstico) | `LinuxInstallationCandidate.kernel_paths`, `fstab_content` | Verificação bootloader, kernels |
| 7 (Backup/Transação) | `DiscoveryDiagnostic` para rollback | Snapshot pré-reparo |
| 8 (Reparo GRUB/Limine) | `EspCandidate.mountpoint`, `UefiState` | Instalação no device correto |