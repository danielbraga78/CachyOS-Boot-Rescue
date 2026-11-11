//! Screen implementations for the TUI.

pub mod chroot;
pub mod diagnostic;
pub mod discovery;
pub mod error;
pub mod install_select;
pub mod logs;
pub mod mount_progress;
pub mod mount_review;
pub mod welcome;

pub use chroot::ChrootScreen;
pub use diagnostic::render_diagnostic;
pub use discovery::DiscoveryScreen;
pub use error::ErrorScreen;
pub use install_select::InstallSelectScreen;
pub use logs::LogsScreen;
pub use mount_progress::MountProgressScreen;
pub use mount_review::MountReviewScreen;
pub use welcome::WelcomeScreen;

use crate::app::AppCommand;
use crate::app::AppModel;
use crate::app::AppState;
use ratatui::{Frame, layout::Rect};

/// Trait for all screens.
pub trait Screen {
    fn render(&mut self, area: Rect, f: &mut Frame, app: &AppModel);
    fn handle_input(
        &mut self,
        key: crossterm::event::KeyEvent,
        app: &AppModel,
    ) -> Option<AppCommand>;
}

/// Screen de diagnóstico simples (stateless).
pub struct DiagnosticScreen;

impl DiagnosticScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for DiagnosticScreen {
    fn render(&mut self, area: Rect, f: &mut Frame, app: &AppModel) {
        diagnostic::render_diagnostic(f, area, &app.shared);
    }

    fn handle_input(
        &mut self,
        key: crossterm::event::KeyEvent,
        app: &AppModel,
    ) -> Option<AppCommand> {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(AppCommand::LeaveChroot),
            KeyCode::Char('r') => {
                // Re-executar diagnóstico
                if let Some(idx) = app.selected_install {
                    Some(AppCommand::RunDiagnostic {
                        candidate_idx: idx,
                        config: crate::diagnostic::runner::full_diagnostic_config(None),
                    })
                } else {
                    None
                }
            }
            KeyCode::Char('e') => {
                // Exportar relatório
                Some(AppCommand::ExportReport {
                    output_path: std::path::PathBuf::from(format!(
                        "diagnostic-report-{}.tar.gz",
                        chrono::Utc::now().format("%Y%m%d-%H%M%S")
                    )),
                })
            }
            _ => None,
        }
    }
}

/// Cria a tela correta para o estado atual da aplicação.
pub fn screen_for_state(state: &AppState) -> Box<dyn Screen> {
    match state {
        AppState::Welcome => Box::new(welcome::WelcomeScreen::new()),
        AppState::Discovery => Box::new(discovery::DiscoveryScreen::new()),
        AppState::InstallSelect => Box::new(install_select::InstallSelectScreen::new()),
        AppState::MountReview => Box::new(mount_review::MountReviewScreen::new()),
        AppState::MountProgress { .. } => Box::new(mount_progress::MountProgressScreen::new()),
        AppState::ChrootActive => Box::new(chroot::ChrootScreen::new()),
        AppState::Diagnostic => Box::new(DiagnosticScreen::new()),
        AppState::LogsReport => Box::new(logs::LogsScreen::new()),
        AppState::Error { .. } => Box::new(error::ErrorScreen::new()),
        AppState::Quit => Box::new(welcome::WelcomeScreen::new()),
    }
}
