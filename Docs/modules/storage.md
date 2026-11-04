# Módulo Storage - Gerenciamento de Armazenamento e Montagem

## Visão Geral

O módulo `storage` implementa o gerenciamento seguro de montagem de sistemas de arquivos para recuperação de boot. Ele fornece:

- Parser puro para `/etc/fstab`
- Plano de montagem explícito com validação de dependências
- Sessão de montagem com rastreamento e rollback seguro
- Resolução de identificadores de dispositivo (UUID, PARTUUID, LABEL, path)
- Detecção e suporte a subvolumes Btrfs
- Detecção de dispositivos LUKS (sem desbloqueio inseguro)

## Componentes Principais

### 1. Parser FSTAB (`fstab.rs`)

Parser robusto para `/etc/fstab` que suporta:
- Especificadores de dispositivo: UUID, PARTUUID, LABEL, path direto
- Opções de montagem: `noauto`, `nofail`, `ro`, `rw`, `subvol`, `subvolid`
- Decodificação de escapes octais (\040, \011, etc.)
- Coleta de erros sem falhar o parse completo
- **Segurança**: `raw_line` e `raw` options não são serializados por padrão

### 2. Plano de Montagem (`mount_plan.rs`)

Define um plano explícito de montagem com:
- Entradas ordenadas topologicamente (raiz primeiro, filhos depois)
- Validação de dependências ausentes e ciclos
- Detecção de pontos de montagem duplicados/conflitantes
- Verificação de filesystem desconhecido
- Validação de ponto de montagem raiz do fstab vs instalação selecionada
- Status por entrada: `Planned`, `SkippedNoauto`, `SkippedNofailMissing`, `SkippedAmbiguous`, `AlreadyMounted`, `Failed`
- **Confinamento**: mount points lógicos (ex: `/boot/efi`) são resolvidos em tempo de execução para `<root_mount>/boot/efi`

### 3. Sessão de Montagem (`mount_session.rs`)

Gerencia o ciclo de vida das montagens:
- Rastreamento de montagens ativas com metadados
- Desmontagem em ordem inversa (filhos antes dos pais)
- `unmount_all` retorna erro detalhado se alguma falhar
- `finalize` só marca como finalizado se limpeza completa
- `rollback` para limpeza controlada em falha
- **Não usa `umount -f` automaticamente** em Drop/rollback
- Idempotência: registro duplicado é ignorado

### 4. Resolvedor de Dispositivos (`device_resolver.rs`)

Resolve identificadores para caminhos reais:
- Mapeia UUID, PARTUUID, LABEL, path para `/dev/...`
- Busca recursiva em dispositivos aninhados
- Retorna apenas mapeamentos não ambíguos
- `DeviceResolution` detalha: `resolved`, `missing`, `ambiguous`

### 5. Btrfs Subvolumes (`btrfs.rs`)

Utilitários para Btrfs:
- Lista subvolumes via `btrfs subvolume list`
- Detecta subvolume montado via `/proc/self/mountinfo`
- Encontra subvolume raiz padrão (@, @root, @_)
- Gera opções de montagem `subvol=` / `subvolid=`

### 6. LUKS (`luks.rs`)

**Apenas detecção** - desbloqueio interativo NÃO implementado:
- Detecta dispositivos LUKS via filesystem type e `cryptsetup luksDump`
- Identifica se já está aberto (device mapper)
- Obtém info do filesystem interno via `blkid`
- `unlock_luks_interactive` retorna erro explícito
- `close_luks` para fechar device mapper

## Fluxo de Uso

```rust
// 1. Descobre instalações candidatas (Fase 3)
let candidate = ...;

// 2. Constrói plano de montagem
let root_mount = PathBuf::from("/mnt/target");
let plan = build_mount_plan(&candidate, &devices, fstab_content, &root_mount);

// 3. Valida plano
if plan.has_blocking_errors() {
    // Erros bloqueantes: dependências ausentes, ciclos, filesystem desconhecido, etc.
    return Err(...);
}

// 4. Cria sessão e executa
let mut session = MountSession::new(root_mount.clone());
execute_mount_plan(&plan, &session)?;

// 5. Finaliza (ou rollback em erro)
session.finalize(false)?;
```

## Segurança

- **Confinamento de montagem**: Todos os mount points são relativos ao `root_mount`
- **Validação estrita**: Rejeita paths relativos, `..`, paths que escapam da raiz
- **noauto respeitado**: Entradas com `noauto` são marcadas como `SkippedNoauto` independentemente do dispositivo
- **nofail preservado**: Dispositivos ausentes com `nofail` geram `SkippedNofailMissing`, não erro
- **Ambiguidade bloqueia**: Identificadores que correspondem a múltiplos dispositivos geram `SkippedAmbiguous`
- **Dados sensíveis**: `raw_line`, `raw` options, senhas não serializados em JSON/logs
- **LUKS**: Apenas detecção; desbloqueio requer implementação segura com zeroização de memória

## Testes

Todos os testes usam:
- Diretórios temporários (`tempfile`)
- Dispositivos fictícios (mocks)
- Fixtures JSON para hardware simulado
- **Nenhum teste monta partições reais, usa chroot real, ou executa cryptsetup real**

## Limitações Conhecidas

1. **LUKS unlock não implementado** - Apenas detecção disponível
2. **Subvolumes Btrfs** - Detecção depende de `btrfs` command disponível
3. **Mounts reais** - Execução real requer privilégios de root e não é testada em CI