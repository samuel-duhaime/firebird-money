pub mod anthropic;
pub mod config;
pub mod email;
pub mod http_error;
pub mod l10n;
pub mod postgres;
#[cfg(test)]
pub mod test_support;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 3055;

/// Pure resolution logic, testable without touching the process environment. `host_env` and
/// `port_env` are `HOST` and `PORT` as read from the environment, or `None` if unset.
fn resolve_server_addr(host_env: Option<&str>, port_env: Option<&str>) -> String {
    let host = host_env
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .unwrap_or(DEFAULT_HOST);
    let port = port_env
        .and_then(|value| value.trim().parse::<u16>().ok())
        // 0 asks the OS for an ephemeral port, which nothing logs or reports back — fall back
        // instead of binding to it.
        .filter(|port| *port != 0)
        .unwrap_or(DEFAULT_PORT);
    format!("{host}:{port}")
}

/// Address this server binds to (`main.rs`). Reads `PORT` from the environment so multiple instances can run side by side (e.g. one per
/// Playwright worker in `client/e2e`), and `HOST` so a container can accept connections from
/// outside itself; unset falls back to the historical default, so ordinary `cargo run` behavior is
/// unchanged.
pub fn server_addr() -> String {
    resolve_server_addr(
        std::env::var("HOST").ok().as_deref(),
        std::env::var("PORT").ok().as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_port_is_unset() {
        assert_eq!(resolve_server_addr(None, None), "127.0.0.1:3055");
    }

    #[test]
    fn uses_a_valid_port() {
        assert_eq!(resolve_server_addr(None, Some("4103")), "127.0.0.1:4103");
    }

    #[test]
    fn falls_back_on_an_invalid_port() {
        assert_eq!(
            resolve_server_addr(None, Some("not-a-port")),
            "127.0.0.1:3055"
        );
    }

    #[test]
    fn uses_a_given_host() {
        assert_eq!(resolve_server_addr(Some("0.0.0.0"), None), "0.0.0.0:3055");
    }

    #[test]
    fn falls_back_on_an_empty_host() {
        assert_eq!(resolve_server_addr(Some(" "), None), "127.0.0.1:3055");
    }

    #[test]
    fn falls_back_on_port_zero() {
        assert_eq!(resolve_server_addr(None, Some("0")), "127.0.0.1:3055");
    }
}
