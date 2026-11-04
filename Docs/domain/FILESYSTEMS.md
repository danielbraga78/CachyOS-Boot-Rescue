# Filesystems Suportados

## Visão Geral

O módulo de descoberta classifica filesystems detectados via `lsblk` e `blkid` em tipos conhecidos, preservando informações de UUID, label e versão quando disponíveis.

## Enum `FilesystemType`

```rust
pub enum FilesystemType {
    // Linux nativos
    Btrfs,
    Ext4,
    Ext3,
    Ext2,
    Xfs,

    // FAT/VFAT (ESP)
    Fat32,
    Fat16,
    Fat12,
    Vfat,     // Genérico quando lsblk retorna "vfat"

    // Windows
    Ntfs,

    // Especial
    Swap,
    Luks,     // crypto_LUKS
    Lvm2,     // LVM2 PV

    // Optical
    Iso9660,
    Udf,

    // Desconhecido
    Unknown,
}
```

## Classificação por Origem

### 1. `lsblk` (fonte primária)
Campo `fstype` do JSON. Mapeamento:

| lsblk fstype | FilesystemType | Notas |
|--------------|----------------|-------|
| `btrfs` | `Btrfs` | |
| `ext4` | `Ext4` | |
| `ext3` | `Ext3` | |
| `ext2` | `Ext2` | |
| `xfs` | `Xfs` | |
| `vfat` | `Vfat` | Genérico |
| `fat32` | `Fat32` | Específico |
| `fat16` | `Fat16` | |
| `fat12` | `Fat12` | |
| `ntfs` | `Ntfs` | |
| `swap` | `Swap` | |
| `crypto_LUKS` | `Luks` | Dispositivo criptografado |
| `lvm2_member` / `lvm2` | `Lvm2` | Physical Volume |
| `iso9660` | `Iso9660` | CD/DVD |
| `udf` | `Udf` | DVD/Blu-ray |
| `squashfs` | `Unknown` | Não mapeado (read-only) |
| *outro* | `Unknown` | Preserva string original? |

### 2. `blkid` (enriquecimento)
Campo `type` do JSON. Usado **apenas** para preencher lacunas do lsblk.

| blkid type | FilesystemType |
|------------|----------------|
| `btrfs` | `Btrfs` |
| `ext4` | `Ext4` |
| `vfat` | `Vfat` |
| `ntfs` | `Ntfs` |
| `swap` | `Swap` |
| `crypto_LUKS` | `Luks` |
| `lvm2_member` | `Lvm2` |

**Regra**: Não sobrescreve valor já presente do lsblk.

## Propriedades por Tipo

### Linux Nativos (`is_linux_native()`)
```rust
pub fn is_linux_native(&self) -> bool {
    matches!(self, Btrfs | Ext4 | Ext3 | Ext2 | Xfs)
}
```
- Usados para partição root (`/`)
- Candidatos a instalação Linux
- Suportam permissões POSIX, xattrs, ACLs

### FAT/VFAT (`is_fat()`)
```rust
pub fn is_fat(&self) -> bool {
    matches!(self, Vfat | Fat32 | Fat16 | Fat12)
}
```
- **Obrigatório para ESP** (UEFI spec)
- Não suporta permissões Unix completas
- Case-insensitive

### Criptografia (`is_crypto()`)
```rust
pub fn is_crypto(&self) -> bool {
    matches!(self, Luks)
}
```
- Indica dispositivo LUKS
- Requer descriptografia para acessar filesystem interno
- `blkid` mostra subsystem (ex: ext4 dentro do LUKS)

## Dados Preservados

Para cada filesystem detectado (`FilesystemInfo`):

```rust
pub struct FilesystemInfo {
    pub fstype: FilesystemType,      // Tipo classificado
    pub uuid: Option<String>,        // UUID do filesystem
    pub label: Option<String>,       // Label/volume name
    pub partuuid: Option<String>,    // PARTUUID (se partição)
    pub version: Option<String>,     // Versão (ex: "1.0" para ext4)
    pub subsystems_uuid: Option<String>, // UUID interno (LUKS->ext4)
    pub subsystems_type: Option<String>, // Tipo interno (LUKS->"ext4")
}
```

## Detecção de Subsistemas (LUKS/LVM)

