//! Módulo de logging estruturado do cachyos-boot-rescue.
//!
//! Fornece logging baseado em sessões com:
//! - Níveis de severidade (ERROR, WARN, INFO, DEBUG, TRACE)
//! - Rotação de arquivos por tamanho e contagem
//! - Caminhos configuráveis com fallback seguro
//! - Eventos estruturados (timestamp, session_id, componente, operação, severidade, mensagem)
//! - Coleta e exportação de relatórios sanitizados
//! - Sanitização de segredos (senhas, tokens, chaves, LUKS, env vars, usernames, machine IDs)

use crate::config::{Config, LogLevel};
use crate::error::{Error, Result, ResultIoExt};
use chrono::{DateTime, Local, Utc};
use flate2::Compression;
use flate2::write::GzEncoder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{Layer, Registry};
use uuid::Uuid;

static LOGGING_INSTANCE: OnceLock<LoggingInstance> = OnceLock::new();

/// Identificador único de sessão de logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    /// Gera um novo ID de sessão aleatório.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Cria SessionId a partir de string UUID.
    pub fn from_string(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

use std::fmt;

/// Configuração do sistema de logging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Diretório base para logs (padrão: /var/log/cachyos-boot-rescue ou fallback).
    pub log_dir: PathBuf,

    /// Nível mínimo de log para arquivo.
    #[serde(default = "default_file_log_level")]
    pub file_log_level: LogLevel,

    /// Nível mínimo de log para console (já controlado por Config.log_level).
    #[serde(default)]
    pub console_log_level: LogLevel,

    /// Tamanho máximo de cada arquivo de log em bytes (padrão: 10MB).
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,

    /// Número máximo de arquivos de log rotacionados (padrão: 10).
    #[serde(default = "default_max_files")]
    pub max_files: usize,

    /// Habilita logging em arquivo.
    #[serde(default = "default_true")]
    pub file_logging_enabled: bool,

    /// Habilita sanitização automática em exportação.
    #[serde(default = "default_true")]
    pub sanitize_on_export: bool,

    /// Padrões adicionais para sanitização (regex).
    #[serde(default)]
    pub extra_sanitize_patterns: Vec<String>,

    /// Se deve mascarar nomes de usuários.
    #[serde(default = "default_true")]
    pub mask_usernames: bool,

    /// Se deve mascarar identificadores de máquina.
    #[serde(default = "default_true")]
    pub mask_machine_ids: bool,
}

fn default_file_log_level() -> LogLevel {
    LogLevel::Debug
}

fn default_max_file_size() -> u64 {
    10 * 1024 * 1024 // 10 MB
}

fn default_max_files() -> usize {
    10
}

fn default_true() -> bool {
    true
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_dir: default_log_dir(),
            file_log_level: default_file_log_level(),
            console_log_level: LogLevel::Info,
            max_file_size: default_max_file_size(),
            max_files: default_max_files(),
            file_logging_enabled: true,
            sanitize_on_export: true,
            extra_sanitize_patterns: Vec::new(),
            mask_usernames: true,
            mask_machine_ids: true,
        }
    }
}

impl LoggingConfig {
    /// Valida a configuração de logging.
    pub fn validate(&self) -> Result<()> {
        if self.max_file_size == 0 {
            return Err(Error::validation(
                "tamanho máximo de arquivo de log deve ser maior que zero",
                Some("logging.max_file_size".to_string()),
            ));
        }

        if self.max_files == 0 {
            return Err(Error::validation(
                "número máximo de arquivos de log deve ser maior que zero",
                Some("logging.max_files".to_string()),
            ));
        }

        // Valida padrões regex extras
        for pattern in &self.extra_sanitize_patterns {
            if pattern.is_empty() {
                return Err(Error::validation(
                    "padrão regex extra não pode ser vazio",
                    Some("logging.extra_sanitize_patterns".to_string()),
                ));
            }
            regex::Regex::new(pattern).map_err(|e| {
                Error::validation(
                    format!("padrão regex inválido: {e}"),
                    Some("logging.extra_sanitize_patterns".to_string()),
                )
            })?;
        }

        Ok(())
    }
}

