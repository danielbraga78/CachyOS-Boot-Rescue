# Módulo de Descoberta (Discovery)

## Visão Geral

O módulo `discovery` é responsável pela descoberta **somente leitura** de hardware, discos, partições, filesystems, partições EFI (ESP) e instalações Linux candidatas. Ele não monta, desmonta, desbloqueia ou modifica nenhum dispositivo.

## Componentes Principais

### 1. `lsblk.rs` - Adaptador para `lsblk --json`

- Executa `lsblk` com argumentos fixos para saída JSON consistente
- Parseia a saída JSON em estruturas `BlockDevice` tipadas
- Suporta timeouts (padrão 30s) e limites de saída (padrão 1MB)
- Força locale C para saída independente de idioma
- Trata ausência de ferramenta como diagnóstico, não panic
- Propaga flag de removível do pai para filhos
- Coleta erros de parsing de filhos sem descartar silenciosamente

### 2. `blkid.rs` - Adaptador para `blkid -o json -p`

- Usado **somente** para complementar dados não disponíveis no lsblk
- Parseia saída JSON do blkid
- Enriquece `BlockDevice` com UUID, label, PARTUUID, tipo de filesystem
- Não sobrescreve dados já presentes do lsblk
- Trata ausência de ferramenta como diagnóstico

### 3. `esp.rs` - Identificação de Candidatos ESP

Identifica partições EFI System Partition (ESP) baseado em:
1. **Tipo de partição GPT EFI System** (40 pontos)
2. **Filesystem FAT/VFAT** (30 pontos)
3. **PARTUUID correspondente a ESP** (20 pontos, fallback)
4. **Label sugerindo ESP** (10 pontos)
5. **Montada em `/boot/efi` ou `/efi`** (15 pontos)
6. **Montada em `/boot`** (5 pontos)
7. **Tamanho típico (50MB - 2GB)** (5 pontos)

Múltiplas ESPs são apresentadas como candidatas ordenadas por confiança, **não escolhidas arbitrariamente**.

### 4. `linux.rs` - Identificação de Instalações Linux

Dois modos de detecção:

#### `identify_linux_installations()` - Dispositivos já montados
- Procura partições com filesystem Linux nativo (ext4, btrfs, xfs, ext3, ext2) montadas em `/`
- Verifica `/etc/os-release`, `/etc/fstab`, kernels em `/boot`
- Associa partições de boot/ESP montadas sob a raiz
- Detecta subvolumes Btrfs (`@`, `@root`, `root`)

#### `identify_from_metadata()` - Metadados de partições não montadas
- Usa apenas tipo de partição GPT (LinuxFilesystem, LinuxRaid, LinuxLvm)
- **Não considera partições swap como instalações candidatas**
- Confiança baixa (15) por ser apenas metadados

### 5. `uefi.rs` - Detecção de Estado UEFI/BIOS

- `UefiState::Uefi` - `/sys/firmware/efi` existe e é legível
- `UefiState::BiosLegacy` - Diretório não existe
- `UefiState::Unknown` - Erro de acesso

### 6. `mod.rs` - Orquestração Principal

`run_discovery()` executa fluxo completo:
1. Detecta estado UEFI
2. Executa e parseia lsblk
3. Enriquece com blkid (se disponível)
4. Identifica candidatos ESP
5. Identifica instalações Linux (montadas + metadados)
6. Combina e deduplica (prioriza montadas)
7. Retorna `DiscoveryReport`

## Tipos de Dados Principais

### `BlockDevice`
Representa disco ou partição com campos `Option` para valores desconhecidos:
- `device_id: Option<DeviceId>` - major:minor
- `size: Option<u64>` - bytes
- `sector_size: Option<u32>` - setor físico
- `logical_sector_size: Option<u32>` - setor lógico
- `mountpoints: Vec<PathBuf>` - pontos de montagem alternativos (lsblk MOUNTPOINTS)
- `serial: Option<String>` - **não serializado** (privacidade)

### `EspCandidate`
- `device: BlockDevice`
- `filesystem: FilesystemInfo`
- `mountpoint: Option<PathBuf>`
- `is_boot_efi_mount: bool` - montada em `/boot/efi` ou `/efi`
- `is_boot_mount: bool` - montada em `/boot`
- `size: u64`
- `free_space: Option<u64>` - **None na fase read-only**
- `confidence: u8` (0-100)
- `reasons: Vec<String>`

### `LinuxInstallationCandidate`
- `root_device: BlockDevice`
- `root_filesystem: FilesystemInfo`
- `boot_partitions: Vec<EspCandidate>`
- `distribution: Option<String>` - de `/etc/os-release`
- `kernel_paths: Vec<PathBuf>`
- `initramfs_paths: Vec<PathBuf>`
- `has_fstab: bool`
- `is_btrfs_subvolume: bool`
- `confidence: u8`
- `reasons: Vec<String>`

### `DiscoveryReport`
Relatório completo com timestamp, session_id, uefi_state, devices, esp_candidates, linux_candidates, diagnostics, versões de ferramentas.

## Segurança e Privacidade

- **Somente leitura**: Nenhuma operação de montagem, escrita ou modificação
- **Serial não serializado**: Campo `serial` tem `#[serde(skip_serializing)]`
- **Sanitização**: Integra com sistema de logging da Fase 2 para relatórios sanitizados
- **Ausência = None**: Valores não disponíveis permanecem `None`, não viram defaults artificiais
- **Timeouts**: Todos os comandos externos têm timeout (30s) e limite de saída (1MB)

## Testes

Todos os testes usam **fixtures JSON sintéticos** (hardware fictício):
- `sata_single_disk.json` - Disco SATA com ESP, root, home
- `nvme_single_disk.json` - NVMe com ESP em `/efi`, Btrfs com subvolumes
- `multiple_esps.json` - Duas ESPs Linux + uma ESP Windows + NTFS
- `no_labels.json` - Partições sem label
- `removable_disks.json` - Disco interno, USB, SD card MBR
- `incomplete_data.json` - Dados parciais, loop device
- `luks_encrypted.json` - LUKS com dispositivo descriptografado dm-0

Cenários cobertos:
- Parsing robusto de JSON do lsblk
- Identificação de múltiplas ESPs sem seleção arbitrária
- Detecção de instalações Linux montadas e via metadados
- Enriquecimento com blkid
- Tratamento de dados incompletos
- Dispositivos removíveis
- LUKS criptografado

## Integração com Fase 2 (Logging)

- Usa `tracing` para logging estruturado
- Registra diagnósticos como `DiscoveryDiagnostic` (Error/Warning/Info)
- Integra com `SessionId` para rastreabilidade
- Exporta relatórios sanitizados via sistema de logging