# Partições EFI (EFI System Partition - ESP)

## Definição

A **EFI System Partition (ESP)** é uma partição especial definida pela especificação UEFI que contém os carregadores de boot (bootloaders) e aplicações UEFI. É **obrigatória** para boot UEFI.

## Identificação

### Critérios Obrigatórios (UEFI Spec)
1. **Tipo de partição GPT**: `C12A7328-F81F-11D2-BA4B-00A0C93EC93B` (EFI System Partition)
2. **Filesystem**: FAT32 (VFAT) - **único filesystem obrigatório** pela spec

### Critérios de Confiança (Implementação)

| Critério | Pontos | Descrição |
|----------|--------|-----------|
| Tipo GPT EFI System | 40 | `parttype` ou `parttypename` corresponde ao GUID EFI |
| Filesystem FAT/VFAT | 30 | `fstype` é `vfat`, `fat32`, `fat16`, `fat12` |
| PARTUUID EFI | 20 | PARTUUID começa com `c12a7328` (fallback) |
| Label sugestivo | 10 | Label contém "efi", "boot", "esp" |
| Montada em `/boot/efi` ou `/efi` | 15 | Mountpoint padrão de ESP |
| Montada em `/boot` | 5 | Pode ser ESP unificada |
| Tamanho 50MB - 2GB | 5 | Tamanho típico de ESP |

**Confiança mínima para ser candidata**: > 0
**Ordenação**: Maior confiança primeiro

## Múltiplas ESPs

A spec UEFI **permite múltiplas ESPs**. Cenários comuns:

### 1. Dual Boot Linux + Windows
```
/dev/nvme0n1p1 - ESP Windows (100MB, label "SYSTEM")
/dev/nvme0n1p2 - ESP Linux (512MB, label "EFI")
/dev/nvme0n1p3 - Windows (NTFS)
/dev/nvme0n1p4 - Linux root (ext4/btrfs)
```

### 2. ESP de Backup/Recuperação
```
/dev/sda1 - ESP principal (/boot/efi)
/dev/sdb1 - ESP backup (não montada)
```

### 3. Múltiplas Distros Linux
```
/dev/nvme0n1p1 - ESP compartilhada
/dev/nvme0n1p2 - Arch root
/dev/nvme0n1p3 - Fedora root
```

**Regra**: Apresentar **todas** como candidatas. Não escolher arbitrariamente.

## Montagem e Pontos de Montagem

### Layouts Comuns

| Layout | ESP Mount | `/boot` Mount | Comum em |
|--------|-----------|---------------|----------|
| Separado | `/boot/efi` | `/boot` (ext4) | Fedora, Ubuntu, Debian |
| Unificado systemd-boot | `/efi` | (mesmo que ESP) | Arch, CachyOS |
| ESP como `/boot` | `/boot` | (mesmo que ESP) | systemd-boot unificado |
| BIOS Legacy | N/A | `/boot` (ext4) | MBR only |

### Detecção de Mountpoint
- Usa `mountpoint` (principal) e `mountpoints` (lista do lsblk)
- **Não usa** `mount_options` (contém opções como `rw,relatime`, não paths)
- Verifica: `/boot/efi`, `/efi`, `/boot`

## Tamanho Recomendado

| Cenário | Tamanho Mínimo | Recomendado |
|---------|----------------|-------------|
| Mínimo UEFI | 100 MB | - |
| Single OS Linux | 300 MB | 512 MB |
| Dual Boot (Win+Linux) | 500 MB | 1 GB |
| Múltiplas distros/kernels | 1 GB | 2 GB |

**Limite prático**: 2 GB (acima disso, provavelmente não é ESP)

## Filesystem

### Obrigatório: FAT32 (VFAT)
- Spec UEFI exige FAT32 (ou FAT16/12 para partições pequenas)
- `mkfs.fat -F 32` para criar
- Labels: "EFI", "ESP", "BOOT", "SYSTEM" (Windows)

### Não Suportados como ESP
- ext4, btrfs, xfs, NTFS, exFAT
- Partição com tipo EFI mas filesystem errado = **não é ESP válida**

## Partição BIOS Boot (Não é ESP)

Para boot BIOS/GPT (não UEFI):
- Tipo GPT: `21686148-6449-6E6F-744E-656564454649` (BIOS Boot)
- Tamanho: 1-2 MB
- **Sem filesystem** (código bruto do GRUB)
- Flag: `bios_grub` no parted

## Detecção no Código

```rust
// Em esp.rs
pub fn analyze_esp_candidate(device: &BlockDevice) -> Option<EspCandidate> {
    // Só considera partições
    if device.device_type != DeviceType::Partition { return None; }

    let mut confidence = 0;
    let mut reasons = Vec::new();

    // 1. Tipo GPT EFI System
    if device.partition_type.as_ref().is_some_and(|p| p.is_efi_system()) {
        confidence += 40;
        reasons.push("Tipo de partição GPT: EFI System Partition".into());
    }

    // 2. Filesystem FAT
    if device.filesystem.as_ref().is_some_and(|f| f.is_fat()) {
        confidence += 30;
        reasons.push(format!("Filesystem: {:?}", device.filesystem.unwrap()));
    }

    // ... demais critérios

    if confidence == 0 { return None; }
    Some(EspCandidate { confidence: confidence.min(100), reasons, ... })
}
```

## Dados Preservados

Para cada ESP candidata, preserva:
- `device.path` - Caminho do dispositivo (`/dev/nvme0n1p1`)
- `device.uuid` - UUID do filesystem FAT
- `device.partuuid` - PARTUUID da partição GPT
- `device.label` - Label do filesystem
- `device.size` - Tamanho em bytes
- `device.mountpoint` - Ponto de montagem atual (se montada)
- `device.mountpoints` - Lista de mountpoints (lsblk)
- `filesystem.fstype` - Tipo (Vfat/Fat32/etc)
- `confidence` - 0-100
- `reasons` - Lista de strings explicativas

## Segurança

- **Não montar** ESPs na descoberta (somente leitura)
- **Não modificar** conteúdo da ESP
- **Preservar** `EFI/Microsoft/` (Windows Boot Manager)
- **Não sobrescrever** `EFI/BOOT/BOOTX64.EFI` (fallback genérico)
- **Reportar** espaço livre apenas se já montada (fase futura)

## Testes (Fixtures)

| Fixture | ESPs Esperadas | Notas |
|---------|----------------|-------|
| `sata_single_disk.json` | 1 (`/dev/sda1`) | Montada em `/boot/efi` |
| `nvme_single_disk.json` | 1 (`/dev/nvme0n1p1`) | Montada em `/efi` |
| `multiple_esps.json` | 3 (`sda1`, `sda3`, `sdb1`) | Duas Linux + uma Windows |
| `removable_disks.json` | 3 (`sda1`, `sdb1`, `sdc1`) | Inclui USB e SD card |
| `incomplete_data.json` | 1 (`sda1`) | Tipo EFI mas sem fstype |
| `no_labels.json` | 1 (`sda1`) | Sem label, mas tipo EFI + FAT |