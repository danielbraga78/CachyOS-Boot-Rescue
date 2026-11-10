use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, StatefulWidget, Widget},
};

/// State for the error dialog.
#[derive(Debug, Default, Clone)]
pub struct ErrorDialogState {
    pub message: String,
    pub session_id: String,
    pub action: ErrorDialogAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorDialogAction {
    Export,
    #[default]
    Back,
}

impl ErrorDialogState {
    pub fn new(message: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            session_id: session_id.into(),
            action: ErrorDialogAction::Back,
        }
    }

    pub fn next(&mut self) {
        self.action = match self.action {
            ErrorDialogAction::Export => ErrorDialogAction::Back,
            ErrorDialogAction::Back => ErrorDialogAction::Export,
        };
    }

    pub fn previous(&mut self) {
        self.next();
    }
}

/// An error dialog widget.
#[derive(Debug, Default)]
pub struct ErrorDialog {
    pub title: String,
}

impl ErrorDialog {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
        }
    }
}

impl StatefulWidget for ErrorDialog {
    type State = ErrorDialogState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        (&*state).render(area, buf);
    }
}

impl Widget for &ErrorDialogState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let dialog_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(3),
            ])
            .split(area)[1];

        let dialog_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Percentage(70),
                Constraint::Percentage(15),
            ])
            .split(dialog_area)[1];

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Erro - Sessão: {} ", self.session_id))
            .style(Style::default().bg(Color::DarkGray).fg(Color::Red));
        block.render(dialog_area, buf);

        // Message
        let message_area = Rect {
            x: dialog_area.x + 2,
            y: dialog_area.y + 1,
            width: dialog_area.width.saturating_sub(4),
            height: dialog_area.height.saturating_sub(5),
        };

        let message = Paragraph::new(self.message.clone())
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });
        message.render(message_area, buf);

        // Buttons
        let button_y = dialog_area.y + dialog_area.height - 2;
        let button_width = 12;
        let total_width = button_width * 2 + 4;
        let start_x = dialog_area.x + (dialog_area.width.saturating_sub(total_width)) / 2;

        // Export button
        let export_style = if matches!(self.action, ErrorDialogAction::Export) {
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let export_text = if matches!(self.action, ErrorDialogAction::Export) {
            " [ Exportar ] "
        } else {
            "  Exportar   "
        };
        let export_area = Rect {
            x: start_x,
            y: button_y,
            width: button_width,
            height: 1,
        };
        Paragraph::new(export_text)
            .style(export_style)
            .alignment(Alignment::Center)
            .render(export_area, buf);

        // Back button
        let back_style = if matches!(self.action, ErrorDialogAction::Back) {
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let back_text = if matches!(self.action, ErrorDialogAction::Back) {
            " [ Voltar ] "
        } else {
            "   Voltar    "
        };
        let back_area = Rect {
            x: start_x + button_width + 4,
            y: button_y,
            width: button_width,
            height: 1,
        };
        Paragraph::new(back_text)
            .style(back_style)
            .alignment(Alignment::Center)
            .render(back_area, buf);
    }
}
