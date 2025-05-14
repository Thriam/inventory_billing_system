use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use dotenv::dotenv;
use once_cell::sync::OnceCell;
use std::env;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client;

mod auth;
mod backup;
mod billing;
mod db;
mod import;
mod inventory;
mod ledger;
mod mail;

use crate::db::DB;

async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("Inventory Billing System is running")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let db_url = env::var("SURREALDB_URL").expect("SURREALDB_URL must be set");
    let db_ns = env::var("SURREALDB_NS").unwrap_or_else(|_| "test".to_string());
    let db_db = env::var("SURREALDB_DB").unwrap_or_else(|_| "test".to_string());
    let db_user = env::var("SURREALDB_USER").unwrap_or_else(|_| "root".to_string());
    let db_pass = env::var("SURREALDB_PASS").unwrap_or_else(|_| "root".to_string());

    let client = Surreal::new::<Client>(db_url.as_str())
        .await
        .expect("Failed to connect to SurrealDB");
    client
        .signin(surrealdb::opt::auth::Root {
            username: &db_user,
            password: &db_pass,
        })
        .await
        .expect("Failed to signin to SurrealDB");
    client
        .use_ns(db_ns)
        .use_db(db_db)
        .await
        .expect("Failed to select namespace and database");

    DB.set(client).expect("Failed to set global DB client");

    HttpServer::new(|| {
        App::new()
            .route("/health", web::get().to(health_check))
            // Auth routes
            .service(
                web::scope("/auth")
                    .route("/register", web::post().to(auth::register_user))
                    .route("/login", web::post().to(auth::login_user))
                    .route("/logout", web::post().to(auth::logout_user))
                    .route("/change_password", web::post().to(auth::change_password))
                    .route(
                        "/request_password_reset",
                        web::post().to(auth::request_password_reset),
                    )
                    .route("/reset_password", web::post().to(auth::reset_password)),
            )
            // Inventory routes
            .service(
                web::scope("/inventory")
                    .route("/create", web::post().to(inventory::create_item))
                    .route("/list", web::get().to(inventory::list_items)),
            )
            // Billing routes
            .service(
                web::scope("/billing")
                    .route("/create", web::post().to(billing::create_bill))
                    .route("/list", web::get().to(billing::list_bills)),
            )
            // Ledger routes
            .service(
                web::scope("/ledger")
                    .route("/create", web::post().to(ledger::create_ledger_entry))
                    .route("/list", web::get().to(ledger::list_ledger_entries)),
            )
            // Backup routes
            .service(web::scope("/backup").route("/run", web::post().to(backup::backup_data)))
            // Import routes
            .service(web::scope("/import").route("/run", web::post().to(import::import_data)))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
