use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Row, StatefulWidget, Table, TableState},
};

/// State for the TableView component.
#[derive(Debug, Default, Clone)]
pub struct TableViewState {
    pub selected: Option<usize>,
    pub offset: usize,
    pub rows: Vec<Vec<String>>,
    pub headers: Vec<String>,
}

impl TableViewState {
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        Self {
            headers,
            rows,
            selected: None,
            offset: 0,
        }
    }

    pub fn next(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let next = match self.selected {
            Some(i) => (i + 1).min(self.rows.len() - 1),
            None => 0,
        };
        self.selected = Some(next);
    }

    pub fn previous(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let prev = match self.selected {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.selected = Some(prev);
    }

    pub fn select(&mut self, index: usize) {
        if index < self.rows.len() {
            self.selected = Some(index);
        }
    }
}

/// A reusable table view widget.
#[derive(Debug, Default)]
pub struct TableView {
    pub title: Option<String>,
    pub show_borders: bool,
    pub column_widths: Vec<Constraint>,
}

impl TableView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn borders(mut self, show: bool) -> Self {
        self.show_borders = show;
        self
    }

    pub fn column_widths(mut self, widths: Vec<Constraint>) -> Self {
        self.column_widths = widths;
        self
    }
}

impl StatefulWidget for TableView {
    type State = TableViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let header_cells: Vec<Span> = state
            .headers
            .iter()
            .map(|h| {
                Span::styled(
                    h.clone(),
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Yellow),
                )
            })
            .collect();
        let header = Row::new(header_cells).style(Style::default().bg(Color::DarkGray));

        let rows: Vec<Row> = state
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let cells: Vec<Span> = row.iter().map(|c| Span::raw(c.clone())).collect();
                let style = if state.selected == Some(i) {
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                Row::new(cells).style(style)
            })
            .collect();

        let mut table = Table::new(rows, self.column_widths.clone())
            .header(header)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));

        if let Some(title) = self.title {
            table = table.block(Block::default().borders(Borders::ALL).title(title));
        } else if self.show_borders {
            table = table.block(Block::default().borders(Borders::ALL));
        }

        ratatui::widgets::StatefulWidget::render(
            table,
            area,
            buf,
            &mut TableState::default().with_selected(state.selected),
        );
    }
}
