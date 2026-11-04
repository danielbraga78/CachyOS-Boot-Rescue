# FSTAB e CRYPTTAB - Parser e Criptografia

## FSTAB - Parser

### Formato Suportado

```
# <device>        <mountpoint>  <fstype>  <options>  <dump>  <pass>
UUID=xxxx-xxxx    /             ext4      defaults   0       1
PARTUUID=yyyy     /boot         vfat      rw,noauto  0       2
LABEL=HOME        /home         btrfs     subvol=@home 0      2
/dev/sda3         /mnt/data     xfs       nofail     0       0
```

### Especificadores de Dispositivo

| Tipo | Exemplo | Descrição |
|------|---------|-----------|
| UUID | `UUID=xxxx-xxxx` | UUID do filesystem |
| PARTUUID | `PARTUUID=yyyy-yyyy` | PARTUUID da partição GPT |
| LABEL | `LABEL=MY_LABEL` | Label do filesystem |
| Path | `/dev/sda1` | Caminho direto do dispositivo |

### Opções de Montagem

| Opção | Significado | Tratamento |
|-------|-------------|------------|
| `noauto` | Não montar automaticamente | `SkippedNoauto` - não montado |
| `nofail` | Não falhar se dispositivo ausente | `SkippedNofailMissing` se ausente |
| `ro` | Read-only | Passado para mount |
| `rw` | Read-write | Passado para mount |
| `subvol=@` | Subvolume Btrfs por nome | `subvol=@` |
| `subvolid=256` | Subvolume Btrfs por ID | `subvolid=256` |
| `relatime`, `discard`, etc. | Outras | Passadas como estão |

### Decodificação de Escapes

Suporta escapes octais em campos:
- `\040` → espaço
- `\011` → tab
- `\012` → newline
- `\134` → backslash
- `\043` → #

### Validação

- 6 campos obrigatórios por linha
- Mount point deve ser absoluto (exceto `none` para swap)
- Filesystem type deve ser conhecido
- Dump e pass devem ser números válidos (0-9)
- Linhas comentadas (`#`) e vazias ignoradas

### Resultado do Parse

```rust
pub struct FstabEntry {
    pub device_spec: FstabDeviceSpec,  // UUID, PARTUUID, LABEL, Path
    pub mount_point: PathBuf,
    pub fs_type: FilesystemType,
    pub options: FstabOptions,         // parsed flags + other
    pub dump: u8,
    pub pass: u8,
    pub raw_line: String,              // NÃO serializado
    pub line_number: usize,
}
```

### Resolução de Dispositivos

`resolve_device_specs_detailed()` retorna `DeviceResolution`:
- `resolved`: HashMap<FstabDeviceSpec, PathBuf> - mapeamentos únicos
- `missing`: Vec<FstabDeviceSpec> - não encontrados
- `ambiguous`: HashMap<FstabDeviceSpec, Vec<PathBuf>> - múltiplos matches

**Nunca escolhe arbitrariamente** - ambiguidade bloqueia a entrada.

## CRYPTTAB - Criptografia (LUKS)

### Detecção LUKS

Dispositivo é LUKS se:
- `filesystem == FilesystemType::Luks`
- `blkid` retorna `TYPE="crypto_LUKS"`

### Informações do Container LUKS

Via `cryptsetup luksDump`:
- UUID do container LUKS
- Cipher, key size, hash
- Key slots ativos

### Device Mapper

Se já descriptografado:
- Path: `/dev/mapper/luks-<partuuid>`
- Filesystem interno acessível via device mapper
- `blkid` no mapper path revela inner filesystem type/UUID/label

### Desbloqueio Interativo

**NÃO IMPLEMENTADO DE FORMA SEGURA** - Apenas detecção disponível.

Requisitos para implementação segura:
1. **Zeroização de memória** - usar crate `zeroize` para senha
2. **Leitura segura** - `rpassword` ou similar (sem echo)
3. **stdin para cryptsetup** - `--key-file=-` para evitar args
4. **Tratamento de falhas** - `write_all` pode falhar (broken pipe)
5. **Timeout e cancelamento** - Ctrl+C deve limpar estado
6. **Sem logs** - senha nunca em logs, args, env vars

### Fluxo Seguro (Requisitos)

O desbloqueio permanece indisponível: os itens acima são requisitos de projeto,
não uma implementação nem um exemplo executável. Não copie um fluxo de
desbloqueio para produção sem tratar falhas de escrita no stdin, timeout,
cancelamento e limpeza da senha em todos os caminhos de saída.

### Fechamento Seguro

```rust
cryptsetup luksClose <mapper_name>
```

Ignora erro "not found" (já fechado).

## Integração no Plano de Montagem

### Entradas fstab com LUKS

Se fstab referencia UUID de container LUKS:
1. Resolução encontra device path (ex: `/dev/sda2`)
2. Se já aberto (`/dev/mapper/luks-xxx` existe), usa mapper path
3. Se não aberto, entrada marcada como `SkippedNofailMissing` ou `SkippedAmbiguous`
4. **Não tenta desbloquear automaticamente**

### Validação de Raiz LUKS

Se root é LUKS:
- `candidate.root_device.path` = device path (ex: `/dev/sda2`)
- fstab root entry deve resolver para mesmo path
- Se mapper ativo, usa mapper path para montagem

## Segurança

### Dados Sensíveis

- `FstabEntry.raw_line` - **NÃO serializado**
- `FstabOptions.raw` - **NÃO serializado**
- Senhas LUKS - **NUNCA** em logs, args, env vars
- `MountPlan` serialização usa projeção segura (`MountPlanSafe`)

### Validação Estrita

- `noauto` → entrada no plano mas `SkippedNoauto`
- `nofail` + ausente → `SkippedNofailMissing` (não erro)
- Ambiguidade → `SkippedAmbiguous` (bloqueia montagem)
- Path relativo/`..` → erro bloqueante
- Filesystem desconhecido → erro bloqueante
- Mount point duplicado → erro bloqueante
- Dependência ausente/ciclo → erro bloqueante

## Testes

Todos os testes usam:
- Fixtures JSON para dispositivos simulados
- Strings fstab sintéticas
- **Nenhum dispositivo real, cryptsetup real, ou montagem real**

## Limitações

1. **LUKS unlock não implementado** - apenas detecção
2. **crypttab não parseado** - apenas fstab (crypttab seria para unlock automático)
3. **LVM não suportado** - fora do escopo Phase 4
4. **Keyfiles não suportados** - apenas senha interativa (quando implementado)