/// Determina o diretório de log apropriado.
///
/// Prioridade:
/// 1. /var/log/cachyos-boot-rescue (se gravável)
/// 2. $XDG_DATA_HOME/cachyos-boot-rescue/logs
/// 3. ~/.local/share/cachyos-boot-rescue/logs
/// 4. Diretório temporário seguro
fn default_log_dir() -> PathBuf {
    // Tenta /var/log primeiro (requer root ou permissão)
    let var_log = PathBuf::from("/var/log/cachyos-boot-rescue");
    if can_write_dir(&var_log) {
        return var_log;
    }

    // Fallback para diretório de dados do usuário
    if let Some(data_dir) = dirs::data_dir() {
        let user_log = data_dir.join("cachyos-boot-rescue").join("logs");
        if can_write_dir(&user_log) || fs::create_dir_all(&user_log).is_ok() {
            return user_log;
        }
    }

    // Último recurso: diretório temporário
    std::env::temp_dir().join("cachyos-boot-rescue-logs")
}

fn can_write_dir(path: &Path) -> bool {
    // Verifica se o diretório existe e é gravável
    if path.exists() {
        // Tenta criar um arquivo temporário para testar escrita
        let test_file = path.join(".write_test");
        match File::create(&test_file) {
            Ok(_) => {
                let _ = fs::remove_file(&test_file);
                true
            }
            Err(_) => false,
        }
    } else {
        // Tenta criar o diretório
        fs::create_dir_all(path).is_ok()
    }
}

/// Evento de log estruturado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    /// Timestamp em UTC com nanossegundos.
    pub timestamp: DateTime<Utc>,

    /// Timestamp local formatado para exibição.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_timestamp: Option<String>,

    /// ID da sessão.
    pub session_id: SessionId,

    /// Componente/módulo que gerou o evento.
    pub component: String,

    /// Operação sendo realizada.
    pub operation: String,

    /// Nível de severidade.
    pub level: LogLevel,

    /// Mensagem principal.
    pub message: String,

    /// Campos adicionais estruturados.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub fields: HashMap<String, serde_json::Value>,

    /// ID do span pai (se aplicável).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<u64>,

    /// ID do span atual.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<u64>,
}

impl LogEvent {
    /// Cria um novo evento de log.
    pub fn new(
        session_id: SessionId,
        component: impl Into<String>,
        operation: impl Into<String>,
        level: LogLevel,
        message: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            timestamp: now,
            local_timestamp: Some(now.with_timezone(&Local).to_rfc3339()),
            session_id,
            component: component.into(),
            operation: operation.into(),
            level,
            message: message.into(),
            fields: HashMap::new(),
            parent_span_id: None,
            span_id: None,
        }
    }

    /// Adiciona um campo estruturado.
    pub fn with_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.fields.insert(key.into(), value);
        self
    }

    /// Adiciona múltiplos campos.
    pub fn with_fields(mut self, fields: HashMap<String, serde_json::Value>) -> Self {
        self.fields.extend(fields);
        self
    }

    /// Define o ID do span pai.
    pub fn with_parent_span(mut self, id: u64) -> Self {
        self.parent_span_id = Some(id);
        self
    }

    /// Define o ID do span atual.
    pub fn with_span(mut self, id: u64) -> Self {
        self.span_id = Some(id);
        self
    }
}

/// Sanitizador de dados sensíveis.
pub struct Sanitizer {
    patterns: Vec<Regex>,
    mask_usernames: bool,
    mask_machine_ids: bool,
}

