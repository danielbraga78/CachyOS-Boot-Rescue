# Módulo Chroot - Gerenciamento de Ambiente Chroot

## Visão Geral

O módulo `chroot` implementa a preparação e execução segura de comandos dentro de um ambiente chroot para reparação de boot. Ele gerencia:

- Montagem de pseudo-filesystems necessários (`/dev`, `/proc`, `/sys`, `/run`)
- Execução de comandos com captura de saída, timeout e sanitização
- Validação do ambiente alvo
- Limpeza transacional com rollback em falha

## Componentes Principais

### 1. ChrootManager (`chroot/mod.rs`)

Gerencia o ciclo de vida do ambiente chroot:

```rust
let mut chroot = ChrootManager::new(root_mount);
chroot.prepare()?;  // Monta pseudo-filesystems

let result = chroot.execute(&["grub-install", "/dev/sda"], None)?;

chroot.cleanup()?;  // Desmonta pseudo-filesystems
```

#### Preparação Transacional (`prepare()`)

- Verifica se `root_mount` existe e é diretório
- Monta pseudo-filesystems com `--rbind`:
  - `/dev` → `<root>/dev` (devtmpfs)
  - `/proc` → `<root>/proc` (proc)
  - `/sys` → `<root>/sys` (sysfs)
  - `/run` → `<root>/run` (tmpfs)
- **Detecta mounts pré-existentes** e não os registra como próprios
- **Rollback automático** se falhar em qualquer etapa
- **NÃO copia `/etc/resolv.conf`** do host (pode sobrescrever dados do usuário)

#### Execução Segura (`execute()`)

- **Sem shell**: Executa binário e argumentos diretamente via `chroot`
- **Timeout configurável** (padrão 60s)
- **Limites de saída independentes** para stdout/stderr (padrão 1MB cada)
- **Drenagem completa de pipes** mesmo ao truncar (evita deadlock)
- **Sanitização automática** de segredos na saída antes de logar
- **Retorna**: `exit_code`, `stdout`, `stderr`, `duration`, `timed_out`, `truncated`

#### Limpeza (`cleanup()`)

- Desmonta em ordem inversa com `umount -R`
- **Preserva mounts pré-existentes** (não desmonta)
- Retorna erro se alguma desmontagem falhar
- Permite nova tentativa

### 2. Configuração (`ChrootCommandConfig`)

```rust
pub struct ChrootCommandConfig {
    pub timeout_secs: u64,           // default: 60
    pub max_stdout_bytes: usize,     // default: 1MB
    pub max_stderr_bytes: usize,     // default: 1MB
    pub env_vars: HashMap<String, String>,
    pub working_dir: Option<PathBuf>,
}
```

### 3. Validação de Ambiente (`validate_chroot_environment`)

Verifica ferramentas essenciais **sem preparar chroot completo**:
- `bash`, `mount`, `cryptsetup`
- `grub-install`, `limine-install`
- `mkinitcpio`, `dracut`
- Retorna `ChrootValidation` com flags por ferramenta

## Fluxo de Uso

```rust
// 1. Valida ambiente (opcional, rápido)
let validation = validate_chroot_environment(&root_mount)?;
if !validation.can_repair_grub() {
    return Err("Ambiente não suporta reparação GRUB");
}

// 2. Cria gerenciador
let mut chroot = ChrootManager::with_session(root_mount, mount_session);

// 3. Prepara (transacional - rollback em falha)
chroot.prepare()?;

// 4. Executa comandos
let result = chroot.execute(&["grub-mkconfig", "-o", "/boot/grub/grub.cfg"], None)?;
if result.exit_code != 0 {
    return Err("grub-mkconfig falhou");
}

// 5. Limpa (sempre, mesmo em erro)
chroot.cleanup()?;
```

## Segurança

### Transacional
- `prepare()` falha → rollback automático dos pseudo-fs montados
- `execute()` timeout → mata processo, drena pipes, retorna `timed_out=true`
- `cleanup()` falha → retorna erro, permite retry
- **Drop** tenta limpeza mas loga warning se não finalizado

### Não Destrutivo
- **Não copia `/etc/resolv.conf`** do host para o target
- **Não usa shell** - execução direta de binário/argumentos
- **Detecta mounts pré-existentes** antes de montar
- **Não desmonta recursos que não criou**

### Sanitização
- Saída (stdout/stderr) passada por `logging::sanitize_output()` antes de logar
- Remove: senhas, tokens, chaves SSH, material LUKS, env vars sensíveis, URLs com credenciais, MAC addresses, UUIDs, usernames
- Logs de execução mostram apenas `binary` e `arg_count`, não argumentos

## Pseudo-Filesystems

| Source | Target | Tipo | Opções |
|--------|--------|------|--------|
| `/dev` | `<root>/dev` | devtmpfs | bind, rbind |
| `/proc` | `<root>/proc` | proc | bind, rbind |
| `/sys` | `<root>/sys` | sysfs | bind, rbind |
| `/run` | `<root>/run` | tmpfs | bind, rbind |

## Testes

Todos os testes usam:
- Diretórios temporários
- **Não montam pseudo-filesystems reais**
- **Não executam chroot real**
- Validação verifica existência de arquivos no filesystem target
- `ChrootCommandResult` testado para estrutura

## Limitações Conhecidas

1. **Timeout de processo**: Após timeout, o processo é morto mas a thread de wait pode não conseguir fazer join limpo
2. **Resolução DNS**: Não configura resolv.conf; use bind mount externo se necessário
3. **Privilégios**: Requer root para montar pseudo-filesystems e executar chroot
3. **Não testado em CI**: Montagem real requer privilégios e não roda em CI padrão