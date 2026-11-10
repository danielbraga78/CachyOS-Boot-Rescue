use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::{AppCommand, AppModel};
use crate::ui::components::HelpFooter;
use crate::ui::help_footer::default_bindings;
use crate::ui::screens::Screen;

/// Discovery screen - shows progress of hardware discovery.
#[derive(Debug, Default)]
pub struct DiscoveryScreen {
    pub spinner_state: usize,
}

impl DiscoveryScreen {
    pub fn new() -> Self {
        Self { spinner_state: 0 }
    }
}

impl Screen for DiscoveryScreen {
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
        let title = Paragraph::new("Descoberta de Hardware")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Descoberta "));
        title.render(chunks[0], f.buffer_mut());

        // Spinner animation
        self.spinner_state = (self.spinner_state + 1) % 4;
        let spinner = match self.spinner_state {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            3 => "⠸",
            _ => "⠋",
        };

        let mut content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    spinner,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Descobrindo hardware, partições e instalações Linux..."),
            ]),
            Line::from(""),
        ];

        if let Some(report) = &app.shared.discovery_report {
            content.push(Line::from(""));
            content.push(Line::from(vec![
                Span::styled(
                    "Dispositivos encontrados: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(report.devices.len().to_string()),
            ]));
            content.push(Line::from(vec![
                Span::styled(
                    "Instalações Linux: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(report.linux_candidates.len().to_string()),
            ]));
            content.push(Line::from(vec![
                Span::styled(
                    "Candidatas ESP: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(report.esp_candidates.len().to_string()),
            ]));
        }

        let content_widget = Paragraph::new(content)
            .block(Block::default().borders(Borders::ALL).title(" Progresso "))
            .style(Style::default().bg(Color::DarkGray))
            .alignment(Alignment::Center);
        content_widget.render(chunks[1], f.buffer_mut());

        // Help footer
        let footer = HelpFooter::new()
            .bindings(default_bindings::discovery())
            .borders(true);
        footer.render(chunks[2], f.buffer_mut());
    }

    fn handle_input(&mut self, key: KeyEvent, _app: &AppModel) -> Option<AppCommand> {
        match key.code {
            KeyCode::Esc => Some(AppCommand::Quit),
            _ => None,
        }
    }
}
