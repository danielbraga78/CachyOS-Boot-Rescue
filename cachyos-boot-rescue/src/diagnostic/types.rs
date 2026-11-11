//! Tipos centrais do motor de diagnóstico de bootloaders.
//!
//! Define as estruturas de dados para achados, evidências, severidade,
//! recomendações e ambiente de boot.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Identificador estável de um diagnóstico.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticId {
    // Layout EFI e ESP
    EfiNotMounted,
    MultipleEspsAmbiguous,
    EspNotFat32,
    EspMissingBootFlag,

    // GRUB
    GrubEfiBinaryMissing,
    GrubConfigMissing,
    GrubConfigInvalid,
    GrubModulesMissing,
    GrubDefaultEntryMissing,
    GrubUuidsMismatch,
    GrubKernelNotFound,
    GrubInitramfsNotFound,
    GrubBiosOnUefi,
    GrubUefiOnBios,

    // Limine
    LimineEfiBinaryMissing,
    LimineConfigMissing,
    LimineConfigInvalid,
    LimineKernelPathInvalid,
    LimineInitramfsPathInvalid,
    LimineEspLayoutMismatch,

    // Kernel e initramfs
    KernelNotFound,
    InitramfsNotFound,
    KernelInitramfsVersionMismatch,
    NoKernelsInstalled,

    // fstab e montagem
    FstabUuidMismatch,
    FstabMissingEspEntry,
    FstabMissingBootEntry,
    FstabInvalidSubvolume,
    MountpointMismatch,

    // UEFI NVRAM
    EfiBootEntriesMissing,
    EfiBootOrderMissing,
    EfiBootEntryInvalid,
    EfiBootEntryPointsToMissingFile,
    SecureBootEnabled,
    SecureBootStateUnknown,

    // Geral
    BootloaderNotDetected,
    MultipleBootloadersDetected,
    UnknownBootloader,
    ChrootValidationFailed,
    InsufficientPermissions,
}

impl DiagnosticId {
    /// Retorna a categoria do diagnóstico.
    pub fn category(&self) -> DiagnosticCategory {
        match self {
            Self::EfiNotMounted
            | Self::MultipleEspsAmbiguous
            | Self::EspNotFat32
            | Self::EspMissingBootFlag => DiagnosticCategory::EfiLayout,

            Self::GrubEfiBinaryMissing
            | Self::GrubConfigMissing
            | Self::GrubConfigInvalid
            | Self::GrubModulesMissing
            | Self::GrubDefaultEntryMissing
            | Self::GrubUuidsMismatch
            | Self::GrubKernelNotFound
            | Self::GrubInitramfsNotFound
            | Self::GrubBiosOnUefi
            | Self::GrubUefiOnBios => DiagnosticCategory::Grub,

            Self::LimineEfiBinaryMissing
            | Self::LimineConfigMissing
            | Self::LimineConfigInvalid
            | Self::LimineKernelPathInvalid
            | Self::LimineInitramfsPathInvalid
            | Self::LimineEspLayoutMismatch => DiagnosticCategory::Limine,

            Self::KernelNotFound
            | Self::InitramfsNotFound
            | Self::KernelInitramfsVersionMismatch
            | Self::NoKernelsInstalled => DiagnosticCategory::KernelInitramfs,

            Self::FstabUuidMismatch
            | Self::FstabMissingEspEntry
            | Self::FstabMissingBootEntry
            | Self::FstabInvalidSubvolume
            | Self::MountpointMismatch => DiagnosticCategory::FstabMount,

            Self::EfiBootEntriesMissing
            | Self::EfiBootOrderMissing
            | Self::EfiBootEntryInvalid
            | Self::EfiBootEntryPointsToMissingFile
            | Self::SecureBootEnabled
            | Self::SecureBootStateUnknown => DiagnosticCategory::UefiNvram,

            Self::BootloaderNotDetected
            | Self::MultipleBootloadersDetected
            | Self::UnknownBootloader
            | Self::ChrootValidationFailed
            | Self::InsufficientPermissions => DiagnosticCategory::General,
        }
    }

