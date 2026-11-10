use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget},
};

/// State for the ListView component.
#[derive(Debug, Default, Clone)]
pub struct ListViewState {
    pub selected: Option<usize>,
    pub offset: usize,
    pub items: Vec<String>,
}

impl ListViewState {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items,
            selected: None,
            offset: 0,
        }
    }

    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let next = match self.selected {
            Some(i) => (i + 1).min(self.items.len() - 1),
            None => 0,
        };
        self.selected = Some(next);
        self.adjust_offset();
    }

    pub fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let prev = match self.selected {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.selected = Some(prev);
        self.adjust_offset();
    }

    pub fn select(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected = Some(index);
            self.adjust_offset();
        }
    }

    fn adjust_offset(&mut self) {
        if let Some(selected) = self.selected {
            // Simple offset adjustment - keep selected item visible
            // In a real implementation, this would consider visible height
            if selected < self.offset {
                self.offset = selected;
            }
        }
    }
}

/// A reusable list view widget.
#[derive(Debug, Default)]
pub struct ListView {
    pub title: Option<String>,
    pub show_borders: bool,
}

impl ListView {
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
}

impl StatefulWidget for ListView {
    type State = ListViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let items: Vec<ListItem> = state
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if state.selected == Some(i) {
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(item.clone(), style)))
            })
            .collect();

        let mut list = List::new(items);
        if let Some(title) = self.title {
            list = list.block(Block::default().borders(Borders::ALL).title(title));
        } else if self.show_borders {
            list = list.block(Block::default().borders(Borders::ALL));
        }
        ratatui::widgets::StatefulWidget::render(
            list,
            area,
            buf,
            &mut ListState::default().with_selected(state.selected),
        );
    }
}
