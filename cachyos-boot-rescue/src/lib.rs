//! cachyos-boot-rescue library crate.

pub mod app;
pub mod chroot;
pub mod cli;
pub mod config;
pub mod discovery;
pub mod error;
pub mod logging;
pub mod storage;

pub use crate::error::{Error, Result};