    /// Retorna uma descrição curta e legível.
    pub fn short_description(&self) -> &'static str {
        match self {
            Self::EfiNotMounted => "ESP não montada",
            Self::MultipleEspsAmbiguous => "Múltiplas ESPs ambíguas",
            Self::EspNotFat32 => "ESP não é FAT32",
            Self::EspMissingBootFlag => "ESP sem flag boot",
            Self::GrubEfiBinaryMissing => "Binário GRUB EFI ausente",
            Self::GrubConfigMissing => "grub.cfg ausente",
            Self::GrubConfigInvalid => "grub.cfg inválido",
            Self::GrubModulesMissing => "Módulos GRUB ausentes",
            Self::GrubDefaultEntryMissing => "Entrada padrão GRUB ausente",
            Self::GrubUuidsMismatch => "UUIDs no GRUB divergentes",
            Self::GrubKernelNotFound => "Kernel referenciado no GRUB não encontrado",
            Self::GrubInitramfsNotFound => "Initramfs referenciado no GRUB não encontrado",
            Self::GrubBiosOnUefi => "GRUB BIOS instalado em sistema UEFI",
            Self::GrubUefiOnBios => "GRUB UEFI instalado em sistema BIOS",
            Self::LimineEfiBinaryMissing => "Binário Limine EFI ausente",
            Self::LimineConfigMissing => "limine.conf ausente",
            Self::LimineConfigInvalid => "limine.conf inválido",
            Self::LimineKernelPathInvalid => "Caminho do kernel no Limine inválido",
            Self::LimineInitramfsPathInvalid => "Caminho do initramfs no Limine inválido",
            Self::LimineEspLayoutMismatch => "Layout ESP incompatível com Limine",
            Self::KernelNotFound => "Kernel não encontrado",
            Self::InitramfsNotFound => "Initramfs não encontrado",
            Self::KernelInitramfsVersionMismatch => "Versão kernel/initramfs divergente",
            Self::NoKernelsInstalled => "Nenhum kernel instalado",
            Self::FstabUuidMismatch => "UUID no fstab diverge do dispositivo",
            Self::FstabMissingEspEntry => "Entrada ESP ausente no fstab",
            Self::FstabMissingBootEntry => "Entrada /boot ausente no fstab",
            Self::FstabInvalidSubvolume => "Subvolume Btrfs inválido no fstab",
            Self::MountpointMismatch => "Ponto de montagem diverge do fstab",
            Self::EfiBootEntriesMissing => "Entradas EFI de boot ausentes",
            Self::EfiBootOrderMissing => "BootOrder EFI ausente",
            Self::EfiBootEntryInvalid => "Entrada EFI inválida",
            Self::EfiBootEntryPointsToMissingFile => "Entrada EFI aponta para arquivo inexistente",
            Self::SecureBootEnabled => "Secure Boot ativo",
            Self::SecureBootStateUnknown => "Estado Secure Boot desconhecido",
            Self::BootloaderNotDetected => "Bootloader não detectado",
            Self::MultipleBootloadersDetected => "Múltiplos bootloaders detectados",
            Self::UnknownBootloader => "Bootloader desconhecido",
            Self::ChrootValidationFailed => "Validação chroot falhou",
            Self::InsufficientPermissions => "Permissões insuficientes",
        }
    }
}

/// Categoria de diagnóstico para agrupamento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCategory {
    EfiLayout,
    Grub,
    Limine,
    KernelInitramfs,
    FstabMount,
    UefiNvram,
    General,
}

/// Severidade do achado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informativo - não indica problema.
    Info,
    /// Aviso - pode causar problemas em certas condições.
    Warning,
    /// Erro - impede boot ou funcionalidade esperada.
    Error,
    /// Crítico - sistema não inicializa sem correção.
    Critical,
}