impl Sanitizer {
    /// Cria um novo sanitizador com configuração padrão.
    pub fn new(config: &LoggingConfig) -> Result<Self> {
        let mut patterns = Vec::new();

        // Padrões padrão para segredos comuns
        let default_patterns = [
            // Senhas em formato key=value ou key: value
            r#"(?i)(password|passwd|pwd|secret|token|key|api[_-]?(key|token)|auth[_-]?(token|key))\s*[:=]\s*["']?([^"'\s]+)["']?"#,
            // Chaves SSH/GPG privadas
            r#"-----BEGIN (RSA|DSA|EC|OPENSSH|PGP) PRIVATE KEY-----[\s\S]*?-----END (RSA|DSA|EC|OPENSSH|PGP) PRIVATE KEY-----"#,
            // Material LUKS (passphrases, keys) - aceita luksPassphrase como palavra única ou luks passphrase
            r#"(?i)(luks|cryptsetup)\s*(passphrase|key|password)\s*[:=]\s*["']?([^"'\s]+)["']?"#,
            // Variáveis de ambiente sensíveis
            r#"(?i)(AWS_SECRET|GITHUB_TOKEN|DOCKER_PASSWORD|NPM_TOKEN|PASSWORD|SECRET|TOKEN|KEY)\s*=\s*["']?([^"'\s]+)["']?"#,
            // URLs com credenciais
            r#"(https?://)([^:/\s]+):([^@/\s]+)@"#,
            // Strings que parecem segredos (base64, hex longos, strings aleatórias longas)
            r#"(?i)([A-Za-z0-9+/]{20,}={0,2})"#,
            r#"(?i)([a-fA-F0-9]{32,})"#,
            // IPs privados (opcional - pode ser útil para debug)
            // r"\b(?:10|172\.(?:1[6-9]|2[0-9]|3[0-1])|192\.168)\.\d{1,3}\.\d{1,3}\b",
        ];

        for pattern in default_patterns {
            patterns.push(Regex::new(pattern).map_err(|e| {
                Error::internal_with_source("Falha ao compilar regex de sanitização padrão", e)
            })?);
        }

        // Padrões extras do usuário
        for pattern in &config.extra_sanitize_patterns {
            patterns.push(Regex::new(pattern).map_err(|e| {
                Error::internal_with_source(
                    format!("Falha ao compilar regex de sanitização extra: {pattern}"),
                    e,
                )
            })?);
        }

        Ok(Self {
            patterns,
            mask_usernames: config.mask_usernames,
            mask_machine_ids: config.mask_machine_ids,
        })
    }

    /// Sanitiza uma string, mascarando segredos.
    pub fn sanitize(&self, input: &str) -> String {
        let mut result = input.to_string();

        // Aplica padrões de regex - usa uma única passagem com todos os padrões combinados
        // para evitar substituições múltiplas e duplicatas
        let combined_pattern = self
            .patterns
            .iter()
            .map(|r| r.as_str())
            .collect::<Vec<_>>()
            .join("|");

        if let Ok(combined_re) = Regex::new(&combined_pattern) {
            result = combined_re
                .replace_all(&result, |caps: &regex::Captures| {
                    let m = caps.get(0).unwrap().as_str();

                    // Se o match contém = ou :, preserva o prefixo (chave=)
                    if m.contains('=') || m.contains(':') {
                        if let Some(eq_pos) = m.find('=') {
                            format!("{}***REDACTED***", &m[..=eq_pos])
                        } else if let Some(colon_pos) = m.find(':') {
                            format!("{} ***REDACTED***", &m[..=colon_pos])
                        } else {
                            "***REDACTED***".to_string()
                        }
                    } else if m.starts_with("-----BEGIN") {
                        "-----BEGIN ***REDACTED***-----".to_string()
                    } else if m.contains("://") {
                        // URL com credenciais
                        if let Some(at_pos) = m.find('@') {
                            let prefix = &m[..=at_pos];
                            format!("{}***REDACTED***@", prefix)
                        } else {
                            "***REDACTED***".to_string()
                        }
                    } else {
                        "***REDACTED***".to_string()
                    }
                })
                .to_string();
        } else {
            // Fallback: aplica padrões individualmente (ordem original)
            for pattern in &self.patterns {
                result = pattern
                    .replace_all(&result, |caps: &regex::Captures| {
                        let m = caps.get(0).unwrap().as_str();

                        if m.contains('=') || m.contains(':') {
                            if let Some(eq_pos) = m.find('=') {
                                format!("{}***REDACTED***", &m[..=eq_pos])
                            } else if let Some(colon_pos) = m.find(':') {
                                format!("{} ***REDACTED***", &m[..=colon_pos])
                            } else {
                                "***REDACTED***".to_string()
                            }
                        } else if m.starts_with("-----BEGIN") {
                            "-----BEGIN ***REDACTED***-----".to_string()
                        } else if m.contains("://") {
                            // URL com credenciais
                            if let Some(at_pos) = m.find('@') {
                                let prefix = &m[..=at_pos];
                                format!("{}***REDACTED***@", prefix)
                            } else {
                                "***REDACTED***".to_string()
                            }
                        } else {
                            "***REDACTED***".to_string()
                        }
                    })
                    .to_string();
            }
        }

        // Mascara nomes de usuários comuns
        if self.mask_usernames {
            let username_patterns = [
                r#"(?i)(user(name)?|login|account)\s*[:=]\s*["']?([^"'\s]+)["']?"#,
                r#"(?i)[-\s](u|user|username)\s+([^-\s]+)"#,
            ];
            for pattern_str in username_patterns {
                if let Ok(re) = Regex::new(pattern_str) {
                    result = re
                        .replace_all(&result, |caps: &regex::Captures| {
                            let m = caps.get(0).unwrap().as_str();
                            if m.contains('=') || m.contains(':') {
                                if let Some(pos) = m.find('=').or_else(|| m.find(':')) {
                                    format!("{}***REDACTED***", &m[..=pos])
                                } else {
                                    "***REDACTED***".to_string()
                                }
                            } else if let Some(space_pos) = m.rfind(' ') {
                                format!("{} ***REDACTED***", &m[..=space_pos])
                            } else {
                                "***REDACTED***".to_string()
                            }
                        })
                        .to_string();
                }
            }
        }

        // Mascara identificadores de máquina
        if self.mask_machine_ids {
            // MAC addresses
            if let Ok(re) = Regex::new(r"([0-9a-fA-F]{2}[:-]){5}[0-9a-fA-F]{2}") {
                result = re.replace_all(&result, "XX:XX:XX:XX:XX:XX").to_string();
            }
            // Hostnames (padrão simples)
            if let Ok(re) =
                Regex::new(r"\b[a-z0-9]([-a-z0-9]*[a-z0-9])?(\.[a-z0-9]([-a-z0-9]*[a-z0-9])?)+\b")
            {
                result = re.replace_all(&result, "host.example.com").to_string();
            }
            // UUIDs
            if let Ok(re) = Regex::new(
                r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
            ) {
                result = re
                    .replace_all(&result, "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx")
                    .to_string();
            }
        }

        result
    }

