use std::collections::HashMap;
use std::sync::Arc;
use lazy_static::lazy_static;
use actix_web::{ post, web, HttpResponse, Responder};
use serde::{Serialize, Deserialize};
use sqlx::{Postgres, Pool};
use tokio::sync::Mutex;
use tokio::time::Instant;
use crate::common::GenericMessageResponse;
use crate::routes::work::generate_nonce;
use crate::service::proof_of_work::{block_to_target_state, RawSubmitProofOfWorkResponse};
use crate::{address, service::{proof_of_work::{submit_proof_of_work, SubmitProofOfWorkError}, block::BlockService}, model::miner::get_miner_by_pkh};

#[derive(Debug, Deserialize)]
struct SubmissionQuery {
    raw: Option<bool>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Submission {
    pub address: String,
    pub entries: Vec<SubmissionEntry>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmissionEntry {
    pub nonce: String
}


lazy_static! {
    static ref RATE_LIMITER: Mutex<HashMap<String, Instant>> = Mutex::new(HashMap::new());
}

#[post("/submit")]
async fn submit(
    pool: web::Data<Pool<Postgres>>,
    block_service: web::Data<Arc<BlockService>>,
    submission: web::Json<Submission>,
    query: web::Query<SubmissionQuery>,
) -> impl Responder {
    let max_entries: usize = std::env::var("MAX_SUBMISSION_ENTRIES")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .unwrap_or(10);

    let submit_frequency_ms: u64 = std::env::var("SUBMIT_FREQUENCY_MS")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .unwrap_or(1000);

    if submission.entries.len() > max_entries {
        return HttpResponse::BadRequest().json(
            GenericMessageResponse { 
                message: format!("Too many entries in submission. Max allowed is {}", max_entries)
            }
        )
    }

    let maybe_pkh = address::pkh_from_address(&submission.address);
    
    let Ok(pkh) = maybe_pkh else {
        return HttpResponse::BadRequest().json(
            GenericMessageResponse { 
                message: format!("Could not create a valid public key hash for address {}", submission.address)
            }
        )
    };

    let mut rate_limiter = RATE_LIMITER.lock().await;

    if let Some(last_request_time) = rate_limiter.get(&pkh) {
        if last_request_time.elapsed() < tokio::time::Duration::from_millis(submit_frequency_ms) {
            return HttpResponse::TooManyRequests().json(
                GenericMessageResponse { 
                    message: String::from("Too many requests.")
                }
            );
        }
    }

    rate_limiter.insert(pkh.clone(), Instant::now());
    
    let maybe_maybe_miner = get_miner_by_pkh(&pool, &pkh).await;
    let Ok(maybe_miner) = maybe_maybe_miner else {
        return HttpResponse::NotFound().json(
            GenericMessageResponse { 
                message: format!("Cannot validate nonce for unseen miner. Please get some /work!")
            }
        )
    };

    let Some(miner) = maybe_miner else {
        return HttpResponse::NotFound().json(
            GenericMessageResponse { 
                message: format!("Cannot validate nonce for unseen miner. Please get some /work!")
            }
        )
    };

    let result = submit_proof_of_work(&pool, &block_service, miner.id, miner.sampling_difficulty as u8, &submission).await;

    match result {
        Ok(submission_response) => {
            if query.raw.unwrap_or(false) {
                let current_block = block_service.get_latest().unwrap_or_default();
                let nonce = generate_nonce(miner.id);


                HttpResponse::Ok().json(RawSubmitProofOfWorkResponse {
                    num_accepted: submission_response.num_accepted,
                    raw_target_state: hex::encode(block_to_target_state(&current_block, &nonce).to_bytes())
                })
            } else {
                HttpResponse::Ok().json(submission_response)
            }
        },
        Err(e) => {
            match e {
                SubmitProofOfWorkError::DatabaseError(_) => {
                    HttpResponse::InternalServerError().json(
                        GenericMessageResponse { 
                            message: String::from("Unexpected database error.")
                        }
                    )
                },
                SubmitProofOfWorkError::BlockServiceFailure(_) => {
                    HttpResponse::InternalServerError().json(
                        GenericMessageResponse { 
                            message: format!("Could not verify submission - BlockService is down.")
                        }
                    )
                },
                SubmitProofOfWorkError::PlutusParseError(_) => {
                    HttpResponse::InternalServerError().json(
                        GenericMessageResponse { 
                            message: format!("Could not verify submission - unable to parse plutus data.")
                        }
                    )
                },
                SubmitProofOfWorkError::SubmissionError(_) => {
                    HttpResponse::InternalServerError().json(
                        GenericMessageResponse { 
                            message: format!("Failed to submit a valid block!")
                        }
                    )
                }
            }
        },
    }
}