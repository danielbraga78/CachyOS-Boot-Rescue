//! Main UI layout for the TUI.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::AppModel;
use crate::ui::screens::Screen;

/// Main layout with header, body, footer, and help.
pub fn draw_main_layout(f: &mut Frame, app: &AppModel, screen: &mut dyn Screen) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Body (screen)
            Constraint::Length(1), // Separator
            Constraint::Length(3), // Footer/Help
        ])
        .split(f.area());

    // Header
    draw_header(f, app, chunks[0]);

    // Body - delegate to current screen
    screen.render(chunks[1], f, app);

    // Separator
    draw_separator(f, chunks[2]);

    // Footer - Help
    draw_footer(f, app, chunks[3]);
}

fn draw_header(f: &mut Frame, app: &AppModel, area: Rect) {
    let session_short = if app.shared.session_id.len() > 8 {
        &app.shared.session_id[..8]
    } else {
        &app.shared.session_id
    };

    let firmware = match app.shared.discovery_report.as_ref().map(|r| r.uefi_state) {
        Some(crate::discovery::types::UefiState::Uefi) => "UEFI",
        Some(crate::discovery::types::UefiState::BiosLegacy) => "BIOS",
        Some(crate::discovery::types::UefiState::Unknown) => "DESCONHECIDO",
        None => "N/A",
    };

    let header_text = Line::from(vec![
        Span::styled(
            "cachyos-boot-rescue ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled(
            format!("v{} ", env!("CARGO_PKG_VERSION")),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled("| ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Sessão: {} ", session_short),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled("| ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Firmware: {}", firmware),
            Style::default().fg(Color::Green),
        ),
    ]);

    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).title(" Cabeçalho "))
        .style(Style::default().bg(Color::DarkGray));
    header.render(area, f.buffer_mut());
}

fn draw_separator(f: &mut Frame, area: Rect) {
    let sep =
        Paragraph::new("─".repeat(area.width as usize)).style(Style::default().fg(Color::DarkGray));
    sep.render(area, f.buffer_mut());
}

fn draw_footer(f: &mut Frame, app: &AppModel, area: Rect) {
    use crate::ui::components::HelpFooter;
    use crate::ui::help_footer::default_bindings;

    let bindings = match app.state {
        crate::app::AppState::Welcome => default_bindings::welcome(),
        crate::app::AppState::Discovery => default_bindings::discovery(),
        crate::app::AppState::InstallSelect => default_bindings::install_select(),
        crate::app::AppState::MountReview => default_bindings::mount_review(),
        crate::app::AppState::MountProgress { .. } => default_bindings::mount_progress(),
        crate::app::AppState::ChrootActive => default_bindings::chroot_active(),
        crate::app::AppState::LogsReport => default_bindings::logs_report(),
        crate::app::AppState::Error { .. } => default_bindings::error(),
        crate::app::AppState::Quit => default_bindings::welcome(),
    };

    let footer = HelpFooter::new().bindings(bindings).borders(true);
    footer.render(area, f.buffer_mut());
}