/// Evidência que fundamenta um achado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    /// Tipo de evidência.
    pub kind: EvidenceKind,
    /// Descrição legível.
    pub description: String,
    /// Caminho do arquivo/diretório verificado (se aplicável).
    pub path: Option<PathBuf>,
    /// Valor esperado (se aplicável).
    pub expected: Option<String>,
    /// Valor observado (se aplicável).
    pub observed: Option<String>,
    /// Dados brutos adicionais (não sensíveis).
    #[serde(default)]
    pub raw_data: HashMap<String, String>,
}

/// Tipo de evidência.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// Arquivo existe.
    FileExists,
    /// Arquivo não existe.
    FileMissing,
    /// Conteúdo de arquivo corresponde ao esperado.
    ContentMatches,
    /// Conteúdo de arquivo não corresponde.
    ContentMismatch,
    /// Diretório existe.
    DirectoryExists,
    /// Diretório não existe.
    DirectoryMissing,
    /// Comando executado com sucesso.
    CommandSuccess,
    /// Comando falhou.
    CommandFailed,
    /// Saída de comando corresponde a padrão.
    CommandOutputMatches,
    /// Saída de comando não corresponde.
    CommandOutputMismatch,
    /// Valor de configuração corresponde.
    ConfigValueMatches,
    /// Valor de configuração diverge.
    ConfigValueMismatch,
    /// Dispositivo/partição corresponde.
    DeviceMatches,
    /// Dispositivo/partição ausente.
    DeviceMissing,
    /// Dispositivo/partição diverge.
    DeviceMismatch,
    /// Montagem ativa.
    MountActive,
    /// Montagem ausente.
    MountMissing,
    /// Entrada EFI existe.
    EfiEntryExists,
    /// Entrada EFI ausente.
    EfiEntryMissing,
    /// Entrada EFI aponta para arquivo válido.
    EfiEntryValid,
    /// Entrada EFI aponta para arquivo inválido.
    EfiEntryInvalid,
    /// Verificação de versão.
    VersionCheck,
    /// Verificação de permissão.
    PermissionCheck,
    /// Outro.
    Other,
}

impl std::fmt::Display for EvidenceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Recomendação de ação.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    /// Identificador da recomendação.
    pub id: String,
    /// Descrição textual explicando a evidência e o risco.
    pub description: String,
    /// Ação sugerida (texto legível).
    pub action: String,
    /// Se a ação é mutável (altera o sistema).
    pub is_mutating: bool,
    /// Risco estimado da ação.
    pub risk: RiskLevel,
    /// Pré-condições necessárias.
    #[serde(default)]
    pub prerequisites: Vec<String>,
}

/// Nível de risco da recomendação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// Baixo risco - operação segura e reversível.
    Low,
    /// Risco moderado - requer atenção e backup.
    Medium,
    /// Alto risco - pode tornar o sistema não inicializável.
    High,
    /// Crítico - apenas em último caso com backup confirmado.
    Critical,
}

/// Achado individual do diagnóstico.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// ID estável do diagnóstico.
    pub id: DiagnosticId,
    /// Severidade.
    pub severity: Severity,
    /// Título curto.
    pub title: String,
    /// Descrição detalhada.
    pub description: String,
    /// Evidências que fundamentam o achado.
    pub evidence: Vec<Evidence>,
    /// Recomendações associadas.
    pub recommendations: Vec<Recommendation>,
    /// Se o achado foi verificado no chroot (vs. apenas no host).
    pub verified_in_chroot: bool,
    /// Contexto adicional (caminhos, versões, etc).
    #[serde(default)]
    pub context: HashMap<String, String>,
}

/// Ambiente de boot detectado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BootEnvironment {
    /// Modo de firmware.
    pub firmware_mode: FirmwareMode,
    /// Bootloader principal detectado.
    pub primary_bootloader: Option<BootloaderType>,
    /// Bootloaders adicionais detectados.
    pub additional_bootloaders: Vec<BootloaderType>,
    /// ESP principal usada para boot (se identificada).
    pub primary_esp: Option<EspInfo>,
    /// Outras ESPs detectadas.
    pub other_esps: Vec<EspInfo>,
    /// Layout de /boot detectado.
    pub boot_layout: BootLayout,
    /// Kernels detectados.
    pub kernels: Vec<KernelInfo>,
    /// Initramfs detectados.
    pub initramfs: Vec<InitramfsInfo>,
    /// Entradas EFI NVRAM (somente leitura).
    pub efi_boot_entries: Vec<EfiBootEntry>,
    /// BootOrder EFI (se disponível).
    pub efi_boot_order: Option<Vec<u16>>,
    /// Status Secure Boot.
    pub secure_boot: SecureBootState,
    /// Distribuição detectada.
    pub distribution: Option<String>,
    /// Versão da distribuição.
    pub distribution_version: Option<String>,
}

