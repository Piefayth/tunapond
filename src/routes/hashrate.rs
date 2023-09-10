use actix_web::{ get, web, Responder, HttpResponse};
use serde::{Deserialize};
use sqlx::SqlitePool;

use crate::common::GenericMessageResponse;

#[derive(Debug, Deserialize)]
struct HashrateRequest {
    miner_id: i64,
}

#[get("/hashrate")]
async fn hashrate(
    pool: web::Data<SqlitePool>,
    query: web::Query<HashrateRequest>,
) -> impl Responder  {
    HttpResponse::Ok().json(
        GenericMessageResponse { 
            message: format!("Very fast")
        }
    )
}