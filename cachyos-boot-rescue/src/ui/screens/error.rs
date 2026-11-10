use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::{AppCommand, AppModel, AppState};
use crate::ui::components::HelpFooter;
use crate::ui::error_dialog::{ErrorDialogAction, ErrorDialogState};
use crate::ui::help_footer::default_bindings;
use crate::ui::screens::Screen;

/// Error screen.
#[derive(Debug, Default)]
pub struct ErrorScreen {
    dialog_state: ErrorDialogState,
}

impl ErrorScreen {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Screen for ErrorScreen {
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
        let title = Paragraph::new("Erro")
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Erro "));
        title.render(chunks[0], f.buffer_mut());

        // Update dialog state from app
        if let AppState::Error { message } = &app.state {
            self.dialog_state.message = message.clone();
            self.dialog_state.session_id = app.shared.session_id.clone();
        }

        ratatui::widgets::Widget::render(&self.dialog_state, chunks[1], f.buffer_mut());

        // Help footer
        let footer = HelpFooter::new()
            .bindings(default_bindings::error())
            .borders(true);
        footer.render(chunks[2], f.buffer_mut());
    }

    fn handle_input(&mut self, key: KeyEvent, _app: &AppModel) -> Option<AppCommand> {
        match key.code {
            KeyCode::Left => {
                self.dialog_state.previous();
                None
            }
            KeyCode::Right => {
                self.dialog_state.next();
                None
            }
            KeyCode::Enter => {
                match self.dialog_state.action {
                    ErrorDialogAction::Export => Some(AppCommand::ExportReport {
                        output_path: std::path::PathBuf::from("/tmp/report.tar.gz"),
                    }),
                    ErrorDialogAction::Back => {
                        Some(AppCommand::Quit) // Go back to welcome
                    }
                }
            }
            KeyCode::Esc => Some(AppCommand::Quit),
            _ => None,
        }
    }
}
