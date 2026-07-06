mod features;
mod l10n;

use actix_web::{web, App, HttpServer};

use crate::l10n::L10n;

/// Main function to start the server
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let addr = "127.0.0.1:3055";
    println!("Server listening on http://{addr}/transactions/sample");
    println!("  and http://{addr}/transactions/{{n}}  (e.g. /transactions/1)");

    let l10n = web::Data::new(L10n::new());

    // Actix web server configuration
    HttpServer::new(move || {
        App::new()
            .app_data(l10n.clone())
            .configure(features::transactions::configure)
    })
        .bind(addr)?
        .run()
        .await
}