/// Modo de firmware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FirmwareMode {
    Uefi,
    BiosLegacy,
    Unknown,
}

/// Tipo de bootloader detectado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BootloaderType {
    Grub,
    Limine,
    SystemdBoot,
    RefInd,
    WindowsBootManager,
    Unknown,
}

/// Informações da ESP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EspInfo {
    /// Caminho do dispositivo.
    pub device_path: PathBuf,
    /// UUID do filesystem.
    pub uuid: Option<String>,
    /// PARTUUID.
    pub partuuid: Option<String>,
    /// Label.
    pub label: Option<String>,
    /// Ponto de montagem atual.
    pub mountpoint: Option<PathBuf>,
    /// Tamanho em bytes.
    pub size: u64,
    /// Espaço livre (se montado).
    pub free_space: Option<u64>,
    /// Confiança como ESP (0-100).
    pub confidence: u8,
}

/// Layout de /boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum BootLayout {
    /// /boot em partição separada (ext4, etc), ESP em /boot/efi.
    SeparateBootEfi,
    /// /boot na ESP (systemd-boot unificado).
    UnifiedBootOnEsp,
    /// ESP montada em /efi, /boot separado.
    EfiAtEfiBootSeparate,
    /// BIOS Legacy com partição /boot separada.
    BiosLegacySeparateBoot,
    /// BIOS Legacy com /boot na raiz.
    BiosLegacyBootOnRoot,
    /// Desconhecido ou não determinado.
    Unknown,
}

/// Informações de kernel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KernelInfo {
    /// Caminho do arquivo.
    pub path: PathBuf,
    /// Versão do kernel (ex: 6.12.1-arch1-1).
    pub version: String,
    /// Se é o kernel padrão/mais recente.
    pub is_default: bool,
    /// Tamanho em bytes.
    pub size: u64,
}

/// Informações de initramfs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InitramfsInfo {
    /// Caminho do arquivo.
    pub path: PathBuf,
    /// Versão correspondente do kernel.
    pub kernel_version: String,
    /// Tipo (ex: default, fallback, lts).
    pub variant: String,
    /// Tamanho em bytes.
    pub size: u64,
}

/// Entrada de boot EFI (somente leitura).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EfiBootEntry {
    /// Número da entrada (BootXXXX).
    pub boot_num: u16,
    /// Rótulo/descrição.
    pub label: String,
    /// Caminho do carregador EFI.
    pub loader_path: String,
    /// Dispositivo/partição (ex: HD(1,GPT,...)).
    pub device_path: String,
    /// Argumentos opcionais.
    pub arguments: Option<String>,
    /// Se está ativa no BootOrder.
    pub is_active: bool,
}

/// Estado do Secure Boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecureBootState {
    Enabled,
    Disabled,
    Unknown,
    Unsupported,
}

/// Relatório completo de diagnóstico.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticReport {
    /// Timestamp da execução.
    pub timestamp: DateTime<Utc>,
    /// ID da sessão.
    pub session_id: String,
    /// Ambiente de boot detectado.
    pub environment: BootEnvironment,
    /// Todos os achados.
    pub findings: Vec<Finding>,
    /// Resumo por severidade.
    pub summary: DiagnosticSummary,
    /// Duração do diagnóstico em milissegundos.
    pub duration_ms: u64,
    /// Versão do diagnostic engine.
    pub engine_version: String,
}

