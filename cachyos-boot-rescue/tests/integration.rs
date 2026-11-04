//! Testes de integração para a Fase 1 - Fundação do projeto.
//!
//! Verifica o comportamento de ponta a ponta da aplicação CLI,
//! configuração, logging e tratamento de erros.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

// Inclui testes da Fase 3 - Descoberta
#[path = "discovery/mod.rs"]
mod discovery_tests;

/// Obtém o caminho do binário compilado.
fn binary_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("target/debug/cachyos-boot-rescue")
}

/// Testa que a aplicação executa e mostra a mensagem de boas-vindas.
#[test]
fn test_run_shows_welcome() {
    let output = Command::new(binary_path())
        .output()
        .expect("Falha ao executar binário");

    assert!(
        output.status.success(),
        "Aplicação falhou: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cachyos-boot-rescue v0.1.0"));
    assert!(stdout.contains("Diagnóstico e recuperação"));
    assert!(stdout.contains("Fase 1 - Fundação"));
}

/// Testa que --version mostra a versão e sai.
#[test]
fn test_version_flag() {
    let output = Command::new(binary_path())
        .arg("--version")
        .output()
        .expect("Falha ao executar binário");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cachyos-boot-rescue 0.1.0"));
}

/// Testa que --help mostra a ajuda e sai.
#[test]
fn test_help_flag() {
    let output = Command::new(binary_path())
        .arg("--help")
        .output()
        .expect("Falha ao executar binário");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CachyOS Boot Rescue"));
    assert!(stdout.contains("--config"));
    assert!(stdout.contains("--log-level"));
    assert!(stdout.contains("--interface"));
    assert!(stdout.contains("--color"));
    assert!(stdout.contains("--no-progress"));
    assert!(stdout.contains("--log-dir"));
    assert!(stdout.contains("--file-logging"));
    assert!(stdout.contains("--file-log-level"));
    assert!(stdout.contains("--export-report"));
    assert!(stdout.contains("--show-log-dir"));
    assert!(stdout.contains("--version"));
}

/// Testa carregamento de configuração TOML válida.
#[test]
fn test_config_file_loading() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let config_content = r#"
log_level = "debug"
data_dir = "/tmp/test-data"
interface_mode = "cli"
color = "always"
progress = false
"#;

    fs::write(&config_path, config_content).unwrap();

    let output = Command::new(binary_path())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("Falha ao executar binário");

    assert!(
        output.status.success(),
        "Falha: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("debug")); // log_level
    assert!(stdout.contains("always")); // color
    assert!(stdout.contains("não")); // progress = false
}

/// Testa que CLI sobrescreve configuração do arquivo.
#[test]
fn test_cli_overrides_config_file() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    // Configuração no arquivo: log_level = "warn"
    fs::write(&config_path, "log_level = \"warn\"\n").unwrap();

    // CLI sobrescreve para debug
    let output = Command::new(binary_path())
        .arg("--config")
        .arg(&config_path)
        .arg("--log-level")
        .arg("debug")
        .output()
        .expect("Falha ao executar binário");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("debug"));
    assert!(!stdout.contains("warn"));
}

/// Testa validação de nível de log inválido.
#[test]
fn test_invalid_log_level() {
    let output = Command::new(binary_path())
        .arg("--log-level")
        .arg("invalid")
        .output()
        .expect("Falha ao executar binário");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("inválido") || stderr.contains("invalid"));
}

/// Testa validação de modo de interface inválido.
#[test]
fn test_invalid_interface_mode() {
    let output = Command::new(binary_path())
        .arg("--interface")
        .arg("gui")
        .output()
        .expect("Falha ao executar binário");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("inválido") || stderr.contains("invalid"));
}

/// Testa validação de escolha de cor inválida.
#[test]
fn test_invalid_color_choice() {
    let output = Command::new(binary_path())
        .arg("--color")
        .arg("invalid")
        .output()
        .expect("Falha ao executar binário");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("inválido") || stderr.contains("invalid"));
}

/// Testa --show-log-dir mostra o diretório de logs.
#[test]
fn test_show_log_dir() {
    let output = Command::new(binary_path())
        .arg("--show-log-dir")
        .output()
        .expect("Falha ao executar binário");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Deve mostrar um caminho (não a mensagem de logging não inicializado)
    assert!(!stdout.contains("Logging não inicializado"));
    assert!(stdout.contains("cachyos-boot-rescue"));
}

/// Testa --export-report exporta relatório corretamente.
#[test]
fn test_export_report_without_logging() {
    let dir = tempdir().unwrap();
    let output_path = dir.path().join("report.tar.gz");

    let output = Command::new(binary_path())
        .arg("--export-report")
        .arg(&output_path)
        .output()
        .expect("Falha ao executar binário");

    // Deve ter sucesso porque logging é inicializado automaticamente
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Relatório exportado para:"));
    // Verifica que o arquivo foi criado
    assert!(output_path.exists() || dir.path().join("report.tar-1.gz").exists());
}

/// Testa configuração com data_dir perigoso é rejeitada.
#[test]
fn test_dangerous_data_dir_rejected() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    fs::write(&config_path, "data_dir = \"/boot/grub\"\n").unwrap();

    let output = Command::new(binary_path())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("Falha ao executar binário");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("não pode estar em") || stderr.contains("dangerous"));
}

/// Testa que aplicação não executa operações administrativas na Fase 1.
#[test]
fn test_no_disk_operations_in_phase1() {
    let output = Command::new(binary_path())
        .output()
        .expect("Falha ao executar binário");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Não deve conter menções a operações de disco
    assert!(!stdout.contains("lsblk"));
    assert!(!stdout.contains("blkid"));
    assert!(!stdout.contains("mount"));
    assert!(!stdout.contains("grub"));
    assert!(!stdout.contains("efibootmgr"));
    assert!(!stdout.contains("mkfs"));
    assert!(!stdout.contains("wipefs"));
    assert!(!stdout.contains("parted"));
    assert!(!stdout.contains("fdisk"));
    assert!(!stdout.contains("sgdisk"));
}

/// Testa carregamento de configuração com logging customizado.
#[test]
fn test_config_with_custom_logging() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let config_content = r#"
log_level = "info"
data_dir = "/tmp/test-data"
interface_mode = "cli"
color = "auto"
progress = true

[logging]
log_dir = "/tmp/custom-logs"
file_log_level = "trace"
console_log_level = "debug"
max_file_size = 5242880
max_files = 5
file_logging_enabled = true
sanitize_on_export = true
extra_sanitize_patterns = []
mask_usernames = true
mask_machine_ids = true
"#;

    fs::write(&config_path, config_content).unwrap();

    let output = Command::new(binary_path())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("Falha ao executar binário");

    assert!(
        output.status.success(),
        "Falha: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Testa que data_dir vazio é rejeitado.
#[test]
fn test_empty_data_dir_rejected() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    fs::write(&config_path, "data_dir = \"\"\n").unwrap();

    let output = Command::new(binary_path())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("Falha ao executar binário");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("não pode ser vazio") || stderr.contains("empty"));
}

/// Testa que raiz do sistema como data_dir é rejeitada.
#[test]
fn test_root_data_dir_rejected() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    fs::write(&config_path, "data_dir = \"/\"\n").unwrap();

    let output = Command::new(binary_path())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("Falha ao executar binário");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("raiz do sistema") || stderr.contains("root"));
}