    /// Sanitiza um valor JSON recursivamente.
    pub fn sanitize_json(&self, value: &mut serde_json::Value) {
        match value {
            serde_json::Value::String(s) => {
                *s = self.sanitize(s);
            }
            serde_json::Value::Object(map) => {
                for (_, v) in map.iter_mut() {
                    self.sanitize_json(v);
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr.iter_mut() {
                    self.sanitize_json(v);
                }
            }
            _ => {}
        }
    }

    /// Sanitiza um evento de log.
    pub fn sanitize_event(&self, event: &mut LogEvent) {
        event.message = self.sanitize(&event.message);
        for v in event.fields.values_mut() {
            self.sanitize_json(v);
        }
    }
}

/// Instância global do sistema de logging.
pub struct LoggingInstance {
    session_id: SessionId,
    config: LoggingConfig,
    sanitizer: Sanitizer,
    _guard: Option<tracing_appender::non_blocking::WorkerGuard>,
    _file_appender: Option<tracing_appender::non_blocking::NonBlocking>,
    session_events: Arc<Mutex<Vec<LogEvent>>>,
}

impl LoggingInstance {
    /// Inicializa o sistema de logging global.
    pub fn init(config: LoggingConfig) -> Result<SessionId> {
        let session_id = SessionId::new();
        let sanitizer = Sanitizer::new(&config)?;

        let session_events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = session_events.clone();

        // Cria o diretório de logs se habilitado
        let (mut non_blocking, guard) = if config.file_logging_enabled {
            let log_dir = &config.log_dir;
            fs::create_dir_all(log_dir)
                .map_err(|e| Error::internal_with_source("criando diretório de logs", e))?;

            let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix("cachyos-boot-rescue")
                .filename_suffix("log")
                .max_log_files(config.max_files)
                .build(log_dir)
                .map_err(|e| Error::internal_with_source("criando appender de log rotativo", e))?;

            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            (Some(non_blocking), Some(guard))
        } else {
            (None, None)
        };

        // Converte LogLevel para tracing::Level
        let console_level = match config.console_log_level {
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        };
        let file_level = match config.file_log_level {
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        };

        // Cria o subscriber com layers
        let console_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout)
            .with_ansi(true)
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .compact()
            .with_filter(tracing_subscriber::filter::LevelFilter::from_level(
                console_level,
            ));

        let subscriber = Registry::default().with(console_layer);

        // Add file layer if enabled - use BoxMakeWriter to unify writer types
        let file_make_writer = if config.file_logging_enabled {
            tracing_subscriber::fmt::writer::BoxMakeWriter::new(non_blocking.take().unwrap())
        } else {
            // Use stdout with OFF filter as fallback (won't produce output)
            tracing_subscriber::fmt::writer::BoxMakeWriter::new(std::io::stdout)
        };

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(file_make_writer)
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_filter(tracing_subscriber::filter::LevelFilter::from_level(
                if config.file_logging_enabled {
                    file_level
                } else {
                    tracing::Level::ERROR
                },
            ));