/// Resumo estatístico do diagnóstico.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DiagnosticSummary {
    /// Total de achados.
    pub total: usize,
    /// Contagem por severidade.
    pub by_severity: HashMap<Severity, usize>,
    /// Contagem por categoria.
    pub by_category: HashMap<DiagnosticCategory, usize>,
    /// Se há achados críticos.
    pub has_critical: bool,
    /// Se há erros.
    pub has_errors: bool,
    /// Se há avisos.
    pub has_warnings: bool,
}

impl DiagnosticSummary {
    /// Atualiza o resumo a partir de uma lista de achados.
    pub fn from_findings(findings: &[Finding]) -> Self {
        let mut summary = Self::default();
        summary.total = findings.len();

        for finding in findings {
            *summary.by_severity.entry(finding.severity).or_insert(0) += 1;
            *summary
                .by_category
                .entry(finding.id.category())
                .or_insert(0) += 1;

            match finding.severity {
                Severity::Critical => summary.has_critical = true,
                Severity::Error => summary.has_errors = true,
                Severity::Warning => summary.has_warnings = true,
                Severity::Info => {}
            }
        }

        summary
    }
}

/// Configuração para execução do diagnóstico.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticConfig {
    /// Incluir verificações de GRUB.
    pub check_grub: bool,
    /// Incluir verificações de Limine.
    pub check_limine: bool,
    /// Incluir verificações de systemd-boot.
    pub check_systemd_boot: bool,
    /// Incluir verificação de entradas EFI NVRAM.
    pub check_efi_entries: bool,
    /// Incluir verificação de Secure Boot.
    pub check_secure_boot: bool,
    /// Incluir verificação de kernels/initramfs.
    pub check_kernels: bool,
    /// Incluir verificação de fstab/montagem.
    pub check_fstab_mount: bool,
    /// Timeout por verificação individual (segundos).
    pub check_timeout_secs: u64,
    /// Executar verificações que requerem chroot.
    pub use_chroot: bool,
    /// Caminho do chroot root (se use_chroot).
    pub chroot_root: Option<PathBuf>,
}

impl Default for DiagnosticConfig {
    fn default() -> Self {
        Self {
            check_grub: true,
            check_limine: true,
            check_systemd_boot: true,
            check_efi_entries: true,
            check_secure_boot: true,
            check_kernels: true,
            check_fstab_mount: true,
            check_timeout_secs: 30,
            use_chroot: false,
            chroot_root: None,
        }
    }
}

/// Resultado da execução de diagnóstico.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticRun {
    /// Relatório gerado.
    pub report: DiagnosticReport,
    /// Erros não fatais durante a execução.
    pub warnings: Vec<String>,
}

impl DiagnosticRun {
    /// Cria uma nova execução de diagnóstico.
    pub fn new(report: DiagnosticReport) -> Self {
        Self {
            report,
            warnings: Vec::new(),
        }
    }

    /// Adiciona um aviso não fatal.
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_diagnostic_id_category() {
        assert_eq!(
            DiagnosticId::GrubConfigMissing.category(),
            DiagnosticCategory::Grub
        );
        assert_eq!(
            DiagnosticId::LimineConfigMissing.category(),
            DiagnosticCategory::Limine
        );
        assert_eq!(
            DiagnosticId::EfiNotMounted.category(),
            DiagnosticCategory::EfiLayout
        );
        assert_eq!(
            DiagnosticId::KernelNotFound.category(),
            DiagnosticCategory::KernelInitramfs
        );
        assert_eq!(
            DiagnosticId::FstabUuidMismatch.category(),
            DiagnosticCategory::FstabMount
        );
        assert_eq!(
            DiagnosticId::EfiBootEntriesMissing.category(),
            DiagnosticCategory::UefiNvram
        );
        assert_eq!(
            DiagnosticId::BootloaderNotDetected.category(),
            DiagnosticCategory::General
        );
    }

