use std::fs::File;
use std::sync::Arc;
use std::{path::Path, fs};

use actix_web::web::Data;
use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use service::block::{BlockService, block_updater};
use sqlx::{SqlitePool};

mod signature_verifier;
mod address;
mod model;
mod service;
mod routes;

#[get("/health")]
async fn health(_: String) -> impl Responder {
    HttpResponse::Ok().body("healthy")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if dotenv::dotenv().is_err() {
        println!(".env file not loaded. If you intended to use one, ensure it exists.");
    }
    // TODO: Respect and utilize the NETWORK env var where appropriate.
    env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let sanitized_url = database_url.replace("sqlite://", "");
    let db_path = Path::new(&sanitized_url);

    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).expect("Failed to create database directory.");
        }
    }

    if !db_path.exists() {
        File::create(&db_path).expect("Failed to create database file.");
    }

    let pool = SqlitePool::connect(&format!("{}", database_url)).await.unwrap();

    sqlx::migrate!().run(&pool).await.unwrap();

    let block_service = Arc::new(BlockService::new());
    tokio::spawn(block_updater(block_service.clone()));

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(block_service.clone()))
            .service(health)
            .service(routes::register::register)
            .service(routes::deregister::deregister)
            .service(routes::submit::submit)
            .service(routes::hashrate::hashrate)
    })
    .bind(("127.0.0.1", 7959))?
    .run()
    .await
}