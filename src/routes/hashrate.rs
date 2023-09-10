use actix_web::{ get, web, Responder, HttpResponse};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{common::GenericMessageResponse, model::proof_of_work::{self, ProofOfWork}};

#[derive(Debug, Deserialize)]
struct HashrateRequest {
    miner_id: Option<i64>,
    start_time: u64,
    end_time: Option<u64>
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

    let maybe_pow = proof_of_work::get_by_time_range(
        &pool, query.miner_id, start_time.unwrap(), end_time.unwrap()
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
            estimated_hash_rate: estimate_hashrate(&pow, start_time.unwrap(), end_time.unwrap())
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