        let subscriber = subscriber.with(file_layer);

        // Adiciona layer para capturar eventos estruturados
        let event_layer = EventCaptureLayer::new(events_clone);
        let subscriber = subscriber.with(event_layer);

        tracing::subscriber::set_global_default(subscriber).map_err(|e| {
            Error::internal_with_source("Falha ao inicializar subscriber de logging", e)
        })?;

        let instance = Self {
            session_id,
            config: config.clone(),
            sanitizer,
            _guard: guard,
            _file_appender: non_blocking,
            session_events,
        };

        LOGGING_INSTANCE
            .set(instance)
            .map_err(|_| Error::internal("Sistema de logging já inicializado"))?;

        // Log inicial da sessão
        tracing::info!(
            session_id = %session_id,
            log_dir = %config.log_dir.display(),
            "Sessão de logging iniciada"
        );

        Ok(session_id)
    }

    /// Obtém a instância atual (se inicializada).
    pub fn current() -> Option<&'static LoggingInstance> {
        LOGGING_INSTANCE.get()
    }

    /// Retorna o ID da sessão atual.
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// Retorna a configuração atual.
    pub fn config(&self) -> &LoggingConfig {
        &self.config
    }

    /// Registra um evento estruturado programaticamente.
    pub fn log_event(&self, event: LogEvent) {
        if let Ok(mut events) = self.session_events.lock() {
            events.push(event);
        }
    }

    /// Obtém todos os eventos da sessão atual.
    pub fn get_session_events(&self) -> Vec<LogEvent> {
        self.session_events
            .lock()
            .map(|e| e.clone())
            .unwrap_or_default()
    }

    /// Exporta relatório da sessão atual para arquivo compactado.
    pub fn export_report(&self, output_path: &Path) -> Result<PathBuf> {
        let events = self.get_session_events();
        let mut sanitized_events = events.clone();

        // Sanitiza eventos se configurado
        if self.config.sanitize_on_export {
            for event in &mut sanitized_events {
                self.sanitizer.sanitize_event(event);
            }
        }

        // Cria metadados do relatório
        let metadata = ReportMetadata {
            session_id: self.session_id,
            generated_at: Utc::now(),
            total_events: events.len(),
            sanitized: self.config.sanitize_on_export,
            config: self.config.clone(),
        };

        let report = SessionReport {
            metadata,
            events: sanitized_events,
        };

        // Serializa para JSON
        let json = serde_json::to_vec_pretty(&report)
            .map_err(|e| Error::internal_with_source("Falha ao serializar relatório", e))?;

        // Garante nome único (não sobrescreve)
        let final_path = ensure_unique_path(output_path)?;

        // Cria arquivo tar.gz
        let file = File::create(&final_path)
            .with_path_context(&final_path, "criando arquivo de relatório")?;
        let mut encoder = GzEncoder::new(BufWriter::new(file), Compression::default());
        {
            let mut tar = tar::Builder::new(&mut encoder);

            // Adiciona relatório.json
            let mut header = tar::Header::new_gnu();
            header
                .set_path("report.json")
                .map_err(|e| Error::internal_with_source("Falha ao criar header tar", e))?;
            header.set_size(json.len() as u64);
            header.set_mode(0o644);
            tar.append(&header, &json[..]).map_err(|e| {
                Error::internal_with_source("Falha ao adicionar relatório ao tar", e)
            })?;

            // Adiciona metadados em arquivo separado para fácil leitura
            let metadata_json = serde_json::to_vec_pretty(&report.metadata)
                .map_err(|e| Error::internal_with_source("Falha ao serializar metadados", e))?;
            let mut header = tar::Header::new_gnu();
            header.set_path("metadata.json").map_err(|e| {
                Error::internal_with_source("Falha ao criar header tar para metadata", e)
            })?;
            header.set_size(metadata_json.len() as u64);
            header.set_mode(0o644);
            tar.append(&header, &metadata_json[..]).map_err(|e| {
                Error::internal_with_source("Falha ao adicionar metadata ao tar", e)
            })?;

            tar.finish()
                .map_err(|e| Error::internal_with_source("Falha ao finalizar tar", e))?;
        }
        // Drop tar before finishing encoder
        encoder
            .finish()
            .map_err(|e| Error::internal_with_source("Falha ao comprimir relatório", e))?;

        tracing::info!(
            session_id = %self.session_id,
            output_path = %final_path.display(),
            events = events.len(),
            "Relatório de sessão exportado"
        );

        Ok(final_path)
    }

    /// Retorna o caminho do diretório de logs.
    pub fn log_dir(&self) -> &Path {
        &self.config.log_dir
    }
}

