//! Version checks for the TUI's new-release badge.

mod check;

pub use check::{UpdateCheckResult, UpdateInfo, check_for_updates};