```json
// blkid -o json -p /dev/nvme0n1p2
{
  "name": "/dev/nvme0n1p2",
  "type": "crypto_LUKS",
  "uuid": "luks-uuid-...",
  "subsystems": [{
    "uuid": "inner-ext4-uuid",
    "type": "ext4"
  }]
}
```

Resulta em:
- `fstype: Luks`
- `subsystems_uuid: Some("inner-ext4-uuid")`
- `subsystems_type: Some("ext4")`

Dispositivo descriptografado (`dm-0`) aparece separadamente no lsblk:
```json
{
  "name": "dm-0",
  "type": "crypt",
  "fstype": "btrfs",
  "mountpoint": "/"
}
```

## Casos Especiais

### Btrfs com Subvolumes
- lsblk mostra `mountoptions: ["subvol=@", "subvol=@home"]`
- Descoberta detecta subvolume raiz via `/proc/self/mountinfo`
- Procura diretórios `@`, `@root`, `root` no mountpoint

### Partições sem Filesystem (Raw)
- `fstype: null` no lsblk
- `filesystem: None` no BlockDevice
- `partition_type` pode indicar propósito (ex: LinuxSwap, BIOS Boot)
- **Não assumir** filesystem baseado só no tipo de partição

### Loop Devices
- `type: "loop"` no lsblk
- `fstype: "squashfs"` comum (ISO live)
- Classificado como `Unknown` (não é alvo de boot repair)

## Tabela de Compatibilidade Boot

| Filesystem | Pode ser `/` (root) | Pode ser `/boot` | Pode ser ESP | Notas |
|------------|---------------------|------------------|--------------|-------|
| ext4 | ✅ | ✅ | ❌ | Padrão amplo |
| btrfs | ✅ | ✅ | ❌ | Subvolumes, snapshots |
| xfs | ✅ | ✅ | ❌ | Sem shrink |
| ext3 | ✅ | ✅ | ❌ | Legacy |
| ext2 | ✅ | ✅ | ❌ | Sem journal |
| vfat/fat32 | ❌ | ⚠️ | ✅ | ESP obrigatório |
| ntfs | ❌ | ❌ | ❌ | Windows only |
| swap | ❌ | ❌ | ❌ | Não é instalação |
| luks | ❌* | ❌* | ❌ | Precisa descriptografar |
| lvm2 | ❌* | ❌* | ❌ | Precisa ativar VG |

*O filesystem *dentro* do LUKS/LVM pode ser root/boot.

## Heurísticas de Detecção

### Candidato a Root (`/`)
1. Filesystem nativo Linux (`is_linux_native()`)
2. Montado em `/` (mountpoint)
3. Tem `/etc/os-release` e/ou `/etc/fstab`

### Candidato a `/boot` ou ESP
1. Filesystem FAT (`is_fat()`)
2. Tipo partição EFI System (GPT)
3. Montado em `/boot`, `/boot/efi`, `/efi`

### Swap
1. `fstype == "swap"` OU `partition_type == LinuxSwap`
2. **NUNCA** considerado instalação Linux candidata

## Testes (Fixtures)

| Fixture | Filesystems Detectados |
|---------|------------------------|
| `sata_single_disk.json` | vfat (ESP), ext4 (root), ext4 (home) |
| `nvme_single_disk.json` | vfat (ESP), btrfs (root+subvol), btrfs (home+subvol) |
| `multiple_esps.json` | vfat (3 ESPs), ext4 (root), ntfs (Windows), ext4 (data) |
| `luks_encrypted.json` | vfat (ESP), crypto_LUKS, btrfs (dm-0) |
| `incomplete_data.json` | null (sda1), ext4 (sda2), squashfs (loop0) |
| `no_labels.json` | vfat (sem label), ext4 (sem label) |

## Limitações Conhecidas

1. **lsblk pode não detectar** filesystem em partição não montada sem blkid
2. **blkid -p** não probeia partições não montadas por padrão (precisa permissões)
3. **Versão do filesystem** nem sempre disponível (lsblk não expõe, blkid -p limitado)
4. **ExFAT** não mapeado explicitamente (classificado como `Unknown`)
5. **ZFS** não mapeado (não é kernel nativo Linux padrão)
6. **Partição extended MBR** (tipo 0x05/0x0f) não tem filesystem próprio