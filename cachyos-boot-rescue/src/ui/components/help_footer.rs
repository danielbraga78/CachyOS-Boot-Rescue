use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

/// Key binding for help footer.
#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub key: String,
    pub description: String,
}

/// Help footer widget showing contextual key bindings.
#[derive(Debug, Default)]
pub struct HelpFooter {
    pub bindings: Vec<KeyBinding>,
    pub show_borders: bool,
}

impl HelpFooter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bindings(mut self, bindings: Vec<KeyBinding>) -> Self {
        self.bindings = bindings;
        self
    }

    pub fn add_binding(mut self, key: impl Into<String>, description: impl Into<String>) -> Self {
        self.bindings.push(KeyBinding {
            key: key.into(),
            description: description.into(),
        });
        self
    }

    pub fn borders(mut self, show: bool) -> Self {
        self.show_borders = show;
        self
    }
}

impl Widget for HelpFooter {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.bindings.is_empty() {
            return;
        }

        let line = Line::from(
            self.bindings
                .iter()
                .flat_map(|b| {
                    vec![
                        Span::styled(
                            &b.key,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(&b.description, Style::default().fg(Color::White)),
                        Span::raw("  "),
                    ]
                })
                .collect::<Vec<_>>(),
        );

        let mut paragraph = Paragraph::new(line)
            .style(Style::default().bg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Left);

        if self.show_borders {
            paragraph = paragraph.block(Block::default().borders(Borders::ALL));
        }

        paragraph.render(area, buf);
    }
}

/// Default key bindings for each screen.
pub mod default_bindings {
    use super::KeyBinding;

    pub fn welcome() -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                key: "Enter".into(),
                description: "Iniciar descoberta".into(),
            },
            KeyBinding {
                key: "Esc/q".into(),
                description: "Sair".into(),
            },
        ]
    }

    pub fn discovery() -> Vec<KeyBinding> {
        vec![KeyBinding {
            key: "Esc".into(),
            description: "Cancelar".into(),
        }]
    }

    pub fn install_select() -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                key: "↑/↓".into(),
                description: "Navegar".into(),
            },
            KeyBinding {
                key: "Enter".into(),
                description: "Selecionar".into(),
            },
            KeyBinding {
                key: "r".into(),
                description: "Refazer descoberta".into(),
            },
            KeyBinding {
                key: "Esc".into(),
                description: "Voltar".into(),
            },
        ]
    }

    pub fn mount_review() -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                key: "↑/↓".into(),
                description: "Navegar entradas".into(),
            },
            KeyBinding {
                key: "y".into(),
                description: "Confirmar montagem".into(),
            },
            KeyBinding {
                key: "r".into(),
                description: "Refazer descoberta".into(),
            },
            KeyBinding {
                key: "Esc".into(),
                description: "Voltar".into(),
            },
        ]
    }

    pub fn mount_progress() -> Vec<KeyBinding> {
        vec![KeyBinding {
            key: "Esc".into(),
            description: "Cancelar".into(),
        }]
    }

    pub fn chroot_active() -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                key: "Ctrl+D".into(),
                description: "Sair do chroot".into(),
            },
            KeyBinding {
                key: "Esc".into(),
                description: "Sair do chroot".into(),
            },
        ]
    }

    pub fn logs_report() -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                key: "↑/↓".into(),
                description: "Navegar logs".into(),
            },
            KeyBinding {
                key: "/".into(),
                description: "Filtrar".into(),
            },
            KeyBinding {
                key: "e".into(),
                description: "Exportar relatório".into(),
            },
            KeyBinding {
                key: "Esc".into(),
                description: "Voltar".into(),
            },
        ]
    }

    pub fn error() -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                key: "←/→".into(),
                description: "Navegar botões".into(),
            },
            KeyBinding {
                key: "Enter".into(),
                description: "Confirmar".into(),
            },
            KeyBinding {
                key: "Esc".into(),
                description: "Voltar".into(),
            },
        ]
    }
}
