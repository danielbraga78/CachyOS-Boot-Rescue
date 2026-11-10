use ratatui::{
    buffer::Buffer,
    layout::Rect,
    prelude::Widget,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget},
};

/// Log entry for the log view.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub component: String,
    pub message: String,
}

/// State for the log view.
#[derive(Debug, Default, Clone)]
pub struct LogViewState {
    pub entries: Vec<LogEntry>,
    pub filter: String,
    pub selected: Option<usize>,
    pub offset: usize,
    pub show_export_hint: bool,
    pub title: String,
    pub show_borders: bool,
}

impl LogViewState {
    pub fn new(entries: Vec<LogEntry>) -> Self {
        Self {
            entries,
            filter: String::new(),
            selected: None,
            offset: 0,
            show_export_hint: true,
            title: String::new(),
            show_borders: true,
        }
    }

    pub fn filtered_entries(&self) -> Vec<&LogEntry> {
        if self.filter.is_empty() {
            self.entries.iter().collect()
        } else {
            self.entries
                .iter()
                .filter(|e| {
                    e.message
                        .to_lowercase()
                        .contains(&self.filter.to_lowercase())
                        || e.component
                            .to_lowercase()
                            .contains(&self.filter.to_lowercase())
                        || e.level.to_lowercase().contains(&self.filter.to_lowercase())
                })
                .collect()
        }
    }

    pub fn next(&mut self) {
        let filtered = self.filtered_entries();
        if filtered.is_empty() {
            return;
        }
        let next = match self.selected {
            Some(i) => (i + 1).min(filtered.len() - 1),
            None => 0,
        };
        self.selected = Some(next);
    }

    pub fn previous(&mut self) {
        let filtered = self.filtered_entries();
        if filtered.is_empty() {
            return;
        }
        let prev = match self.selected {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.selected = Some(prev);
    }
}

/// A log view widget.
#[derive(Debug, Default)]
pub struct LogView;

impl LogView {
    pub fn new() -> Self {
        Self
    }
}

impl StatefulWidget for LogView {
    type State = LogViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let filtered = state.filtered_entries();

        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let level_style = match entry.level.as_str() {
                    "ERROR" => Style::default()
                        .fg(Color::Red)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                    "WARN" => Style::default().fg(Color::Yellow),
                    "INFO" => Style::default().fg(Color::Green),
                    "DEBUG" => Style::default().fg(Color::Blue),
                    _ => Style::default(),
                };

                let line = Line::from(vec![
                    Span::styled(&entry.timestamp, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(&entry.level, level_style),
                    Span::raw(" "),
                    Span::styled(&entry.component, Style::default().fg(Color::Cyan)),
                    Span::raw(": "),
                    Span::raw(&entry.message),
                ]);

                let style = if state.selected == Some(i) {
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(line).style(style)
            })
            .collect();

        let mut list = List::new(items);
        if state.show_borders {
            list = list.block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(state.title.clone()),
            );
        }

        ratatui::widgets::StatefulWidget::render(
            list,
            area,
            buf,
            &mut ListState::default().with_selected(state.selected),
        );

        // Export hint
        if state.show_export_hint {
            let hint_area = Rect {
                x: area.x,
                y: area.y + area.height.saturating_sub(1),
                width: area.width,
                height: 1,
            };
            let hint =
                Paragraph::new("Pressione 'e' para exportar, '/' para filtrar, 'Esc' para voltar")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(ratatui::layout::Alignment::Center);
            hint.render(hint_area, buf);
        }
    }
}
