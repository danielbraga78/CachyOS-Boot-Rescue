# UEFI (Unified Extensible Firmware Interface)

## Visão Geral

UEFI é a especificação moderna de interface de firmware que substituiu o BIOS Legacy. O CachyOS Boot Rescue deve detectar e lidar corretamente com ambos os modos.

## Detecção de Modo

### Verificação Primária: `/sys/firmware/efi`

```rust
pub fn detect_uefi_state() -> Result<UefiState> {
    let path = Path::new("/sys/firmware/efi");
    match path.try_exists() {
        Ok(true) => {
            if path.is_dir() {
                match std::fs::read_dir(path) {
                    Ok(_) => Ok(UefiState::Uefi),
                    Err(e) => Ok(UefiState::Unknown), // Diretório existe mas não legível
                }
            } else {
                Ok(UefiState::Unknown) // Existe mas não é diretório
            }
        }
        Ok(false) => Ok(UefiState::BiosLegacy),
        Err(e) => Ok(UefiState::Unknown), // Erro de acesso
    }
}
```

### Estados Possíveis

| Estado | Descrição |
|--------|-----------|
| `Uefi` | Sistema bootou em modo UEFI, variáveis EFI acessíveis |
| `BiosLegacy` | Sistema bootou em modo BIOS/Legacy (CSM) |
| `Unknown` | Não foi possível determinar (erro de permissão, container, etc) |

## Variáveis EFI

Em modo UEFI, as variáveis estão em `/sys/firmware/efi/efivars/`. O acesso é **somente leitura** na fase de descoberta.

```rust
pub fn list_efi_variables() -> Result<Vec<String>> {
    // Lista entradas em /sys/firmware/efi/efivars/
    // Não modifica nenhuma variável
}
```

## Boot Entries (NVRAM)

Entradas de boot UEFI são gerenciadas via `efibootmgr`. Na fase de descoberta:
- Usa `efibootmgr -v` **somente leitura**
- Preserva todas as entradas existentes
- Sinaliza ambiguidades (múltiplas entradas para mesmo OS)
- Não cria, remove ou modifica entradas

## ESP (EFI System Partition)

### Requisitos UEFI Spec
- Tipo de partição GPT: `C12A7328-F81F-11D2-BA4B-00A0C93EC93B`
- Filesystem: FAT32 (VFAT) - **obrigatório** pela spec UEFI
- Tamanho mínimo: 100 MB (recomendado 512 MB+)
- Flags: `boot`, `esp` no GPT

### Múltiplas ESPs
- Spec permite múltiplas ESPs
- Windows cria sua própria ESP
- Linux pode ter ESP separada ou usar `/boot` como ESP
- **Não escolher arbitrariamente** - apresentar todas como candidatas

## Diferenças Críticas: UEFI vs BIOS Legacy

| Aspecto | UEFI | BIOS Legacy |
|---------|------|-------------|
| Particionamento | GPT (obrigatório) | MBR (MSDOS) |
| Bootloader | Arquivo .efi na ESP | Código no MBR + gap |
| ESP | Obrigatória (FAT32) | Não existe |
| `/boot` | Pode ser separado ou na ESP | Partição separada comum |
| Secure Boot | Suportado | Não suportado |
| Variáveis de boot | NVRAM (efivars) | Não aplicável |

## Detecção de Layouts Comuns

### 1. ESP em `/boot/efi` (Padrão Fedora/Ubuntu)
```
/dev/nvme0n1p1 -> /boot/efi (vfat, ESP)
/dev/nvme0n1p2 -> / (ext4/btrfs)
```

### 2. ESP em `/efi` (Padrão Arch/CachyOS com systemd-boot)
```
/dev/nvme0n1p1 -> /efi (vfat, ESP)
/dev/nvme0n1p2 -> / (btrfs, subvol=@)
```

### 3. `/boot` na ESP (systemd-boot unificado)
```
/dev/nvme0n1p1 -> /boot (vfat, ESP)
/dev/nvme0n1p2 -> / (ext4/btrfs)
```

### 4. BIOS Legacy com partição BIOS Boot
```
/dev/sda1 -> BIOS Boot (1MB, sem filesystem, tipo 21686148-6449-6E6F-744E-656564454649)
/dev/sda2 -> /boot (ext4)
/dev/sda3 -> / (ext4/btrfs)
```

## Secure Boot

- **Não modificar** chaves ou políticas na fase de descoberta
- Detectar status: `mokutil --sb-state` (somente leitura)
- Reportar se ativo para diagnóstico de reparo futuro
- Reparo de Secure Boot requer fase dedicada (fora do escopo Fase 3)

## Considerações de Segurança

1. **Nunca escrever** em `/sys/firmware/efi/efivars/`
2. **Não executar** `efibootmgr` com flags de escrita (`-c`, `-d`, `-o`, `-B`)
3. **Preservar** entradas Windows (`EFI/Microsoft/Boot/bootmgfw.efi`)
4. **Não assumir** que ESP montada em `/boot/efi` é a "correta"
5. **Reportar** todas as ESPs candidatas com nível de confiança