/// Garante que o caminho de saída não exista, adicionando sufixo numérico se necessário.
fn ensure_unique_path(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        return Ok(path.to_path_buf());
    }

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("report");
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{s}"))
        .unwrap_or_default();
    let parent = path.parent().unwrap_or(Path::new("."));

    for i in 1..10000 {
        let new_name = format!("{stem}-{i}{ext}");
        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return Ok(new_path);
        }
    }

    Err(Error::filesystem(
        "Não foi possível gerar nome único para relatório",
        Some(path.to_path_buf()),
    ))
}

/// Metadados do relatório de sessão.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub session_id: SessionId,
    pub generated_at: DateTime<Utc>,
    pub total_events: usize,
    pub sanitized: bool,
    pub config: LoggingConfig,
}

/// Relatório completo de uma sessão.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionReport {
    pub metadata: ReportMetadata,
    pub events: Vec<LogEvent>,
}

/// Layer para capturar eventos estruturados.
struct EventCaptureLayer {
    events: Arc<Mutex<Vec<LogEvent>>>,
}

impl EventCaptureLayer {
    fn new(events: Arc<Mutex<Vec<LogEvent>>>) -> Self {
        Self { events }
    }
}

impl<S> Layer<S> for EventCaptureLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        // Extrai informações do evento
        let mut fields = HashMap::new();
        let mut message = String::new();

        struct FieldVisitor<'a>(&'a mut HashMap<String, serde_json::Value>, &'a mut String);
        impl tracing::field::Visit for FieldVisitor<'_> {
            fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
                self.0
                    .insert(field.name().to_string(), serde_json::json!(value));
            }
            fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
                self.0
                    .insert(field.name().to_string(), serde_json::json!(value));
            }
            fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
                self.0
                    .insert(field.name().to_string(), serde_json::json!(value));
            }
            fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
                self.0
                    .insert(field.name().to_string(), serde_json::json!(value));
            }
            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                if field.name() == "message" {
                    self.1.push_str(value);
                } else {
                    self.0
                        .insert(field.name().to_string(), serde_json::json!(value));
                }
            }
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                self.0.insert(
                    field.name().to_string(),
                    serde_json::json!(format!("{:?}", value)),
                );
            }
        }

        let mut visitor = FieldVisitor(&mut fields, &mut message);
        event.record(&mut visitor);

        // Obtém metadados do span atual
        let span = ctx.current_span();
        let span_id = span.id().map(|id| id.into_u64());
        // parent() não existe em CurrentSpan, usamos 0 como fallback
        let parent_span_id = Option::<u64>::default();

        // Cria evento estruturado
        let metadata = span.metadata();
        let component = metadata
            .map(|m| m.module_path().unwrap_or("unknown"))
            .unwrap_or("unknown");
        let operation = metadata.map(|m| m.name()).unwrap_or("unknown");

        let log_event = LogEvent::new(
            LOGGING_INSTANCE
                .get()
                .map(|i| i.session_id)
                .unwrap_or_default(),
            component,
            operation,
            match *event.metadata().level() {
                tracing::Level::ERROR => LogLevel::Error,
                tracing::Level::WARN => LogLevel::Warn,
                tracing::Level::INFO => LogLevel::Info,
                tracing::Level::DEBUG => LogLevel::Debug,
                tracing::Level::TRACE => LogLevel::Trace,
            },
            message,
        )
        .with_fields(fields)
        .with_parent_span(parent_span_id.unwrap_or(0))
        .with_span(span_id.unwrap_or(0));

        // Armazena
        if let Ok(mut events) = self.events.lock() {
            events.push(log_event);
        }
    }
}

