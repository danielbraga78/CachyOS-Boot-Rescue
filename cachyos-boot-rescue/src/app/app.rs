use std::{
    io::{self, Stdout},
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    app::{AppCommand, AppModel, AppState, MountStage, OpId, OpKind, OperationState, reduce},
    runtime::{
        COMMAND_CHANNEL_BUFFER, CommandSender, EVENT_CHANNEL_BUFFER, EventReceiver, TaskManager,
        command_channel, event_channel,
    },
    ui::{draw_main_layout, screens::screen_for_state},
};

/// Aplicação principal da TUI.
pub struct App {
    model: AppModel,
    command_tx: CommandSender,
    event_rx: EventReceiver,
    current_screen: Box<dyn crate::ui::screens::Screen>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    task_manager: Option<TaskManager>,
    should_quit: bool,
    last_tick: Instant,
}

impl App {
    /// Cria a instância principal da aplicação.
    pub fn new() -> Self {
        let model = AppModel::default();
        let current_screen = screen_for_state(&model.state);
        let (command_tx, command_rx) = command_channel(COMMAND_CHANNEL_BUFFER);
        let (event_tx, event_rx) = event_channel(EVENT_CHANNEL_BUFFER);
        let task_manager = Some(TaskManager::new(command_rx, event_tx));

        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend).expect("terminal must be initialized");

        Self {
            model,
            command_tx,
            event_rx,
            current_screen,
            terminal,
            task_manager,
            should_quit: false,
            last_tick: Instant::now(),
        }
    }

    /// Inicia a aplicação em modo TUI.
    pub async fn run() -> crate::error::Result<()> {
        let mut app = Self::new();
        app.setup_terminal()?;
        let task_manager = app.task_manager.take().expect("task manager initialized");
        tokio::spawn(task_manager.run());

        while !app.should_quit {
            app.draw()?;
            app.handle_input_events().await?;
            app.handle_runtime_events().await?;

            if app.last_tick.elapsed() >= Duration::from_millis(250) {
                app.command_tx
                    .send(AppCommand::Tick)
                    .await
                    .map_err(|error| crate::error::Error::internal(error.to_string()))?;
                app.last_tick = Instant::now();
            }
        }

        app.shutdown().await?;
        Ok(())
    }

    fn setup_terminal(&mut self) -> crate::error::Result<()> {
        enable_raw_mode().map_err(|error| crate::error::Error::internal(error.to_string()))?;
        execute!(io::stdout(), EnterAlternateScreen,)
            .map_err(|error| crate::error::Error::internal(error.to_string()))?;
        Ok(())
    }

    async fn shutdown(&mut self) -> crate::error::Result<()> {
        let _ = self.command_tx.send(AppCommand::Quit).await;
        disable_raw_mode().map_err(|error| crate::error::Error::internal(error.to_string()))?;
        execute!(io::stdout(), LeaveAlternateScreen,)
            .map_err(|error| crate::error::Error::internal(error.to_string()))?;
        self.terminal
            .show_cursor()
            .map_err(|error| crate::error::Error::internal(error.to_string()))?;
        Ok(())
    }

    fn draw(&mut self) -> crate::error::Result<()> {
        self.terminal
            .draw(|frame| draw_main_layout(frame, &self.model, self.current_screen.as_mut()))
            .map_err(|error| crate::error::Error::internal(error.to_string()))?;
        Ok(())
    }

    async fn handle_input_events(&mut self) -> crate::error::Result<()> {
        if !event::poll(Duration::from_millis(50))
            .map_err(|error| crate::error::Error::internal(error.to_string()))?
        {
            return Ok(());
        }

        match event::read().map_err(|error| crate::error::Error::internal(error.to_string()))? {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if let Some(command) = self.current_screen.handle_input(key, &self.model) {
                    self.apply_command(command.clone()).await?;
                    if self.should_quit {
                        return Ok(());
                    }
                    self.command_tx
                        .send(command)
                        .await
                        .map_err(|error| crate::error::Error::internal(error.to_string()))?;
                }
            }
            Event::Resize(_, _) => {
                self.command_tx
                    .send(AppCommand::TerminalResize)
                    .await
                    .map_err(|error| crate::error::Error::internal(error.to_string()))?;
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_runtime_events(&mut self) -> crate::error::Result<()> {
        while let Ok(event) = self.event_rx.try_recv() {
            let next_model = std::mem::take(&mut self.model);
            let (new_model, commands) = reduce(next_model, event);
            self.model = new_model;
            self.current_screen = screen_for_state(&self.model.state);

            for command in commands {
                self.apply_command(command.clone()).await?;
                self.command_tx
                    .send(command)
                    .await
                    .map_err(|error| crate::error::Error::internal(error.to_string()))?;
            }
        }

        Ok(())
    }

    async fn apply_command(&mut self, command: AppCommand) -> crate::error::Result<()> {
        match &command {
            AppCommand::RunDiscovery => {
                self.model.state = AppState::Discovery;
                self.model.operation = OperationState::Running {
                    op_id: OpId::new(),
                    kind: OpKind::Discovery,
                    cancel_requested: false,
                };
                self.model.shared.last_error = None;
                self.current_screen = screen_for_state(&self.model.state);
            }
            AppCommand::BuildMountPlan { candidate_idx } => {
                self.model.selected_install = Some(*candidate_idx);
                self.model.operation = OperationState::Running {
                    op_id: OpId::new(),
                    kind: OpKind::Discovery,
                    cancel_requested: false,
                };
            }
            AppCommand::ExecuteMount { .. } => {
                self.model.state = AppState::MountProgress {
                    stage: MountStage::Preparing,
                };
                self.model.operation = OperationState::Running {
                    op_id: OpId::new(),
                    kind: OpKind::Mount,
                    cancel_requested: false,
                };
                self.current_screen = screen_for_state(&self.model.state);
            }
            AppCommand::ExecuteChrootCommand { .. } => {
                self.model.operation = OperationState::Running {
                    op_id: OpId::new(),
                    kind: OpKind::ChrootCommand,
                    cancel_requested: false,
                };
            }
            AppCommand::CancelOperation { op_id } => {
                if matches!(
                    &self.model.operation,
                    OperationState::Running { op_id: current, .. } if *current == *op_id
                ) {
                    self.model.operation = OperationState::Running {
                        op_id: *op_id,
                        kind: OpKind::Mount,
                        cancel_requested: true,
                    };
                }
            }
            AppCommand::LeaveChroot => {
                self.model.state = AppState::MountReview;
                self.model.operation = OperationState::Idle;
                self.current_screen = screen_for_state(&self.model.state);
            }
            AppCommand::Quit => {
                self.should_quit = true;
                self.model.state = AppState::Quit;
                self.model.operation = OperationState::Idle;
                self.current_screen = screen_for_state(&self.model.state);
            }
            _ => {}
        }

        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
