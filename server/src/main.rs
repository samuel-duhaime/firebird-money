mod features;
mod shared;

use actix_web::{web, App, HttpServer};

use crate::shared::l10n::L10n;

/// Main function to start the server
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let addr = "127.0.0.1:3055";
    println!("Server listening on http://{addr}/transactions            (GET list, optional ?date=&merchant=; POST create)");
    println!("  and http://{addr}/transactions/{{id}}        (GET, PATCH, DELETE)");
    println!("  and http://{addr}/categories               (GET list; POST create)");
    println!("  and http://{addr}/categories/{{id}}         (GET, PATCH, DELETE)");

    let l10n = web::Data::new(L10n::new());
    let pool = web::Data::new(shared::postgres::create_pool().await);

    // Actix web server configuration
    HttpServer::new(move || {
        App::new()
            .app_data(l10n.clone())
            .app_data(pool.clone())
            .configure(features::transactions::configure)
            .configure(features::categories::configure)
    })
        .bind(addr)?
        .run()
        .await
}