    #[test]
    fn test_diagnostic_id_short_description() {
        assert_eq!(
            DiagnosticId::GrubConfigMissing.short_description(),
            "grub.cfg ausente"
        );
        assert_eq!(
            DiagnosticId::EfiNotMounted.short_description(),
            "ESP não montada"
        );
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Critical);
    }

    #[test]
    fn test_diagnostic_summary_from_findings() {
        let findings = vec![
            Finding {
                id: DiagnosticId::GrubConfigMissing,
                severity: Severity::Error,
                title: "Test".to_string(),
                description: "Test".to_string(),
                evidence: vec![],
                recommendations: vec![],
                verified_in_chroot: false,
                context: HashMap::new(),
            },
            Finding {
                id: DiagnosticId::EfiNotMounted,
                severity: Severity::Warning,
                title: "Test".to_string(),
                description: "Test".to_string(),
                evidence: vec![],
                recommendations: vec![],
                verified_in_chroot: false,
                context: HashMap::new(),
            },
            Finding {
                id: DiagnosticId::KernelNotFound,
                severity: Severity::Critical,
                title: "Test".to_string(),
                description: "Test".to_string(),
                evidence: vec![],
                recommendations: vec![],
                verified_in_chroot: false,
                context: HashMap::new(),
            },
        ];

        let summary = DiagnosticSummary::from_findings(&findings);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.by_severity[&Severity::Error], 1);
        assert_eq!(summary.by_severity[&Severity::Warning], 1);
        assert_eq!(summary.by_severity[&Severity::Critical], 1);
        assert!(summary.has_critical);
        assert!(summary.has_errors);
        assert!(summary.has_warnings);
        assert_eq!(summary.by_category[&DiagnosticCategory::Grub], 1);
        assert_eq!(summary.by_category[&DiagnosticCategory::EfiLayout], 1);
        assert_eq!(summary.by_category[&DiagnosticCategory::KernelInitramfs], 1);
    }

    #[test]
    fn test_evidence_kind_serialization() {
        let evidence = Evidence {
            kind: EvidenceKind::FileMissing,
            description: "grub.cfg not found".to_string(),
            path: Some(PathBuf::from("/boot/grub/grub.cfg")),
            expected: Some("file exists".to_string()),
            observed: Some("file not found".to_string()),
            raw_data: HashMap::new(),
        };

        let json = serde_json::to_string(&evidence).unwrap();
        assert!(json.contains("file_missing"));
        assert!(json.contains("grub.cfg"));
    }

    #[test]
    fn test_recommendation_risk_levels() {
        let rec = Recommendation {
            id: "test-rec".to_string(),
            description: "Test".to_string(),
            action: "Test action".to_string(),
            is_mutating: true,
            risk: RiskLevel::High,
            prerequisites: vec!["backup".to_string()],
        };

        assert_eq!(rec.risk, RiskLevel::High);
        assert!(rec.is_mutating);
    }

    #[test]
    fn test_boot_environment_serialization() {
        let env = BootEnvironment {
            firmware_mode: FirmwareMode::Uefi,
            primary_bootloader: Some(BootloaderType::Grub),
            additional_bootloaders: vec![],
            primary_esp: Some(EspInfo {
                device_path: PathBuf::from("/dev/nvme0n1p1"),
                uuid: Some("ABCD-1234".to_string()),
                partuuid: Some("12345678-1234-1234-1234-123456789abc".to_string()),
                label: Some("EFI".to_string()),
                mountpoint: Some(PathBuf::from("/efi")),
                size: 512 * 1024 * 1024,
                free_space: Some(256 * 1024 * 1024),
                confidence: 95,
            }),
            other_esps: vec![],
            boot_layout: BootLayout::EfiAtEfiBootSeparate,
            kernels: vec![],
            initramfs: vec![],
            efi_boot_entries: vec![],
            efi_boot_order: None,
            secure_boot: SecureBootState::Disabled,
            distribution: Some("CachyOS".to_string()),
            distribution_version: Some("2026.11".to_string()),
        };

        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("uefi"));
        assert!(json.contains("grub"));
        assert!(json.contains("nvme0n1p1"));
        assert!(json.contains("EfiAtEfiBootSeparate"));
    }
}
