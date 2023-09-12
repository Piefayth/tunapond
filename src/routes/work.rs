use std::sync::Arc;

use actix_web::{get, web, HttpResponse, Responder};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use crate::{
    common::GenericMessageResponse,
    address::{self},
    model::miner::{create_miner, get_miner_by_pkh},
    service::{
        block::{BlockService, ReadableBlock},
    },
};

#[derive(Debug, Deserialize)]
struct WorkRequest {
    address: String,
}

#[derive(Debug, Serialize)]
struct WorkResponse {
    nonce: String,
    min_zeroes: u8,
    current_block: ReadableBlock,
}

#[get("/work")]
async fn work(
    pool: web::Data<SqlitePool>, 
    query: web::Query<WorkRequest>,
    block_service: web::Data<Arc<BlockService>>,
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

    let maybe_maybe_miner = get_miner_by_pkh(&pool, &pkh).await;

    let miner_id = match maybe_maybe_miner {
        Ok(maybe_miner) => match maybe_miner {
            Some(miner) => miner.id,
            None => {
                let Ok(miner_id) = create_miner(&pool, pkh, query.address.clone()).await else {
                    return HttpResponse::InternalServerError().json(GenericMessageResponse {
                        message: format!("Could not save new miner {}", query.address),
                    });
                };
                miner_id as i64
            }
        },
        Err(_) => {
            return HttpResponse::InternalServerError().json(GenericMessageResponse {
                message: format!("Failed to retrieve miner status."),
            });
        }
    };

    let nonce = generate_nonce(miner_id);

    let Ok(current_block) = block_service.get_latest() else {
        return HttpResponse::InternalServerError().json(GenericMessageResponse {
            message: format!("Could not retrieve current block state."),
        });
    };

    HttpResponse::Ok().json(WorkResponse {
        nonce,
        min_zeroes: 8,
        current_block: current_block.into()
    })
}

pub fn generate_nonce(miner_id: i64) -> String {
    let pool_id: u8 = std::env::var("POOL_ID")
        .expect("POOL_ID must be set")
        .parse()
        .expect("POOL_ID must be a valid number");

    let mut rng = rand::thread_rng();

    let mut nonce = vec![0u8; 16]; // 16 bytes in total
    
    // Generate 12 random bytes
    for i in 0..12 {
        nonce[i] = rng.gen::<u8>();
    }
    
    // 3 byte miner_id
    nonce[12..15].copy_from_slice(&miner_id.to_be_bytes()[..3]);
    
    // 1 byte pool_id
    nonce[15] = pool_id;

    hex::encode(nonce)
}