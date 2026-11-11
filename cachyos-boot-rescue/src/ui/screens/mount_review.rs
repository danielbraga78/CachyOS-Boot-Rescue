use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::StatefulWidget,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::{AppCommand, AppModel};
use crate::storage::mount_plan::MountEntryStatus;
use crate::ui::components::HelpFooter;
use crate::ui::help_footer::default_bindings;
use crate::ui::list_view::{ListView, ListViewState};
use crate::ui::screens::Screen;

/// Mount review screen - shows the mount plan before execution.
#[derive(Debug, Default)]
pub struct MountReviewScreen {
    list_state: ListViewState,
    confirmed: bool,
}

impl MountReviewScreen {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Screen for MountReviewScreen {
    fn render(&mut self, area: Rect, f: &mut Frame, app: &AppModel) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Revisão do Plano de Montagem")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Revisão "));
        title.render(chunks[0], f.buffer_mut());

        // Build list items from mount plan
        let mut items = Vec::new();

        if let Some(plan) = &app.shared.mount_plan {
            // Existing mounts
            items.push("=== MONTAGENS EXISTENTES ===".to_string());
            if let Some(report) = &app.shared.discovery_report {
                for device in &report.devices {
                    if let Some(mountpoint) = &device.mountpoint {
                        items.push(format!(
                            "  {} -> {} ({:?})",
                            device.path.display(),
                            mountpoint.display(),
                            device.filesystem
                        ));
                    }
                }
            }
            items.push("".to_string());

            // Planned mounts
            items.push("=== MONTAGENS PLANEJADAS ===".to_string());
            for entry in &plan.entries {
                let status = match entry.status {
                    MountEntryStatus::Planned => "[PLANEJADO]",
                    MountEntryStatus::SkippedNoauto => "[IGNORADO: noauto]",
                    MountEntryStatus::SkippedNofailMissing => {
                        "[IGNORADO: nofail, dispositivo ausente]"
                    }
                    MountEntryStatus::SkippedAmbiguous => "[IGNORADO: dispositivo ambíguo]",
                    MountEntryStatus::AlreadyMounted => "[JÁ MONTADO]",
                    MountEntryStatus::Failed => "[FALHOU]",
                };

                let subvol = entry
                    .subvolume
                    .as_deref()
                    .map_or(String::new(), |s| format!(" subvol={}", s));
                let fs_type = format!("{:?}", entry.fs_type);

                items.push(format!(
                    "{} {} -> {} ({}){}",
                    status,
                    entry.device_path.display(),
                    entry.mount_point.display(),
                    fs_type,
                    subvol
                ));
            }

            // Warnings
            if !plan.warnings.is_empty() {
                items.push("".to_string());
                items.push("=== AVISOS ===".to_string());
                for warning in &plan.warnings {
                    items.push(format!("⚠ {}", warning));
                }
            }

            // Errors
            if !plan.errors.is_empty() {
                items.push("".to_string());
                items.push("=== ERROS (BLOQUEANTES) ===".to_string());
                for error in &plan.errors {
                    items.push(format!("✗ {}", error));
                }
            }
        }

        if items.is_empty() {
            items.push("Nenhum plano de montagem disponível".to_string());
        }

        self.list_state.items = items;

        let list_view = ListView::new().title("Plano de Montagem").borders(true);
        list_view.render(chunks[1], f.buffer_mut(), &mut self.list_state);

        // Confirmation prompt
        let confirm_text = if self.confirmed {
            "Digite 'sim' para confirmar a montagem"
        } else {
            "Pressione 'y' para confirmar montagem, 'r' para refazer descoberta, Esc para voltar"
        };

        let confirm = Paragraph::new(confirm_text)
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Confirmação "),
            );
        confirm.render(chunks[2], f.buffer_mut());

        // Help footer
        let footer = HelpFooter::new()
            .bindings(default_bindings::mount_review())
            .borders(true);
        footer.render(chunks[2], f.buffer_mut());
    }

    fn handle_input(&mut self, key: KeyEvent, _app: &AppModel) -> Option<AppCommand> {
        match key.code {
            KeyCode::Char('y') => {
                self.confirmed = true;
                Some(AppCommand::ExecuteMount {
                    plan: _app.shared.mount_plan.as_ref()?.clone(),
                })
            }
            KeyCode::Char('r') => Some(AppCommand::RunDiscovery),
            KeyCode::Esc => Some(AppCommand::Quit),
            _ => None,
        }
    }
}
