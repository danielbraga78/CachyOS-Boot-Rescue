use crate::app::MountStage;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Widget},
};

/// Progress bar widget for mount operations.
#[derive(Debug, Default)]
pub struct ProgressBar {
    pub show_stage: bool,
    pub show_percent: bool,
    pub show_message: bool,
}

impl ProgressBar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show_stage(mut self, show: bool) -> Self {
        self.show_stage = show;
        self
    }

    pub fn show_percent(mut self, show: bool) -> Self {
        self.show_percent = show;
        self
    }

    pub fn show_message(mut self, show: bool) -> Self {
        self.show_message = show;
        self
    }
}

/// State for the progress bar.
#[derive(Debug, Default, Clone)]
pub struct ProgressBarState {
    pub stage: MountStage,
    pub percent: u8,
    pub message: String,
}

impl Widget for &ProgressBarState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(2)])
            .split(area);

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progresso"))
            .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
            .percent(u16::from(self.percent.min(100)));
        gauge.render(chunks[0], buf);

        let mut lines = vec![Line::from(vec![
            Span::styled(
                "Stage: ",
                Style::default().add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::raw(self.stage.label()),
        ])];

        if !self.message.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "Msg: ",
                    Style::default().add_modifier(ratatui::style::Modifier::BOLD),
                ),
                Span::raw(&self.message),
            ]));
        }
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL))
            .render(chunks[1], buf);
    }
}
