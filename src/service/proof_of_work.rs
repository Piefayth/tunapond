use std::sync::Arc;
use chrono::NaiveDateTime;
use chrono::Duration;
use serde::{Serialize, Deserialize};
use sqlx::SqlitePool;
use crate::model::mining_session;
use crate::model::proof_of_work::ProofOfWork;
use crate::model::proof_of_work::{self};
use crate::model::mining_session::get_latest;
use crate::routes::submit::Submission;

use super::block::{BlockService, BlockServiceError, ReadableBlock};

#[derive(Debug)]
pub enum SubmitProofOfWorkError {
    DatabaseError(sqlx::Error),
    NoCurrentSession,
    BlockServiceFailure(BlockServiceError),
}

impl From<sqlx::Error> for SubmitProofOfWorkError {
    fn from(err: sqlx::Error) -> Self {
        SubmitProofOfWorkError::DatabaseError(err)
    }
}

impl From<BlockServiceError> for SubmitProofOfWorkError {
    fn from(err: BlockServiceError) -> Self {
        SubmitProofOfWorkError::BlockServiceFailure(err)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitProofOfWorkResponse {
    num_accepted: u64,
    session_id: i64,
    working_block: ReadableBlock,
}

pub async fn submit_proof_of_work(
    pool: &SqlitePool,
    block_service: &Arc<BlockService>,
    pkh: String,
    submission: &Submission,
) -> Result<SubmitProofOfWorkResponse, SubmitProofOfWorkError> {
    let maybe_latest_session = get_latest(pool, &pkh).await?;

    match maybe_latest_session {
        Some(latest_session) => {
            // TODO: Some database errors (like primary key collisions) are a result of malicious miner
                // behavior. Depending on the error, miners may need removed from the pool...

            // TODO: Check how many of the submitted were actually valid for the target state
                // Also check that they had sufficient 0s for the baseline sampling difficulty
            
            // TODO: Check that the miner is mining the block their session is assigned. Reject hashes that aren't.
                // Also, update their session assignment if the current block is new
            let num_accepted = proof_of_work::create(pool, latest_session.id, &submission.entries)
                .await
                .map_err(SubmitProofOfWorkError::from)?;

            let current_block = block_service.get_latest()?;

            Ok(
                SubmitProofOfWorkResponse {
                    num_accepted: num_accepted,
                    session_id: latest_session.id,
                    working_block: current_block.into()
                }
            )
        }
        None => {
            Err(SubmitProofOfWorkError::NoCurrentSession)
        }
    }

}

#[derive(Debug)]
pub enum GetHashrateError {
    DatabaseError(sqlx::Error),
    SessionNotFound,
}

impl From<sqlx::Error> for GetHashrateError {
    fn from(err: sqlx::Error) -> Self {
        GetHashrateError::DatabaseError(err)
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct HashrateResult {
    pub estimated_hashes_per_second: f64
}

pub async fn get_session_hashrate(
    pool: &SqlitePool,
    mining_session_id: i64
) -> Result<HashrateResult, GetHashrateError>{
    let hashes = proof_of_work::get(pool, mining_session_id).await?;
    let session = mining_session::get_from_id(pool, mining_session_id)
        .await?
        .ok_or(GetHashrateError::SessionNotFound)?;

    let start_time = session.start_time;
    let end_time = match session.end_time {
        Some(session_end_time) => session_end_time,
        None =>  chrono::Utc::now().naive_utc()
    };

    let hashrate = estimate_hashrate(&hashes, start_time, end_time);

    Ok(
        HashrateResult {
            estimated_hashes_per_second: hashrate
        }
    )
}

pub async fn get_proof_of_work(
    pool: &SqlitePool,
    mining_session_id: i64,
) -> Result<Vec<ProofOfWork>, sqlx::Error> {
    proof_of_work::get(pool, mining_session_id).await
}


fn estimate_hashes_for_difficulty(proofs: usize, zeros: u32) -> f64 {
    let p_n: f64 = 16f64.powi(-(zeros as i32));
    (proofs as f64) / p_n
}

fn calculate_hashrate(total_hashes: f64, duration: Duration) -> f64 {
    total_hashes / duration.num_seconds() as f64
}

/// Given a vec of ProofOfWork structs and a time range, calculate the hashrate.
fn estimate_hashrate(proofs: &Vec<ProofOfWork>, start_time: NaiveDateTime, end_time: NaiveDateTime) -> f64 {
    let duration = end_time - start_time;
    
    let valid_proofs = proofs.iter().filter(|p| p.created_at >= start_time && p.created_at <= end_time).count();
    let zeros = 8; // TODO: this value comes from somewhere else? this is "min_zeroes" really... 
    
    let total_hashes = estimate_hashes_for_difficulty(valid_proofs, zeros);
    
    calculate_hashrate(total_hashes, duration)
}