/// Inicializa o logging com configuração padrão baseada na Config do app.
pub fn init_logging_from_config(config: &Config) -> Result<SessionId> {
    let logging_config = LoggingConfig {
        log_dir: config.data_dir.join("logs"),
        console_log_level: config.log_level,
        file_log_level: LogLevel::Debug,
        ..Default::default()
    };
    LoggingInstance::init(logging_config)
}

/// Inicializa o logging com configuração customizada.
pub fn init_logging(logging_config: LoggingConfig) -> Result<SessionId> {
    LoggingInstance::init(logging_config)
}

/// Obtém a instância de logging atual.
pub fn current_logging() -> Option<&'static LoggingInstance> {
    LoggingInstance::current()
}

/// Registra um evento estruturado.
pub fn log_event(event: LogEvent) {
    if let Some(instance) = current_logging() {
        instance.log_event(event);
    }
}

/// Exporta relatório da sessão atual.
pub fn export_session_report(output_path: &Path) -> Result<PathBuf> {
    if let Some(instance) = current_logging() {
        instance.export_report(output_path)
    } else {
        Err(Error::invalid_state(
            "Sistema de logging não inicializado",
            Some("Chame init_logging() primeiro".to_string()),
        ))
    }
}

/// Retorna o diretório de logs atual.
pub fn current_log_dir() -> Option<&'static Path> {
    current_logging().map(|i| i.log_dir())
}

/// Sanitiza uma string de saída usando o sanitizador da sessão atual.
/// Se o logging não estiver inicializado, retorna a string original.
pub fn sanitize_output(input: &str) -> String {
    if let Some(instance) = current_logging() {
        instance.sanitizer.sanitize(input)
    } else {
        input.to_string()
    }
}

