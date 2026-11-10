use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

/// Confirmation dialog result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmResult {
    Yes,
    No,
}

/// State for the confirm dialog.
#[derive(Debug, Default, Clone)]
pub struct ConfirmDialogState {
    pub message: String,
    pub focus_yes: bool,
    pub result: Option<ConfirmResult>,
}

impl ConfirmDialogState {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            focus_yes: false, // Focus on "No" by default
            result: None,
        }
    }

    pub fn next(&mut self) {
        self.focus_yes = !self.focus_yes;
    }

    pub fn previous(&mut self) {
        self.focus_yes = !self.focus_yes;
    }

    pub fn confirm(&mut self) {
        self.result = if self.focus_yes {
            Some(ConfirmResult::Yes)
        } else {
            Some(ConfirmResult::No)
        };
    }
}

/// A confirmation dialog widget.
#[derive(Debug, Default)]
pub struct ConfirmDialog {
    pub title: String,
}

impl ConfirmDialog {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
        }
    }
}

impl Widget for &ConfirmDialogState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the area
        Clear.render(area, buf);

        // Calculate dialog size
        let dialog_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(area)[1];

        let dialog_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(dialog_area)[1];

        // Render dialog background
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Confirmação ")
            .style(Style::default().bg(Color::DarkGray));
        block.render(dialog_area, buf);

        // Message
        let message_area = Rect {
            x: dialog_area.x + 2,
            y: dialog_area.y + 1,
            width: dialog_area.width.saturating_sub(4),
            height: dialog_area.height.saturating_sub(4),
        };

        let message = Paragraph::new(self.message.clone())
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });
        message.render(message_area, buf);

        // Buttons
        let button_y = dialog_area.y + dialog_area.height - 2;
        let button_width = 10;
        let total_width = button_width * 2 + 4;
        let start_x = dialog_area.x + (dialog_area.width.saturating_sub(total_width)) / 2;

        // "No" button (default focus)
        let no_style = if !self.focus_yes {
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let no_text = if !self.focus_yes {
            " [ Não ] "
        } else {
            "   Não   "
        };
        let no_area = Rect {
            x: start_x,
            y: button_y,
            width: button_width,
            height: 1,
        };
        Paragraph::new(no_text)
            .style(no_style)
            .alignment(Alignment::Center)
            .render(no_area, buf);

        // "Yes" button
        let yes_style = if self.focus_yes {
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let yes_text = if self.focus_yes {
            " [ Sim ] "
        } else {
            "   Sim   "
        };
        let yes_area = Rect {
            x: start_x + button_width + 4,
            y: button_y,
            width: button_width,
            height: 1,
        };
        Paragraph::new(yes_text)
            .style(yes_style)
            .alignment(Alignment::Center)
            .render(yes_area, buf);
    }
}
