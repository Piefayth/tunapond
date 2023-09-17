use std::collections::HashSet;

use std::sync::Arc;


use actix_web::web::Data;
use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use service::block::{BlockService, block_updater};
use service::submission::submission_updater;
use sqlx::postgres::PgPoolOptions;

mod address;
mod model;
mod service;
mod routes;
mod common;

#[get("/health")]
async fn health(_: String) -> impl Responder {
    HttpResponse::Ok().body("healthy")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if dotenv::dotenv().is_err() {
        println!(".env file not loaded. If you intended to use one, ensure it exists.");
    }

    env_logger::init();
    pool_is_configured();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let listen_address = std::env::var("LISTEN_ADDRESS").unwrap_or(String::from("0.0.0.0"));
    let listen_port_str = std::env::var("LISTEN_PORT").unwrap_or(String::from("7959"));
    let listen_port: u16 = listen_port_str.parse().expect("Invalid port number");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url).await.unwrap();

    sqlx::migrate!().run(&pool).await.unwrap();

    let block_service = Arc::new(BlockService::new());

    tokio::spawn(block_updater(block_service.clone()));
    tokio::spawn(submission_updater(pool.clone()));
    
    let whitelist = parse_whitelist();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(whitelist.clone()))
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(block_service.clone()))
            .service(health)
            .service(routes::work::work)
            .service(routes::submit::submit)
            .service(routes::hashrate::hashrate)
    })
    .bind((listen_address, listen_port))?
    .run()
    .await
}

fn pool_is_configured() {
    std::env::var("POOL_CONTRACT_ADDRESS").expect("POOL_CONTRACT_ADDRESS must be set");
    std::env::var("POOL_SCRIPT_HASH").expect("POOL_CONTRACT_ADDRESS must be set");
    std::env::var("POOL_OUTPUT_REFERENCE").expect("POOL_CONTRACT_ADDRESS must be set");
    std::env::var("POOL_FIXED_FEE").expect("POOL_FIXED_FEE must be set");
    std::env::var("KUPO_URL").expect("KUPO_URL must be set");
    std::env::var("OGMIOS_URL").expect("OGMIOS_URL must be set");
}

fn parse_whitelist() -> HashSet<String> {
    let whitelist_str = std::env::var("WHITELIST").unwrap_or_default();
    whitelist_str.split(',').map(|s| s.to_string()).collect()
}