/// Sanitiza bytes de saída (para stdout/stderr binários).
pub fn sanitize_output_bytes(input: &[u8]) -> String {
    let s = String::from_utf8_lossy(input);
    sanitize_output(&s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_session_id_generation() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1, id2);
        assert!(!id1.0.is_nil());
    }

    #[test]
    fn test_default_log_dir() {
        let dir = default_log_dir();
        assert!(
            dir.ends_with("cachyos-boot-rescue-logs") || dir.ends_with("cachyos-boot-rescue/logs")
        );
    }

    #[test]
    fn test_log_event_creation() {
        let session_id = SessionId::new();
        let event = LogEvent::new(
            session_id,
            "test_component",
            "test_operation",
            LogLevel::Info,
            "Test message",
        )
        .with_field("key1", serde_json::json!("value1"))
        .with_field("key2", serde_json::json!(42));

        assert_eq!(event.session_id, session_id);
        assert_eq!(event.component, "test_component");
        assert_eq!(event.operation, "test_operation");
        assert_eq!(event.level, LogLevel::Info);
        assert_eq!(event.message, "Test message");
        assert_eq!(event.fields.get("key1"), Some(&serde_json::json!("value1")));
        assert_eq!(event.fields.get("key2"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_sanitizer_password() {
        let config = LoggingConfig::default();
        let sanitizer = Sanitizer::new(&config).unwrap();

        let input = "password=secret123";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("***REDACTED***"));
        assert!(!output.contains("secret123"));
    }

    #[test]
    fn test_sanitizer_token() {
        let config = LoggingConfig::default();
        let sanitizer = Sanitizer::new(&config).unwrap();

        let input = "api_token=abc123def456";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("***REDACTED***"));
        assert!(!output.contains("abc123def456"));
    }

    #[test]
    fn test_sanitizer_ssh_key() {
        let config = LoggingConfig::default();
        let sanitizer = Sanitizer::new(&config).unwrap();

        let input =
            "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("***REDACTED***"));
        assert!(!output.contains("MIIEpAIBAAKCAQEA"));
    }

    #[test]
    fn test_sanitizer_luks() {
        let config = LoggingConfig::default();
        let sanitizer = Sanitizer::new(&config).unwrap();

        let input = "cryptsetup luksPassphrase=mysecretpass";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("***REDACTED***"));
        assert!(!output.contains("mysecretpass"));
    }

    #[test]
    fn test_sanitizer_env_var() {
        let config = LoggingConfig::default();
        let sanitizer = Sanitizer::new(&config).unwrap();

        let input = "AWS_SECRET_ACCESS_KEY=supersecret";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("***REDACTED***"));
        assert!(!output.contains("supersecret"));
    }

    #[test]
    fn test_sanitizer_url_credentials() {
        let config = LoggingConfig::default();
        let sanitizer = Sanitizer::new(&config).unwrap();

        let input = "https://user:password@github.com/repo.git";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("***REDACTED***"));
        assert!(!output.contains("password"));
    }

    #[test]
    fn test_sanitizer_mac_address() {
        let config = LoggingConfig::default();
        let sanitizer = Sanitizer::new(&config).unwrap();

        let input = "interface eth0 mac aa:bb:cc:dd:ee:ff";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("XX:XX:XX:XX:XX:XX"));
        assert!(!output.contains("aa:bb:cc:dd:ee:ff"));
    }

    #[test]
    fn test_sanitizer_uuid() {
        let config = LoggingConfig::default();
        let sanitizer = Sanitizer::new(&config).unwrap();

        let input = "device uuid 123e4567-e89b-12d3-a456-426614174000";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"));
        assert!(!output.contains("123e4567-e89b-12d3-a456-426614174000"));
    }

    #[test]
    fn test_sanitizer_json() {
        let config = LoggingConfig::default();
        let sanitizer = Sanitizer::new(&config).unwrap();

        // Use longer secrets that match the sanitizer patterns (base64/hex)
        let mut value = serde_json::json!({
            "password": "secret12345678901234567890",  // 26 chars - matches base64 pattern
            "nested": {
                "api_key": "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6"  // 32 hex chars - matches hex pattern
            },
            "array": ["dG9rZW4xMjM0NTY3ODkwMTIzNDU2", "dG9rZW4yMjM0NTY3ODkwMTIzNDU2"]  // base64 encoded, >20 chars
        });
        sanitizer.sanitize_json(&mut value);

        assert!(
            value["password"]
                .as_str()
                .unwrap()
                .contains("***REDACTED***")
        );
        assert!(
            value["nested"]["api_key"]
                .as_str()
                .unwrap()
                .contains("***REDACTED***")
        );
        assert!(
            value["array"][0]
                .as_str()
                .unwrap()
                .contains("***REDACTED***")
        );
    }

    #[test]
    fn test_sanitizer_username() {
        let config = LoggingConfig::default();
        let sanitizer = Sanitizer::new(&config).unwrap();

        let input = "user=john";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("***REDACTED***"));
        assert!(!output.contains("john"));
    }

    #[test]
    fn test_ensure_unique_path() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("report.tar.gz");

        // Primeira vez deve retornar o mesmo caminho
        let result = ensure_unique_path(&path).unwrap();
        assert_eq!(result, path);

        // Cria o arquivo
        File::create(&path).unwrap();

        // Segunda vez deve retornar caminho com sufixo
        let result = ensure_unique_path(&path).unwrap();
        assert_ne!(result, path);
        assert!(result.to_string_lossy().contains("report.tar-1"));
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.file_log_level, LogLevel::Debug);
        assert_eq!(config.console_log_level, LogLevel::Info);
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert_eq!(config.max_files, 10);
        assert!(config.file_logging_enabled);
        assert!(config.sanitize_on_export);
        assert!(config.mask_usernames);
        assert!(config.mask_machine_ids);
    }

    #[test]
    fn test_logging_config_validate() {
        let config = LoggingConfig {
            max_file_size: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = LoggingConfig {
            max_files: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = LoggingConfig {
            extra_sanitize_patterns: vec!["".to_string()],
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = LoggingConfig {
            extra_sanitize_patterns: vec!["[invalid".to_string()],
            ..Default::default()
        };
        assert!(config.validate().is_err());

        // Config válida
        let config = LoggingConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_report_metadata_serialization() {
        let metadata = ReportMetadata {
            session_id: SessionId::new(),
            generated_at: Utc::now(),
            total_events: 42,
            sanitized: true,
            config: LoggingConfig::default(),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: ReportMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_events, 42);
        assert!(deserialized.sanitized);
    }

    #[test]
    fn test_session_report_serialization() {
        let session_id = SessionId::new();
        let event = LogEvent::new(session_id, "test", "op", LogLevel::Info, "msg");

        let report = SessionReport {
            metadata: ReportMetadata {
                session_id,
                generated_at: Utc::now(),
                total_events: 1,
                sanitized: false,
                config: LoggingConfig::default(),
            },
            events: vec![event],
        };

        let json = serde_json::to_string(&report).unwrap();
        let deserialized: SessionReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.events.len(), 1);
        assert_eq!(deserialized.events[0].message, "msg");
    }
}
