# Modelo de Privilégio - CachyOS Boot Rescue

## Princípios Fundamentais

### 1. Privilégio Mínimo
- Operações de leitura (descoberta, diagnóstico) **não requerem root**
- Operações de escrita (montagem, chroot, reparação) **requerem root**
- Separação clara entre fases read-only e read-write

### 2. Elevação Explícita
- **Nenhuma elevação implícita** - usuário deve invocar com `sudo` ou `doas`
- Programa **não faz `sudo` interno** em nenhum momento
- Verifica privilégios no início e falha se insuficientes

### 3. Confirmação Explícita
- Toda operação mutável requer **confirmação interativa**
- Confirmação genérica da sessão **não** autoriza operações futuras
- Cada reparação (GRUB, initramfs, etc.) tem sua própria confirmação

## Operações por Nível de Privilégio

### Sem Privilégios (Usuário Comum)

| Operação | Descrição |
|----------|-----------|
| Descoberta de hardware | `lsblk --json`, `blkid -o json` |
| Detecção UEFI/BIOS | Leitura `/sys/firmware/efi` |
| Identificação ESP | Por tipo de partição GPT + filesystem FAT |
| Detecção instalações Linux | Montadas + metadados (sem montar) |
| Parse fstab | Leitura arquivo (se legível) |
| Validação plano | Verificação lógica |
| Exportação relatórios | Arquivos locais |
| Logging | Arquivos em diretório do usuário |

### Com Root (sudo/doas)

| Operação | Descrição | Confirmação |
|----------|-----------|-------------|
| Montagem de partições | `mount` com confinamento | Sim (plano completo) |
| Desmontagem | `umount` (ordem inversa) | Automática (cleanup) |
| Pseudo-filesystems chroot | `mount --rbind` | Sim (prepare) |
| Execução chroot | `chroot` + comandos | Sim (por comando/operação) |
| GRUB install | `grub-install` | Sim (por bootloader) |
| GRUB config | `grub-mkconfig` | Sim |
| systemd-boot | `bootctl install` | Sim |
| Limine | `limine-install` | Sim |
| Initramfs | `mkinitcpio -P` / `dracut` | Sim |
| LUKS close | `cryptsetup luksClose` | Automática (cleanup) |

## Verificação de Privilégios

### No Início
```rust
fn check_privileges() -> Result<()> {
    if !is_root() && requires_root_operation() {
        return Err(Error::permission_denied(
            "Esta operação requer privilégios de root. Execute com sudo."
        ));
    }
    Ok(())
}
```

### Durante Execução
- Cada `Command::new("mount")` verifica erro de permissão
- Falha de permissão → erro claro com sugestão de `sudo`

## Modelo de Confirmação

### Níveis de Confirmação

1. **Confirmação de Sessão** - Inicia fluxo com operações privilegiadas
2. **Confirmação de Plano** - Aprova plano de montagem completo
3. **Confirmação de Reparação** - Por operação (GRUB, initramfs, etc.)

### Fluxo de Confirmação

```
Usuário seleciona instalação
       ↓
Exibe plano de montagem (read-only)
       ↓
Confirma: "Montar estas partições?" → Sim/Não
       ↓
Executa montagem (root)
       ↓
Prepara chroot (root)
       ↓
Exibe opções de reparação
       ↓
Seleciona: "Reinstalar GRUB"
       ↓
Confirma: "Executar grub-install em /dev/sda?" → Sim/Não
       ↓
Executa no chroot (root)
       ↓
Valida resultado
       ↓
Limpeza automática
```

### Não Confirmação Genérica
- "Sim para tudo" **não existe**
- Cada operação mutável tem sua confirmação
- Usuário pode cancelar a qualquer momento

## Auditoria e Logs

### O que é Logado (Sanitizado)
- Operações executadas (comando binário, não args sensíveis)
- Timestamps, duração, exit codes
- Montagens/desmontagens (device, mount point)
- Erros e warnings

### O que NÃO é Logado
- Senhas, tokens, chaves
- Argumentos sensíveis de comandos
- Conteúdo de arquivos privados
- Variáveis de ambiente completas

### Relatórios Exportados
- Sanitizados automaticamente
- `raw_line` do fstab omitido
- Senhas LUKS nunca incluídas

## Separação de Fases

### Fase Read-Only (Descoberta/Diagnóstico)
```
1. lsblk --json
2. blkid -o json -p
3. efibootmgr -v (somente leitura)
4. cat /etc/fstab (se acessível)
5. cat /etc/os-release
6. find /boot -name "vmlinuz*"
```
**Pode rodar como usuário comum**

### Fase Read-Write (Reparação)
```
1. mount (com confinamento)
2. mount --rbind (pseudo-fs)
3. chroot + comandos
4. grub-install / grub-mkconfig
5. mkinitcpio / dracut
6. umount (ordem inversa)
```
**Requer root + confirmação explícita**

## Implementação no Código

### Verificação de Root
```rust
pub fn requires_root() -> bool {
    !is_root_operation_pending()
}

fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}
```

### Execução sem Sudo Interno
```rust
// CORRETO: executa direto, falha se sem permissão
Command::new("mount").args([...]).output()?

// ERRADO: não faz isso
Command::new("sudo").args(["mount", ...]).output()?
```

### Confirmação Interativa
```rust
pub fn confirm_operation(description: &str) -> Result<bool> {
    println!("{}", description);
    print!("Confirmar? [s/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().eq_ignore_ascii_case("s"))
}
```

## Testes de Privilégio

### Testes Unitários (Sem Root)
- Parsing, validação, estruturas
- Mocks para comandos externos
- Diretórios temporários

### Testes de Integração (Com Root - Opcional)
- VMs descartáveis (CI separado)
- Montagem real em loop devices
- Chroot real
- **Não rodam em CI padrão**

## Referência Rápida

| Comando | Privilégio | Confirmação |
|---------|------------|-------------|
| `lsblk`, `blkid` | Usuário | Não |
| `mount` | Root | Sim (plano) |
| `umount` | Root | Auto (cleanup) |
| `chroot` | Root | Sim (prepare) |
| `grub-install` | Root | Sim (operação) |
| `grub-mkconfig` | Root | Sim |
| `mkinitcpio` | Root | Sim |
| `cryptsetup luksClose` | Root | Auto |
| `efibootmgr` (write) | Root | Sim (futuro) |

## Exceções

### LUKS Unlock
- **Não implementado** - requer design seguro
- Se implementado: confirmação específica + zeroização

### Debug/Desenvolvimento
- `--dry-run` mostra operações sem executar
- `--no-confirm` **não existe** por design