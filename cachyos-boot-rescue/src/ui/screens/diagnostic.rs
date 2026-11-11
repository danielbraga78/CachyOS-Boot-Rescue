//! Tela de diagnóstico de bootloader.

use crate::app::state::SharedData;
use crate::diagnostic::types::{DiagnosticReport, Finding, Severity};
use crate::ui::components::HelpFooter;
use crate::ui::help_footer::default_bindings;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Widget,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

/// Renderiza a tela de diagnóstico.
pub fn render_diagnostic(f: &mut Frame, area: Rect, shared: &SharedData) {
    let run = match &shared.diagnostic_report {
        Some(r) => r,
        None => {
            let msg = Paragraph::new("Nenhum diagnóstico executado")
                .block(Block::default().borders(Borders::ALL).title("Diagnóstico"))
                .alignment(Alignment::Center);
            f.render_widget(msg, area);
            return;
        }
    };

    let report = &run.report;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Summary
            Constraint::Length(3), // Divider
            Constraint::Min(5),    // Findings list
            Constraint::Length(3), // Help footer
        ])
        .split(area);

    // Header
    render_header(f, chunks[0], report);

    // Summary
    render_summary(f, chunks[1], report);

    // Findings
    render_findings(f, chunks[3], report);

    // Help footer
    let footer = HelpFooter::new()
        .bindings(default_bindings::diagnostic())
        .borders(true);
    footer.render(chunks[4], f.buffer_mut());
}

fn render_header(f: &mut Frame, area: Rect, report: &DiagnosticReport) {
    let env = &report.environment;
    let firmware_str = match env.firmware_mode {
        crate::diagnostic::types::FirmwareMode::Uefi => "UEFI",
        crate::diagnostic::types::FirmwareMode::BiosLegacy => "BIOS Legacy",
        crate::diagnostic::types::FirmwareMode::Unknown => "Desconhecido",
    };

    let bootloader_str = match env.primary_bootloader {
        Some(crate::diagnostic::types::BootloaderType::Grub) => "GRUB",
        Some(crate::diagnostic::types::BootloaderType::Limine) => "Limine",
        Some(crate::diagnostic::types::BootloaderType::SystemdBoot) => "systemd-boot",
        Some(crate::diagnostic::types::BootloaderType::RefInd) => "RefInd",
        Some(crate::diagnostic::types::BootloaderType::WindowsBootManager) => {
            "Windows Boot Manager"
        }
        Some(crate::diagnostic::types::BootloaderType::Unknown) => "Desconhecido",
        None => "Não detectado",
    };

    let layout_str = match env.boot_layout {
        crate::diagnostic::types::BootLayout::SeparateBootEfi => "/boot separado, ESP em /boot/efi",
        crate::diagnostic::types::BootLayout::UnifiedBootOnEsp => "/boot na ESP (unificado)",
        crate::diagnostic::types::BootLayout::EfiAtEfiBootSeparate => "ESP em /efi, /boot separado",
        crate::diagnostic::types::BootLayout::BiosLegacySeparateBoot => {
            "BIOS Legacy, /boot separado"
        }
        crate::diagnostic::types::BootLayout::BiosLegacyBootOnRoot => "BIOS Legacy, /boot na raiz",
        crate::diagnostic::types::BootLayout::Unknown => "Desconhecido",
    };

    let distro = env.distribution.as_deref().unwrap_or("Desconhecida");
    let version = env.distribution_version.as_deref().unwrap_or("");

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Distro: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} {}", distro, version)),
            Span::raw("  |  "),
            Span::styled("Firmware: ", Style::default().fg(Color::Cyan)),
            Span::raw(firmware_str),
            Span::raw("  |  "),
            Span::styled("Bootloader: ", Style::default().fg(Color::Cyan)),
            Span::raw(bootloader_str),
        ]),
        Line::from(vec![
            Span::styled("Layout: ", Style::default().fg(Color::Cyan)),
            Span::raw(layout_str),
            Span::raw("  |  "),
            Span::styled("Kernels: ", Style::default().fg(Color::Cyan)),
            Span::raw(env.kernels.len().to_string()),
            Span::raw("  |  "),
            Span::styled("Initramfs: ", Style::default().fg(Color::Cyan)),
            Span::raw(env.initramfs.len().to_string()),
        ]),
        Line::from(vec![
            Span::styled("Duração: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}ms", report.duration_ms)),
            Span::raw("  |  "),
            Span::styled("Sessão: ", Style::default().fg(Color::Cyan)),
            Span::raw(&report.session_id[..8.min(report.session_id.len())]),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Diagnóstico de Bootloader"),
    )
    .alignment(Alignment::Left);

    f.render_widget(header, area);
}

