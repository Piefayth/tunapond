use actix_web::{ get, web, Responder, HttpResponse};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{common::GenericMessageResponse, model::proof_of_work::{self, ProofOfWork}};

#[derive(Debug, Deserialize)]
struct HashrateRequest {
    miner_id: Option<i64>,
    start_time: NaiveDateTime,
    end_time: NaiveDateTime
}

#[derive(Debug, Serialize)]
struct HashrateResponse {
    estimated_hash_rate: f64
}

#[get("/hashrate")]
async fn hashrate(
    pool: web::Data<SqlitePool>,
    query: web::Query<HashrateRequest>,
) -> impl Responder  {
    let maybe_pow = proof_of_work::get_by_time_range(
        &pool, query.miner_id, query.start_time, query.end_time
    ).await;

    let Ok(pow) = maybe_pow else {
        return HttpResponse::InternalServerError().json(GenericMessageResponse {
            message: format!(
                "Failed to fetch proofs of work.",
            ),
        });
    };
    
    HttpResponse::Ok().json(
        HashrateResponse { 
            estimated_hash_rate: estimate_hashrate(&pow, query.start_time, query.end_time)
        }
    )
}

fn estimate_hashes_for_difficulty(proofs: usize, zeros: u32) -> f64 {
    let p_n: f64 = 16f64.powi(-(zeros as i32));
    (proofs as f64) / p_n
}

/// Given a vec of ProofOfWork structs and a time range, calculate the hashrate.
fn estimate_hashrate(
    proofs: &Vec<ProofOfWork>,
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
) -> f64 {
    let duration = end_time - start_time;

    let valid_proofs = proofs
        .iter()
        .filter(|p| p.created_at >= start_time && p.created_at <= end_time)
        .count();
    let zeros = 8; // TODO: this value comes from somewhere else? this is "min_zeroes" really...

    let total_hashes = estimate_hashes_for_difficulty(valid_proofs, zeros);

    total_hashes / duration.num_seconds() as f64
}