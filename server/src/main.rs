mod features;
mod shared;

use std::time::Duration;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use log::{info, warn};

use crate::features::transactions::JobStore;
use crate::shared::config::AuthConfig;
use crate::shared::l10n::L10n;

/// Main function to start the server
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Refuse to start on an unsafe auth setup (e.g. the "skip the email" dev shortcut left on in
    // production) rather than discovering it on someone's sign-in.
    let auth_config = match AuthConfig::from_env() {
        Ok(config) => config,
        Err(message) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid configuration: {message} (see .env.example)"),
            ))
        }
    };

    if auth_config.skip_email_verification {
        // Security-relevant state: warn, not println!, so it carries a level and timestamp and
        // can be picked up by log collection rather than only appearing on an attached terminal.
        warn!(
            "SKIP_EMAIL_VERIFICATION is on: sign-in skips the magic-link email and logs you straight in."
        );
    }

    let addr = shared::server_addr();
    // Routes are documented in the README rather than echoed here, so the two can't drift.
    info!("Server listening on http://{addr}");

    // The origin the client actually runs at, so a real deployment's CORS allowance matches
    // where `CLIENT_BASE_URL` points the magic link — not just localhost.
    let allowed_origin = auth_config
        .client_base_url
        .trim_end_matches('/')
        .to_string();

    let l10n = web::Data::new(L10n::new());
    let pool = web::Data::new(shared::postgres::create_pool().await);
    let import_jobs = web::Data::new(JobStore::default());
    let auth_config = web::Data::new(auth_config);
    // One client, shared by every magic-link send, so the TLS connection pool is reused. Short
    // timeouts keep a stalled Resend call from tying up the request for the 30s default.
    let http_client = web::Data::new(
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("failed to build the shared HTTP client"),
    );

    // Actix web server configuration
    HttpServer::new(move || {
        // Allows the client (a different origin) to call this API.
        // `Content-Disposition` must be explicitly exposed, or the browser's `fetch()` can't read
        // the filename off download responses (it's not on the CORS response-header safelist).
        // `supports_credentials` is what lets the browser send the session cookie on these
        // cross-origin calls; it only works alongside an explicit origin, never a wildcard.
        let cors = Cors::default()
            .allowed_origin(&allowed_origin)
            .allow_any_method()
            .allow_any_header()
            .supports_credentials()
            .expose_headers(["Content-Disposition"]);

        App::new()
            .wrap(cors)
            .app_data(l10n.clone())
            .app_data(pool.clone())
            .app_data(import_jobs.clone())
            .app_data(auth_config.clone())
            .app_data(http_client.clone())
            .configure(features::auth::configure)
            .configure(features::transactions::configure)
            .configure(features::categories::configure)
            .configure(features::households::configure)
            .configure(features::users::configure)
            .configure(features::household_members::configure)
    })
    .bind(addr.as_str())?
    .run()
    .await
}
