use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::StatefulWidget,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::{AppCommand, AppModel};
use crate::diagnostic::runner::full_diagnostic_config;
use crate::ui::components::HelpFooter;
use crate::ui::help_footer::default_bindings;
use crate::ui::list_view::{ListView, ListViewState};
use crate::ui::screens::Screen;

/// Install selection screen.
#[derive(Debug, Default)]
pub struct InstallSelectScreen {
    list_state: ListViewState,
}

impl InstallSelectScreen {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Screen for InstallSelectScreen {
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
        let title = Paragraph::new("Selecionar Instalação Linux")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Seleção "));
        title.render(chunks[0], f.buffer_mut());

        // Build list items from discovery report
        let mut items = Vec::new();
        if let Some(report) = &app.shared.discovery_report {
            for candidate in report.linux_candidates.iter() {
                let distro = candidate.distribution.as_deref().unwrap_or("Desconhecido");
                let version = candidate.version.as_deref().unwrap_or("");
                let device = candidate.root_device.path.display().to_string();
                let fs = format!("{:?}", candidate.root_filesystem.fstype);
                let confidence = candidate.confidence;

                let item = format!(
                    "{} {} | {} | {} | Conf: {}%",
                    distro, version, device, fs, confidence
                );
                items.push(item);
            }
        }

        if items.is_empty() {
            items.push("Nenhuma instalação Linux encontrada".to_string());
        }

        self.list_state.items = items;

        // Sync selected index with app
        self.list_state.selected = app.selected_install;

        let list_view = ListView::new()
            .title("Instalações Encontradas")
            .borders(true);
        list_view.render(chunks[1], f.buffer_mut(), &mut self.list_state);

        // Help footer
        let footer = HelpFooter::new()
            .bindings(default_bindings::install_select())
            .borders(true);
        footer.render(chunks[2], f.buffer_mut());
    }

    fn handle_input(&mut self, key: KeyEvent, _app: &AppModel) -> Option<AppCommand> {
        match key.code {
            KeyCode::Up => {
                self.list_state.previous();
                None
            }
            KeyCode::Down => {
                self.list_state.next();
                None
            }
            KeyCode::Enter => self
                .list_state
                .selected
                .map(|idx| AppCommand::BuildMountPlan { candidate_idx: idx }),
            KeyCode::Char('d') => self
                .list_state
                .selected
                .map(|idx| AppCommand::RunDiagnostic {
                    candidate_idx: idx,
                    config: full_diagnostic_config(None),
                }),
            KeyCode::Char('r') => Some(AppCommand::RunDiscovery),
            KeyCode::Esc => Some(AppCommand::Quit),
            _ => None,
        }
    }
}
