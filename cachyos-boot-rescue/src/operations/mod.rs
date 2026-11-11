//! Operations modules for async execution.

pub mod chroot;
pub mod discovery;
pub mod mount;

pub use crate::diagnostic::runner::run_diagnostic_op;
pub use chroot::run_chroot_task;
pub use discovery::run_discovery_op;
pub use mount::execute_mount;
