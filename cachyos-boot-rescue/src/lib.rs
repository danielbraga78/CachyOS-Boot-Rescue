//! cachyos-boot-rescue library crate.

#![allow(clippy::large_enum_variant)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::cmp_owned)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::unnecessary_sort_by)]
#![allow(clippy::new_without_default)]
#![allow(clippy::manual_map)]

pub mod app;
pub mod chroot;
pub mod cli;
pub mod config;
pub mod diagnostic;
pub mod discovery;
pub mod error;
pub mod logging;
pub mod operations;
pub mod runtime;
pub mod storage;
pub mod ui;

pub use crate::error::{Error, Result};
