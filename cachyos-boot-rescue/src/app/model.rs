//! Modelo central da aplicação.

use crate::app::state::{AppState, OperationState, SharedData};
use uuid::Uuid;

/// Identificador único de operação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpId(pub Uuid);

impl OpId {
    /// Cria um novo ID único.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for OpId {
    fn default() -> Self {
        Self::new()
    }
}

/// Tipo de operação para controle de concorrência.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpKind {
    /// Descoberta (pode rodar múltiplas).
    Discovery,
    /// Montagem (exclusivo com outras mutáveis).
    Mount,
    /// Comando chroot (exclusivo com outras mutáveis).
    ChrootCommand,
}

impl OpKind {
    /// Verifica se este tipo de operação conflita com outro.
    pub fn conflicts_with(&self, other: &OpKind) -> bool {
        match (self, other) {
            (OpKind::Discovery, OpKind::Discovery) => false,
            (OpKind::Mount, OpKind::Mount) => true,
            (OpKind::Mount, OpKind::ChrootCommand) => true,
            (OpKind::ChrootCommand, OpKind::Mount) => true,
            (OpKind::ChrootCommand, OpKind::ChrootCommand) => true,
            (OpKind::Discovery, _) | (_, OpKind::Discovery) => false,
        }
    }
}

/// Estágio de montagem para progresso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MountStage {
    /// Preparando montagem.
    Preparing,
    /// Montando raiz.
    MountingRoot,
    /// Montando /boot.
    MountingBoot,
    /// Montando EFI.
    MountingEfi,
    /// Montando outros pontos.
    MountingOther,
    /// Finalizando.
    Finalizing,
    /// Concluído.
    Completed,
}

/// Modelo principal da aplicação.
#[derive(Debug)]
pub struct AppModel {
    /// Estado de navegação atual.
    pub state: AppState,
    /// Dados compartilhados entre telas.
    pub shared: SharedData,
    /// Índice da instalação selecionada na lista de candidatos.
    pub selected_install: Option<usize>,
    /// Estado da operação em andamento.
    pub operation: OperationState,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            state: AppState::Welcome,
            shared: SharedData::default(),
            selected_install: None,
            operation: OperationState::Idle,
        }
    }
}