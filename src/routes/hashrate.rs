use actix_web::{ get, web, Responder, HttpResponse};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Pool};

use crate::{common::GenericMessageResponse, model::proof_of_work::{self, MinerProofCount}};

#[derive(Debug, Deserialize)]
struct HashrateRequest {
    miner_id: Option<i32>,
    start_time: u64,
    end_time: Option<u64>
}

#[derive(Debug, Serialize)]
struct HashrateResponse {
    estimated_hash_rate: f64
}

#[get("/hashrate")]
async fn hashrate(
    pool: web::Data<Pool<Postgres>>,
    query: web::Query<HashrateRequest>,
) -> impl Responder  {
    let now = Utc::now().naive_utc();
    let start_time = NaiveDateTime::from_timestamp_opt(query.start_time as i64, 0);
    let end_time_value = query.end_time.unwrap_or(now.timestamp_millis() as u64 / 1000);
    let end_time = NaiveDateTime::from_timestamp_opt(end_time_value as i64, 0);

    if start_time.is_none() || end_time.is_none() {
        return HttpResponse::BadRequest().json(GenericMessageResponse {
            message: format!(
                "Timestamp input was invalid.",
            ),
        });
    };

    let maybe_pow = proof_of_work::count_by_time_range(
        &pool, query.miner_id, start_time.unwrap(), end_time.unwrap()
    ).await;

    let pow = match maybe_pow {
        Ok(p) => p,
        Err(_) => return HttpResponse::InternalServerError().json(GenericMessageResponse {
            message: format!("Failed to fetch proofs of work."),
        })
    };
    
    HttpResponse::Ok().json(
        HashrateResponse { 
            estimated_hash_rate: estimate_hashrate(pow, start_time.unwrap(), end_time.unwrap())
        }
    )
}

pub fn estimate_hashes_for_difficulty(proof_count: usize, zeros: u8) -> f64 {
    let estimated_proofs_per_proof: f64 = 16f64.powi(zeros as i32);  
    (proof_count as f64) * estimated_proofs_per_proof
}

pub fn estimate_hashrate(
    proofs: Vec<MinerProofCount>,
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
) -> f64 {
    let duration = end_time - start_time;

    let total_hashes: f64 = proofs.into_iter().map(|proof| {
        estimate_hashes_for_difficulty(proof.proof_count as usize, proof.sampling_difficulty as u8)
    }).sum();

    total_hashes / duration.num_seconds() as f64
}