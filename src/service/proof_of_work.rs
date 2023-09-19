use crate::model::proof_of_work::{self};
use crate::routes::submit::Submission;
use crate::routes::work::generate_nonce;
use cardano_multiplatform_lib::error::JsError;
use cardano_multiplatform_lib::ledger::common::value::BigInt;
use cardano_multiplatform_lib::ledger::common::value::BigNum;
use cardano_multiplatform_lib::plutus::ConstrPlutusData;
use cardano_multiplatform_lib::plutus::PlutusData;
use cardano_multiplatform_lib::plutus::PlutusList;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Postgres};
use std::sync::Arc;

use super::block::{Block, BlockService, BlockServiceError, ReadableBlock};
use super::submission::submit;
use super::submission::SubmissionError;

#[derive(Debug)]
pub enum SubmitProofOfWorkError {
    DatabaseError(sqlx::Error),
    BlockServiceFailure(BlockServiceError),
    PlutusParseError(JsError),
    SubmissionError(SubmissionError),
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
    pub num_accepted: u64,
    pub nonce: String,
    pub working_block: ReadableBlock,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawSubmitProofOfWorkResponse {
    pub num_accepted: u64,
    pub raw_target_state: String,
}

#[derive(Debug, Clone)]
pub struct ProcessedSubmissionEntry {
    pub miner_id: i32,
    pub block_number: i32,
    pub nonce: [u8; 16],
    pub sha: [u8; 32],
    pub sampling_difficulty: u8, 
}

pub fn block_to_target_state(block: &Block, nonce: &[u8; 16]) -> PlutusData {
    let mut target_state_fields = PlutusList::new();

    let nonce_field = PlutusData::new_bytes(nonce.to_vec());
    let block_number_field = PlutusData::new_integer(&BigInt::from(block.block_number));
    let current_hash_field = PlutusData::new_bytes(block.current_hash.clone());
    let leading_zeroes_field = PlutusData::new_integer(&BigInt::from(block.leading_zeroes));
    let difficulty_number_field = PlutusData::new_integer(&BigInt::from(block.difficulty_number));
    let epoch_time_field = PlutusData::new_integer(&BigInt::from(block.epoch_time));

    target_state_fields.add(&nonce_field);
    target_state_fields.add(&block_number_field);
    target_state_fields.add(&current_hash_field);
    target_state_fields.add(&leading_zeroes_field);
    target_state_fields.add(&difficulty_number_field);
    target_state_fields.add(&epoch_time_field);

    let target_state = PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(
        &BigNum::from_str("0").unwrap(),
        &target_state_fields,
    ));

    return target_state;
}

pub async fn submit_proof_of_work(
    pool: &Pool<Postgres>,
    block_service: &Arc<BlockService>,
    miner_id: i32,
    miner_sampling_difficulty: u8,
    submission: &Submission,
) -> Result<SubmitProofOfWorkResponse, SubmitProofOfWorkError> {
    let pool_id: u8 = std::env::var("POOL_ID")
        .expect("POOL_ID must be set")
        .parse()
        .expect("POOL_ID must be a valid number");

    // TODO: Some database errors (like primary key collisions) are a result of malicious miner
    // behavior. Depending on the error, miners may need removed from the pool...

    let current_block = block_service.get_latest()?;
    let nonce = generate_nonce(miner_id);
    let mut target_state_bytes = block_to_target_state(&current_block, &nonce).to_bytes();

    let valid_samples: Vec<ProcessedSubmissionEntry> = submission
        .entries
        .iter()
        .filter_map(|entry| {
            let nonce_binding = hex::decode(&entry.nonce).unwrap_or_default();

            if nonce_binding.len() != 16 {
                return None;
            }
            let nonce_bytes: [u8; 16] = nonce_binding.try_into().unwrap();

            target_state_bytes[4..20].copy_from_slice(&nonce_bytes);
            let hashed_data = sha256_digest_as_bytes(&target_state_bytes);
            let hashed_hash = sha256_digest_as_bytes(&hashed_data);

            let entry_difficulty = get_difficulty(&hashed_hash);
            if entry_difficulty.leading_zeroes < miner_sampling_difficulty as u128 {
                return None;
            }

            if !verify_nonce(&nonce_bytes, miner_id, pool_id) {
                return None;
            }

            return Some(ProcessedSubmissionEntry {
                miner_id,
                block_number: current_block.block_number,
                nonce: nonce_bytes,
                sha: hashed_hash,
                sampling_difficulty: miner_sampling_difficulty,
            });
        })
        .collect();

    let _ =
        proof_of_work::create(pool, miner_id, current_block.block_number, &valid_samples).await?;

    let maybe_found_block = valid_samples.iter().find(|sample| {
        let entry_difficulty = get_difficulty(&sample.sha);

        let too_many_zeroes =
            entry_difficulty.leading_zeroes > current_block.leading_zeroes as u128;
        let just_enough_zeroes =
            entry_difficulty.leading_zeroes == current_block.leading_zeroes as u128;
        let enough_difficulty =
            entry_difficulty.difficulty_number < current_block.difficulty_number as u128;

        // to keep the submission server from exploding due to request volume in preview, submission are minimum 10
        let enough_preview_zeroes = entry_difficulty.leading_zeroes > 9;
        let is_true_new_block = too_many_zeroes || (just_enough_zeroes && enough_difficulty);

        enough_preview_zeroes && is_true_new_block
    });

    match maybe_found_block {
        Some(entry) => {
            let _ = submit(pool, &current_block, miner_id, &entry.sha, &entry.nonce).await;
        }
        None => {}
    }

    Ok(SubmitProofOfWorkResponse {
        num_accepted: valid_samples.len() as u64,
        working_block: current_block.into(),
        nonce: hex::encode(&nonce),
    })
}

fn sha256_digest_as_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let arr: [u8; 32] = result.into();
    arr
}

fn verify_nonce(nonce_bytes: &[u8], miner_id: i32, pool_id: u8) -> bool {
    if nonce_bytes.len() != 16 {
        return false;
    }

    // Extract the last 4 bytes
    let last_4_bytes = &nonce_bytes[12..16];

    // Compare the first 3 bytes of the last 4 bytes to miner_id
    if &miner_id.to_be_bytes()[1..] != &last_4_bytes[..3] {
        return false;
    }

    // Compare the last byte to pool_id
    if pool_id != last_4_bytes[3] {
        return false;
    }

    true
}

#[derive(Debug)]
pub enum GetHashrateError {
    DatabaseError(sqlx::Error),
}

impl From<sqlx::Error> for GetHashrateError {
    fn from(err: sqlx::Error) -> Self {
        GetHashrateError::DatabaseError(err)
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct HashrateResult {
    pub estimated_hashes_per_second: f64,
}

// TODO: Why are these u128s?
#[derive(Default)]
pub struct Difficulty {
    pub leading_zeroes: u128,
    pub difficulty_number: u128,
}

pub fn get_difficulty(hash: &[u8]) -> Difficulty {
    if hash.len() != 32 {
        return Difficulty::default();
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
                return Difficulty {
                    leading_zeroes,
                    difficulty_number,
                };
            } else {
                difficulty_number += (chr as u128) * 256;
                difficulty_number += hash[indx + 1] as u128;
                return Difficulty {
                    leading_zeroes,
                    difficulty_number,
                };
            }
        } else {
            leading_zeroes += 2;
        }
    }
    return Difficulty {
        leading_zeroes: 32,
        difficulty_number: 0,
    };
}
