# Btrfs - Subvolumes e Montagem

## Visão Geral

Btrfs (B-tree filesystem) suporta subvolumes, que são namespaces de filesystem independentes dentro de um único filesystem. Isso é comum em instalações Arch/CachyOS onde:
- `@` ou `@root` = subvolume raiz do sistema
- `@home` = subvolume para `/home`
- `@var`, `@srv`, etc. = outros subvolumes

## Detecção de Subvolumes

### Listagem (`btrfs subvolume list`)

```bash
btrfs subvolume list -p /mnt
```

Saída:
```
ID 256 gen 123 top level 5 path @
ID 257 gen 456 top level 5 path @home
ID 258 gen 789 top level 256 path @/var
```

Parsing:
- `ID <id>` - ID do subvolume
- `top level <parent_id>` - ID do pai (5 = filesystem root)
- `path <path>` - Caminho relativo ao topo do filesystem

### Detecção de Subvolume Montado

Via `/proc/self/mountinfo`:
```
123 456 0:20 / /mnt rw,relatime - btrfs /dev/sda2 rw,subvol=@
```

Extraí `subvol=@` ou `subvolid=256`.

## Subvolume Raiz Padrão

Ordem de preferência:
1. `@` (padrão mais comum)
2. `@root` (alternativa)
3. `@_` (variação)
4. Primeiro subvolume com `parent_id = 5` (top level) e `id != 5`

## Montagem de Subvolumes

### Opções de Montagem

```bash
# Por nome
mount -o subvol=@ /dev/sda2 /mnt

# Por ID
mount -o subvolid=256 /dev/sda2 /mnt
```

### fstab

```
UUID=xxxx  /  btrfs  rw,relatime,subvol=@  0  1
UUID=xxxx  /home  btrfs  rw,relatime,subvol=@home  0  2
```

Parser suporta:
- `subvol=@name` → `subvol=@name`
- `subvolid=256` → `subvolid=256`

## Integração no Storage Module

### build_mount_plan

1. Detecta se root device é Btrfs
2. Se `candidate.is_btrfs_subvolume == true`:
   - Root entry: `is_btrfs_subvolume = true`, `subvolume = candidate.btrfs_subvolume`
   - Outras entradas no mesmo device: herdam subvolume
3. Entradas fstab com `subvol=`/`subvolid=`: usam valor do fstab

### Execução de Montagem

```rust
if entry.is_btrfs_subvolume && entry.subvolume.is_some() {
    cmd.arg("-o").arg(format!("{}subvol={}", current_opts, subvol));
}
```

## Detecção Automática

### find_default_root_subvolume

1. Lista subvolumes do filesystem montado
2. Procura `@`, `@root`, `@_`
3. Fallback: primeiro com `parent_id = 5`

### detect_mounted_subvolume

Lê `/proc/self/mountinfo` e extrai `subvol=` ou `subvolid=`.

## Considerações de Segurança

- **Não assume `@` automaticamente** - confirma o que existe
- **fstab tem precedência** sobre detecção automática
- **Validação**: Se fstab especifica subvol, usa esse valor
- **Montagem confinada**: Subvolume montado em `<root_mount>/path`

## Limitações

1. **Requer `btrfs` command** para listagem
2. **Requer mount ativo** para detectar subvolume atual
3. **Não modifica subvolumes** - apenas detecta e monta
4. **Snapshots não gerenciados** - fora do escopo do Phase 4