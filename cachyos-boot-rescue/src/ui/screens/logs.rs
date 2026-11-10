use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::StatefulWidget,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::{AppCommand, AppModel};
use crate::ui::components::{HelpFooter, LogEntry, LogView, LogViewState};
use crate::ui::help_footer::default_bindings;
use crate::ui::screens::Screen;

/// Logs and reports screen.
#[derive(Debug, Default)]
pub struct LogsScreen {
    log_state: LogViewState,
}

impl LogsScreen {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Screen for LogsScreen {
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
        let title = Paragraph::new("Logs e Relatórios")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Logs "));
        title.render(chunks[0], f.buffer_mut());

        // Build log entries from logging system
        // For now, create some sample entries
        let entries = vec![
            LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                level: "INFO".to_string(),
                component: "app".to_string(),
                message: "Aplicação iniciada".to_string(),
            },
            LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                level: "INFO".to_string(),
                component: "discovery".to_string(),
                message: "Descoberta de hardware concluída".to_string(),
            },
        ];

        self.log_state.entries = entries;
        self.log_state.title = "Logs da Sessão".to_string();
        self.log_state.show_borders = true;

        StatefulWidget::render(LogView, chunks[1], f.buffer_mut(), &mut self.log_state);

        // Help footer
        let footer = HelpFooter::new()
            .bindings(default_bindings::logs_report())
            .borders(true);
        footer.render(chunks[2], f.buffer_mut());
    }

    fn handle_input(&mut self, key: KeyEvent, _app: &AppModel) -> Option<AppCommand> {
        match key.code {
            KeyCode::Up => {
                self.log_state.previous();
                None
            }
            KeyCode::Down => {
                self.log_state.next();
                None
            }
            KeyCode::Char('/') => {
                // Filter - not implemented yet
                None
            }
            KeyCode::Char('e') => Some(AppCommand::ExportReport {
                output_path: std::path::PathBuf::from("/tmp/report.tar.gz"),
            }),
            KeyCode::Esc => Some(AppCommand::Quit),
            _ => None,
        }
    }
}
