use std::sync::Arc;
use cardano_multiplatform_lib::error::JsError;
use cardano_multiplatform_lib::ledger::common::value::BigInt;
use cardano_multiplatform_lib::ledger::common::value::BigNum;
use cardano_multiplatform_lib::plutus::ConstrPlutusData;
use cardano_multiplatform_lib::plutus::PlutusData;
use cardano_multiplatform_lib::plutus::PlutusList;
use cardano_multiplatform_lib::plutus::encode_json_str_to_plutus_datum;
use sha2::{Sha256, Digest};
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
use super::submission::SubmissionError;
use super::submission::submit;

#[derive(Debug)]
pub enum SubmitProofOfWorkError {
    DatabaseError(sqlx::Error),
    NoCurrentSession,
    InvalidTargetState,
    BlockServiceFailure(BlockServiceError),
    PlutusParseError(JsError),
    SubmissionError(SubmissionError)
}

impl From<SubmissionError> for SubmitProofOfWorkError {
    fn from(err: SubmissionError) -> Self {
        SubmitProofOfWorkError::SubmissionError(err)
    }
}


impl From<sqlx::Error> for SubmitProofOfWorkError {
    fn from(err: sqlx::Error) -> Self {
        SubmitProofOfWorkError::DatabaseError(err)
    }
}

impl From<JsError> for SubmitProofOfWorkError {
    fn from(err: JsError) -> Self {
        SubmitProofOfWorkError::PlutusParseError(err)
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

const SAMPLING_DIFFICULTY: u8 = 8;

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
            
            // TODO: Check that the miner is mining the block their session is assigned. Reject hashes that aren't.
                // Also, update their session assignment if the current block is new


            let current_block = block_service.get_latest()?;
            let default_nonce: [u8; 16] = [0; 16];

            let mut target_state_fields = PlutusList::new();

            let nonce_field = PlutusData::new_bytes(default_nonce.to_vec());
            let block_number_field = PlutusData::new_integer(&BigInt::from(current_block.block_number));
            let current_hash_field = PlutusData::new_bytes(current_block.current_hash.clone());
            let leading_zeroes_field = PlutusData::new_integer(&BigInt::from(current_block.leading_zeroes));
            let difficulty_number_field = PlutusData::new_integer(&BigInt::from(current_block.difficulty_number));
            let epoch_time_field = PlutusData::new_integer(&BigInt::from(current_block.epoch_time));

            target_state_fields.add(&nonce_field);
            target_state_fields.add(&block_number_field);
            target_state_fields.add(&current_hash_field);
            target_state_fields.add(&leading_zeroes_field);
            target_state_fields.add(&difficulty_number_field);
            target_state_fields.add(&epoch_time_field);

            let target_state = PlutusData::new_constr_plutus_data(
                &ConstrPlutusData::new(&BigNum::from_str("0").unwrap(), &target_state_fields)
            );

            let mut target_state_bytes = target_state.to_bytes();

            let valid_samples: Vec<_> = submission.entries.iter()
                .filter(|&entry| {
                    let sha_binding = hex::decode(&entry.sha).unwrap_or_default();
                    let sha_bytes = sha_binding.as_slice();
                    let nonce_binding = hex::decode(&entry.nonce).unwrap_or_default();
                    let nonce_bytes = nonce_binding.as_slice();
                    let entry_difficulty = get_difficulty(sha_bytes);
                    
                    if entry_difficulty.leading_zeroes < SAMPLING_DIFFICULTY as u128 {
                        return false
                    }
                    
                    target_state_bytes[4..20].copy_from_slice(nonce_bytes);
                    let hashed_data = sha256_digest_as_bytes(&target_state_bytes);
                    let hashed_hash = sha256_digest_as_bytes(&hashed_data);

                    if !hashed_hash.eq(sha_bytes){
                        return false
                    }

                    true

                })
                .collect();
            
            let num_accepted = proof_of_work::create(
                pool, 
                latest_session.id, 
                latest_session.currently_mining_block, 
                &valid_samples
            )
            .await?;

            let maybe_found_block = valid_samples.iter().find(|sample| {
                let sha_binding = hex::decode(&sample.sha).unwrap_or_default();
                let sha_bytes = sha_binding.as_slice();
                let entry_difficulty = get_difficulty(sha_bytes);
                
                let too_many_zeroes = entry_difficulty.leading_zeroes > current_block.leading_zeroes as u128;
                let just_enough_zeroes = entry_difficulty.leading_zeroes == current_block.leading_zeroes as u128;
                let enough_difficulty = entry_difficulty.difficulty_number < current_block.difficulty_number as u128;
                if too_many_zeroes || (just_enough_zeroes && enough_difficulty) {
                    true
                } else {
                    false
                }
            });

            match maybe_found_block {
                Some(entry) => {
                    submit(
                        pool, 
                        &current_block, 
                        hex::decode(&entry.sha).unwrap_or_default().as_slice(), 
                        hex::decode(&entry.nonce).unwrap_or_default().as_slice()
                    ).await?;
                },
                None => {}
            }



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

fn sha256_digest_as_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let arr: [u8; 32] = result.into();
    arr
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

/// Given a vec of ProofOfWork structs and a time range, calculate the hashrate.
fn estimate_hashrate(proofs: &Vec<ProofOfWork>, start_time: NaiveDateTime, end_time: NaiveDateTime) -> f64 {
    let duration = end_time - start_time;
    
    let valid_proofs = proofs.iter().filter(|p| p.created_at >= start_time && p.created_at <= end_time).count();
    let zeros = 8; // TODO: this value comes from somewhere else? this is "min_zeroes" really... 
    
    let total_hashes = estimate_hashes_for_difficulty(valid_proofs, zeros);
    
    total_hashes / duration.num_seconds() as f64
}

// TODO: Why are these u128s?
#[derive(Default)]
pub struct Difficulty {
    pub leading_zeroes: u128,
    pub difficulty_number: u128
}

pub fn get_difficulty(hash: &[u8]) -> Difficulty {
    if hash.len() != 32 {
        return Difficulty::default()
    }

    let mut leading_zeroes = 0;
    let mut difficulty_number = 0;

    for (indx, &chr) in hash.iter().enumerate() {
        if chr != 0 {
            if (chr & 0x0F) == chr {
                leading_zeroes += 1;
                difficulty_number += (chr as u128) * 4096;
                difficulty_number += (hash[indx + 1] as u128) * 16;
                difficulty_number += (hash[indx + 2] as u128) / 16;
                return Difficulty { leading_zeroes, difficulty_number }
            } else {
                difficulty_number += (chr as u128) * 256;
                difficulty_number += hash[indx + 1] as u128;
                return Difficulty { leading_zeroes, difficulty_number }
            }
        } else {
            leading_zeroes += 2;
        }
    }
    return Difficulty { leading_zeroes: 32, difficulty_number: 0 }
}