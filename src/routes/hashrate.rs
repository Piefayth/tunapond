use actix_web::{ get, web, Responder, HttpResponse};
use serde::{Deserialize};
use sqlx::SqlitePool;

use crate::{routes::common::GenericMessageResponse, address, service::{proof_of_work::{submit_proof_of_work, SubmitProofOfWorkError, SubmitProofOfWorkResponse, self}, block::BlockService}};

#[derive(Debug, Deserialize)]
struct HashrateRequest {
    session_id: i64,
}

#[get("/hashrate")]
async fn hashrate(
    pool: web::Data<SqlitePool>,
    query: web::Query<HashrateRequest>,
) -> impl Responder  {
    let maybe_hashrate = proof_of_work::get_session_hashrate(&pool, query.session_id)
        .await;

    match maybe_hashrate {
        Ok(hashrate) => {
            HttpResponse::Ok().json(hashrate)
        },
        Err(_) => {
            HttpResponse::BadRequest().json(
                GenericMessageResponse { 
                    message: format!("Could not calculate a hashrate for session {}", query.session_id)
                }
            )
        }
    }
}