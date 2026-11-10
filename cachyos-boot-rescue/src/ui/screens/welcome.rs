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

/// Welcome screen.
#[derive(Debug, Default)]
pub struct WelcomeScreen;

impl WelcomeScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for WelcomeScreen {
    fn render(&mut self, area: Rect, f: &mut Frame, _app: &AppModel) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(area);

        // Title
        let title = Paragraph::new("cachyos-boot-rescue")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Bem-vindo "));
        title.render(chunks[0], f.buffer_mut());

        // Main content
        let content = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Diagnóstico e recuperação de inicialização Linux",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "Foco: CachyOS / Arch Linux",
                Style::default().fg(Color::DarkGray),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Versão: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(env!("CARGO_PKG_VERSION")),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Pressione ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " para iniciar a descoberta de hardware",
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Line::from(vec![
                Span::styled("Pressione ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "Esc",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ou ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "q",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" para sair", Style::default().fg(Color::DarkGray)),
            ]),
        ];

        let content_widget = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Informações "),
            )
            .style(Style::default().bg(Color::DarkGray))
            .alignment(Alignment::Center);
        content_widget.render(chunks[1], f.buffer_mut());

        // Help footer
        let footer = HelpFooter::new()
            .bindings(default_bindings::welcome())
            .borders(true);
        footer.render(chunks[2], f.buffer_mut());
    }

    fn handle_input(&mut self, key: KeyEvent, _app: &AppModel) -> Option<AppCommand> {
        match key.code {
            KeyCode::Enter => Some(AppCommand::RunDiscovery),
            KeyCode::Esc | KeyCode::Char('q') => Some(AppCommand::Quit),
            _ => None,
        }
    }
}
