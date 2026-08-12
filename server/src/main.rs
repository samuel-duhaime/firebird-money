mod features;
mod shared;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};

use crate::features::transactions::JobStore;
use crate::shared::config::AuthConfig;
use crate::shared::l10n::L10n;
use crate::shared::SERVER_ADDR;

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
        println!(
            "SKIP_EMAIL_VERIFICATION is on: sign-in skips the magic-link email and logs you straight in."
        );
    }

    let addr = SERVER_ADDR;
    // Routes are documented in the README rather than echoed here, so the two can't drift.
    println!("Server listening on http://{addr}");

    let l10n = web::Data::new(L10n::new());
    let pool = web::Data::new(shared::postgres::create_pool().await);
    let import_jobs = web::Data::new(JobStore::default());
    let auth_config = web::Data::new(auth_config);
    // One client, shared by every magic-link send, so the TLS connection pool is reused.
    let http_client = web::Data::new(reqwest::Client::new());

    // Actix web server configuration
    HttpServer::new(move || {
        // Dev-only: allows the Vite client (a different origin) to call this API.
        // `Content-Disposition` must be explicitly exposed, or the browser's `fetch()` can't read
        // the filename off download responses (it's not on the CORS response-header safelist).
        // `supports_credentials` is what lets the browser send the session cookie on these
        // cross-origin calls; it only works alongside an explicit origin, never a wildcard.
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173")
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
    .bind(addr)?
    .run()
    .await
}
