use std::{sync::Arc, collections::HashSet};

use actix_web::{get, web, HttpResponse, Responder};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use crate::{
    common::GenericMessageResponse,
    address::{self},
    model::miner::{create_miner, get_miner_by_pkh, update_sampling_difficulty_by_pkh},
    service::{
        block::{BlockService, ReadableBlock}, proof_of_work::block_to_target_state,
    },
};

#[derive(Debug, Deserialize)]
struct WorkRequest {
    address: String,
    sample_diff: Option<u8>,
    raw: Option<bool>,
}

#[derive(Debug, Serialize)]
struct WorkResponse {
    miner_id: i32,
    nonce: String,
    min_zeroes: u8,
    current_block: ReadableBlock,
}

#[derive(Debug, Serialize)]
struct RawWorkResponse {
    miner_id: i32,
    min_zeroes: u8,
    raw_target_state: String,
}

#[get("/work")]
async fn work(
    pool: web::Data<Pool<Postgres>>, 
    query: web::Query<WorkRequest>,
    block_service: web::Data<Arc<BlockService>>,
    whitelist: web::Data<HashSet<String>>,
) -> impl Responder {
    let maybe_pkh = address::pkh_from_address(&query.address);

    let Ok(pkh) = maybe_pkh else {
        return HttpResponse::BadRequest().json(GenericMessageResponse {
            message: format!(
                "Could not create a valid public key hash for address {}",
                query.address
            ),
        });
    };

    // Empty whitelist allows everyone
    if !whitelist.is_empty() && !whitelist.contains(&pkh) {
        return HttpResponse::Forbidden().json(GenericMessageResponse {
            message: "Access denied".to_string(),
        });
    }

    let minimum_sample_diff = 5;    // might not be needed with the limits on /submit
    let default_sample_diff = 8;
    let requested_sampling_diff = match query.sample_diff {
        Some(diff_req) => {
            if diff_req < minimum_sample_diff {
                minimum_sample_diff
            } else {
                diff_req
            }
        }
        None => {
            default_sample_diff
        }
    };

    let maybe_maybe_miner = get_miner_by_pkh(&pool, &pkh).await;

    let miner = match maybe_maybe_miner {
        Ok(maybe_miner) => match maybe_miner {
            Some(miner) => miner,
            None => {
                let Ok(miner) = create_miner(&pool, pkh, query.address.clone()).await else {
                    return HttpResponse::InternalServerError().json(GenericMessageResponse {
                        message: format!("Could not save new miner {}", query.address),
                    });
                };
                miner
            }
        },
        Err(_) => {
            return HttpResponse::InternalServerError().json(GenericMessageResponse {
                message: format!("Failed to retrieve miner status."),
            });
        }
    };

    // if the miners requested sampling diff is different from what they requested, update it 
    let miner = if miner.sampling_difficulty as u8 != requested_sampling_diff {
        let Ok(updated_miner) = update_sampling_difficulty_by_pkh(&pool, &miner.pkh, requested_sampling_diff).await else {
            return HttpResponse::InternalServerError().json(GenericMessageResponse {
                message: format!("Could not update miner {}", query.address),
            });
        };
        updated_miner
    } else {
        miner
    };

    let nonce = generate_nonce(miner.id);

    let Ok(current_block) = block_service.get_latest() else {
        return HttpResponse::InternalServerError().json(GenericMessageResponse {
            message: format!("Could not retrieve current block state."),
        });
    };

    if query.raw.is_some() && query.raw.unwrap() {
        HttpResponse::Ok().json(RawWorkResponse {
            miner_id: miner.id,
            min_zeroes: miner.sampling_difficulty as u8,
            raw_target_state: hex::encode(block_to_target_state(&current_block, &nonce).to_bytes())
        })
    } else {
        HttpResponse::Ok().json(WorkResponse {
            nonce: hex::encode(nonce),
            miner_id: miner.id,
            min_zeroes: miner.sampling_difficulty as u8,
            current_block: current_block.into()
        })
    }

}

pub fn generate_nonce(miner_id: i32) -> [u8; 16] {
    let pool_id: u8 = std::env::var("POOL_ID")
        .expect("POOL_ID must be set")
        .parse()
        .expect("POOL_ID must be a valid number");

    let mut rng = rand::thread_rng();

    let mut nonce = [0u8; 16]; // 16 bytes in total

    // Generate 12 random bytes
    nonce[0..12].copy_from_slice(&rng.gen::<[u8; 12]>());

    // 3 byte miner_id
    nonce[12..15].copy_from_slice(&miner_id.to_be_bytes()[1..]);

    // 1 byte pool_id
    nonce[15] = pool_id;

    nonce
}