fn render_summary(f: &mut Frame, area: Rect, report: &DiagnosticReport) {
    let summary = &report.summary;

    let critical_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
    let error_style = Style::default().fg(Color::Red);
    let warning_style = Style::default().fg(Color::Yellow);
    let info_style = Style::default().fg(Color::Green);

    let lines = vec![
        Line::from(vec![
            Span::styled("Total: ", Style::default().fg(Color::Cyan)),
            Span::raw(summary.total.to_string()),
            Span::raw("  |  "),
            Span::styled("Críticos: ", critical_style),
            Span::raw(
                summary
                    .by_severity
                    .get(&Severity::Critical)
                    .copied()
                    .unwrap_or(0)
                    .to_string(),
            ),
            Span::raw("  |  "),
            Span::styled("Erros: ", error_style),
            Span::raw(
                summary
                    .by_severity
                    .get(&Severity::Error)
                    .copied()
                    .unwrap_or(0)
                    .to_string(),
            ),
            Span::raw("  |  "),
            Span::styled("Avisos: ", warning_style),
            Span::raw(
                summary
                    .by_severity
                    .get(&Severity::Warning)
                    .copied()
                    .unwrap_or(0)
                    .to_string(),
            ),
            Span::raw("  |  "),
            Span::styled("Info: ", info_style),
            Span::raw(
                summary
                    .by_severity
                    .get(&Severity::Info)
                    .copied()
                    .unwrap_or(0)
                    .to_string(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Categorias: ", Style::default().fg(Color::Cyan)),
            Span::raw(format_categories(summary)),
        ]),
    ];

    let summary_widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Resumo"))
        .wrap(Wrap { trim: true });

    f.render_widget(summary_widget, area);
}

fn format_categories(summary: &crate::diagnostic::types::DiagnosticSummary) -> String {
    let mut cats: Vec<String> = summary
        .by_category
        .iter()
        .map(|(cat, count)| format!("{:?}: {}", cat, count))
        .collect();
    cats.sort();
    cats.join(" | ")
}

fn render_findings(f: &mut Frame, area: Rect, report: &DiagnosticReport) {
    let findings = &report.findings;

    if findings.is_empty() {
        let msg = Paragraph::new("Nenhum achado")
            .block(Block::default().borders(Borders::ALL).title("Achados"))
            .alignment(Alignment::Center);
        f.render_widget(msg, area);
        return;
    }

    // Cria itens para lista
    let items: Vec<ListItem> = findings
        .iter()
        .map(|finding| {
            let severity_color = match finding.severity {
                Severity::Critical => Color::Red,
                Severity::Error => Color::Red,
                Severity::Warning => Color::Yellow,
                Severity::Info => Color::Green,
            };

            let category_str = format!("{:?}", finding.id.category());
            let severity_str = format!("{:?}", finding.severity);

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        finding.title.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("[{}]", severity_str),
                        Style::default()
                            .fg(severity_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("[{}]", category_str),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(vec![Span::raw(finding.description.clone())]),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Achados (↑↓ navegar, Enter detalhes)"),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_widget(list, area);
}

/// Renderiza detalhes de um achado específico em modal.
pub fn render_finding_detail(f: &mut Frame, area: Rect, finding: &Finding) {
    let popup_area = centered_rect(80, 80, area);
    f.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title + severity
            Constraint::Min(5),    // Description
            Constraint::Length(3), // Divider
            Constraint::Min(3),    // Evidence
            Constraint::Length(3), // Divider
            Constraint::Min(5),    // Recommendations
        ])
        .split(popup_area);

    // Title
    let severity_color = match finding.severity {
        Severity::Critical => Color::Red,
        Severity::Error => Color::Red,
        Severity::Warning => Color::Yellow,
        Severity::Info => Color::Green,
    };

    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                finding.title.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("ID: {:?}", finding.id),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Severidade: {:?}", finding.severity),
                Style::default()
                    .fg(severity_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Categoria: {:?}", finding.id.category()),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![Span::raw(if finding.verified_in_chroot {
            "✓ Verificado no chroot"
        } else {
            "⚠ Apenas host"
        })]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Detalhes do Achado"),
    )
    .wrap(Wrap { trim: true });

    f.render_widget(title, chunks[0]);

    // Description
    let desc = Paragraph::new(finding.description.clone())
        .block(Block::default().borders(Borders::ALL).title("Descrição"))
        .wrap(Wrap { trim: true });
    f.render_widget(desc, chunks[1]);

    // Evidence
    let evidence_text = if finding.evidence.is_empty() {
        "Nenhuma evidência registrada".to_string()
    } else {
        finding
            .evidence
            .iter()
            .map(|e| {
                format!(
                    "• {}: {} (esperado: {:?}, observado: {:?})",
                    e.kind, e.description, e.expected, e.observed
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let evidence = Paragraph::new(evidence_text)
        .block(Block::default().borders(Borders::ALL).title("Evidências"))
        .wrap(Wrap { trim: true });
    f.render_widget(evidence, chunks[3]);

    // Recommendations
    let rec_text = if finding.recommendations.is_empty() {
        "Nenhuma recomendação".to_string()
    } else {
        finding
            .recommendations
            .iter()
            .map(|r| {
                format!(
                    "• [{}] {} (Risco: {:?}, Mutável: {})",
                    r.id, r.action, r.risk, r.is_mutating
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let recommendations = Paragraph::new(rec_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Recomendações"),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(recommendations, chunks[5]);
}

/// Helper para criar retângulo centralizado.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::types::*;

    #[test]
    fn test_format_categories() {
        let mut summary = DiagnosticSummary::default();
        summary.by_category.insert(DiagnosticCategory::Grub, 2);
        summary.by_category.insert(DiagnosticCategory::EfiLayout, 1);
        let result = format_categories(&summary);
        assert!(result.contains("Grub"));
        assert!(result.contains("EfiLayout"));
    }
}
