//! Operations modules for async execution.

pub mod discovery;
pub mod mount;

pub use discovery::run_discovery_op;
pub use mount::execute_mount;