use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::{AppCommand, AppModel, AppState, OperationState};
use crate::ui::components::{HelpFooter, ProgressBarState};
use crate::ui::help_footer::default_bindings;
use crate::ui::screens::Screen;

/// Mount progress screen.
#[derive(Debug, Default)]
pub struct MountProgressScreen {
    progress_state: ProgressBarState,
}

impl MountProgressScreen {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Screen for MountProgressScreen {
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
        let title = Paragraph::new("Progresso da Montagem")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Montagem "));
        title.render(chunks[0], f.buffer_mut());

        // Update progress state from app
        if let AppState::MountProgress { stage } = app.state {
            self.progress_state.stage = stage;
        }

        // Progress bar - render ProgressBarState directly as Widget
        ratatui::widgets::Widget::render(&self.progress_state, chunks[1], f.buffer_mut());

        // Help footer
        let footer = HelpFooter::new()
            .bindings(default_bindings::mount_progress())
            .borders(true);
        footer.render(chunks[2], f.buffer_mut());
    }

    fn handle_input(&mut self, key: KeyEvent, app: &AppModel) -> Option<AppCommand> {
        match key.code {
            KeyCode::Esc => match &app.operation {
                OperationState::Running { op_id, .. } => {
                    Some(AppCommand::CancelOperation { op_id: *op_id })
                }
                _ => Some(AppCommand::Quit),
            },
            _ => None,
        }
    }
}
