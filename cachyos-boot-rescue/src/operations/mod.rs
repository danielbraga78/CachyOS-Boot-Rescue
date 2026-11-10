//! Operations modules for async execution.

pub mod discovery;
pub mod mount;
pub mod chroot;

pub use discovery::run_discovery_op;
pub use mount::execute_mount;
pub use chroot::run_chroot_task;