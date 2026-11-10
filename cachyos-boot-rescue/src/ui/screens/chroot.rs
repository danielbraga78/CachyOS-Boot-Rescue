use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};

use crate::app::{AppCommand, AppModel};
use crate::ui::components::HelpFooter;
use crate::ui::help_footer::default_bindings;
use crate::ui::screens::Screen;

/// Chroot active screen - interactive shell.
#[derive(Debug, Default)]
pub struct ChrootScreen {
    input_buffer: String,
    history: Vec<String>,
    history_index: Option<usize>,
    output_lines: Vec<String>,
    list_state: ListState,
}

impl ChrootScreen {
    pub fn new() -> Self {
        Self {
            input_buffer: String::new(),
            history: Vec::new(),
            history_index: None,
            output_lines: Vec::new(),
            list_state: ListState::default(),
        }
    }
}

impl Screen for ChrootScreen {
    fn render(&mut self, area: Rect, f: &mut Frame, _app: &AppModel) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Chroot Ativo - Shell Interativo")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Chroot "));
        title.render(chunks[0], f.buffer_mut());

        // Output
        let items: Vec<ListItem> = self
            .output_lines
            .iter()
            .map(|line| ListItem::new(Line::from(Span::raw(line.clone()))))
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Saída "))
            .style(Style::default().bg(Color::Black).fg(Color::White));
        StatefulWidget::render(list, chunks[1], f.buffer_mut(), &mut self.list_state);

        // Input line
        let input = Paragraph::new(format!("$ {}", self.input_buffer))
            .style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL).title(" Comando "));
        input.render(chunks[2], f.buffer_mut());

        // Help footer
        let footer = HelpFooter::new()
            .bindings(default_bindings::chroot_active())
            .borders(true);
        footer.render(chunks[3], f.buffer_mut());
    }

    fn handle_input(&mut self, key: KeyEvent, _app: &AppModel) -> Option<AppCommand> {
        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'd' {
                    return Some(AppCommand::Quit);
                }
                self.input_buffer.push(c);
                None
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                None
            }
            KeyCode::Enter => {
                let command = self.input_buffer.clone();
                if !command.is_empty() {
                    self.history.push(command.clone());
                    self.history_index = None;
                    self.output_lines.push(format!("$ {}", command));
                    self.input_buffer.clear();
                    return Some(AppCommand::ExecuteChrootCommand {
                        command: vec![command],
                    });
                }
                None
            }
            KeyCode::Up => {
                if let Some(idx) = self.history_index {
                    if idx > 0 {
                        self.history_index = Some(idx - 1);
                        self.input_buffer = self.history[self.history_index.unwrap()].clone();
                    }
                } else if !self.history.is_empty() {
                    self.history_index = Some(self.history.len() - 1);
                    self.input_buffer = self.history[self.history_index.unwrap()].clone();
                }
                None
            }
            KeyCode::Down => {
                if let Some(idx) = self.history_index {
                    if idx + 1 < self.history.len() {
                        self.history_index = Some(idx + 1);
                        self.input_buffer = self.history[self.history_index.unwrap()].clone();
                    } else {
                        self.history_index = None;
                        self.input_buffer.clear();
                    }
                }
                None
            }
            KeyCode::Esc => Some(AppCommand::Quit),
            _ => None,
        }
    }
}
