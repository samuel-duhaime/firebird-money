pub mod config;
pub mod email;
pub mod http_error;
pub mod l10n;
pub mod postgres;

/// Address this server binds to (`main.rs`) and that the unattended import subprocess calls back
/// into (`features::transactions::import`) — a single source of truth so the two can't drift.
pub const SERVER_ADDR: &str = "127.0.0.1:3055";
