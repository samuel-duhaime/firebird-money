pub mod config;
pub mod email;
pub mod http_error;
pub mod l10n;
pub mod postgres;

const DEFAULT_PORT: u16 = 3055;

/// Pure resolution logic, testable without touching the process environment. `port_env` is `PORT`
/// as read from the environment, or `None` if unset.
fn resolve_server_addr(port_env: Option<&str>) -> String {
    let port = port_env
        .and_then(|value| value.trim().parse::<u16>().ok())
        // 0 asks the OS for an ephemeral port, which `server_addr()`'s caller here can't discover
        // after the fact (see the import callback below) — fall back instead of binding to it.
        .filter(|port| *port != 0)
        .unwrap_or(DEFAULT_PORT);
    format!("127.0.0.1:{port}")
}

/// Address this server binds to (`main.rs`) and that the unattended import subprocess calls back
/// into (`features::transactions::import`) — a single source of truth so the two can't drift.
/// Reads `PORT` from the environment so multiple instances can run side by side (e.g. one per
/// Playwright worker in `client/e2e`); unset falls back to the historical default, so ordinary
/// `cargo run` behavior is unchanged.
pub fn server_addr() -> String {
    resolve_server_addr(std::env::var("PORT").ok().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_port_is_unset() {
        assert_eq!(resolve_server_addr(None), "127.0.0.1:3055");
    }

    #[test]
    fn uses_a_valid_port() {
        assert_eq!(resolve_server_addr(Some("4103")), "127.0.0.1:4103");
    }

    #[test]
    fn falls_back_on_an_invalid_port() {
        assert_eq!(resolve_server_addr(Some("not-a-port")), "127.0.0.1:3055");
    }

    #[test]
    fn falls_back_on_port_zero() {
        assert_eq!(resolve_server_addr(Some("0")), "127.0.0.1:3055");
    }
}
