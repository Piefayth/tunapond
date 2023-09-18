use std::collections::HashMap;

use cardano_multiplatform_lib::{
    error::JsError,
};
use chrono::{Utc};
use serde::{Deserialize, Serialize};
use sqlx::{ Postgres, Pool};

use crate::{
    model::{
        datum_submission::{
            self, accept, get_newest_confirmed_datum, get_unconfirmed, reject, DatumSubmission,
        },
        proof_of_work::{self, get_by_time_range, cleanup_old_proofs, count_by_time_range_by_miner_id},
    },
    routes::hashrate::{estimate_hashrate, estimate_hashrate_numeric},
    service::proof_of_work::get_difficulty,
};

use super::block::{Block, KupoUtxo};

#[derive(Debug)]
pub enum SubmissionError {
    DatabaseError(sqlx::Error),
    JsError(JsError),
    ReqwestError(reqwest::Error),
}

impl From<sqlx::Error> for SubmissionError {
    fn from(err: sqlx::Error) -> Self {
        SubmissionError::DatabaseError(err)
    }
}

impl From<JsError> for SubmissionError {
    fn from(err: JsError) -> Self {
        SubmissionError::JsError(err)
    }
}

impl From<reqwest::Error> for SubmissionError {
    fn from(err: reqwest::Error) -> Self {
        SubmissionError::ReqwestError(err)
    }
}

const TUNA_PER_DATUM: usize = 5_000_000_000;
#[derive(Debug, Serialize)]
pub struct DenoSubmission {
    nonce: String,
    sha: String,
    current_block: Block,
    new_zeroes: i64,
    new_difficulty: i64,
    miner_payments: HashMap<String, usize>, // <Address, Payment>
    hash_rate: f64,
}
#[derive(Deserialize)]
pub struct DenoSubmissionResponse {
    tx_hash: String,
    //message: String,
}

pub async fn submit(
    pool: &Pool<Postgres>,
    current_block: &Block,
    miner_id: i32,
    sha: &[u8],
    nonce: &[u8],
) -> Result<(), SubmissionError> {
    let new_diff_data = get_difficulty(sha);

    let default_fee: i64 = 25000000;
    let pool_fixed_fee: i64 = std::env::var("POOL_FIXED_FEE")
        .map(|s| s.parse().unwrap_or(default_fee))
        .unwrap_or(default_fee);

    let default_finders_fee: i64 = 20000000;
    let finders_fee: i64 = std::env::var("POOL_FIXED_FEE")
        .map(|s| s.parse().unwrap_or(default_finders_fee))
        .unwrap_or(default_finders_fee);

    let total_payout = TUNA_PER_DATUM - pool_fixed_fee as usize - finders_fee as usize;

    let maybe_last_paid_datum = get_newest_confirmed_datum(pool).await?;

    let start_time = match maybe_last_paid_datum {
        Some(last_paid_datum) => {
            let maybe_last_confirmed = last_paid_datum.confirmed_at;
            if maybe_last_confirmed.is_some() {
                maybe_last_confirmed.unwrap()
            } else {
                proof_of_work::get_oldest(pool)
                    .await
                    .unwrap()
                    .unwrap()
                    .created_at
            }
        }
        None => {
            // if no datum has ever been paid before, start from the first proof of work
            proof_of_work::get_oldest(pool)
                .await
                .unwrap()
                .unwrap()
                .created_at // invariant upheld by the fact that we have a datum
        }
    };

    let end_time = Utc::now().naive_utc();

    let miner_counts = count_by_time_range_by_miner_id(pool, start_time, end_time).await?;

    let estimated_hashrate_total = miner_counts.iter().fold(0.0, |acc, m| {
        acc + estimate_hashrate_numeric(m.proof_count as usize, start_time, end_time)
    });
    
    let mut miner_payments: HashMap<String, usize> = HashMap::new();
    for miner_count in &miner_counts {
        let miner_hashrate = estimate_hashrate_numeric(miner_count.proof_count as usize, start_time, end_time);
        let miner_share = miner_hashrate / estimated_hashrate_total;
        let miner_payment = (total_payout as f64 * miner_share) as usize;
        let miner_bonus = if miner_count.miner_id == miner_id {
            finders_fee
        } else {
            0
        } as usize;
        miner_payments.insert(miner_count.miner_address.clone(), miner_payment + miner_bonus);
    }

    let submission = DenoSubmission {
        nonce: hex::encode(nonce),
        sha: hex::encode(sha),
        current_block: current_block.clone(),
        new_difficulty: new_diff_data.difficulty_number as i64,
        new_zeroes: new_diff_data.leading_zeroes as i64,
        miner_payments: miner_payments.clone(),
        hash_rate: estimated_hashrate_total,
    };

    let response: DenoSubmissionResponse = reqwest::Client::new()
        .post("http://localhost:22123/submit")
        .json(&submission)
        .send()
        .await?
        .json()
        .await?;

    log::info!("Submitted datum on chain in tx_hash {}", &response.tx_hash);

    datum_submission::create(
        pool,
        response.tx_hash.clone(),
        hex::encode(sha),
        current_block.block_number,
    )
    .await?;

    Ok(())
}

pub async fn submission_updater(pool: Pool<Postgres>) {
    let kupo_url = std::env::var("KUPO_URL")
        .expect("Cannot instantiate BlockService because KUPO_URL is not set.");

    let default_number_of_datums_to_retain_old_proofs_for: i64 = 10;
    let number_of_datums_to_retain_old_proofs_for: i64 = std::env::var("PROOF_RETENTION_LENGTH_IN_DATUMS")
        .map(|s| s.parse().unwrap_or(default_number_of_datums_to_retain_old_proofs_for))
        .unwrap_or(default_number_of_datums_to_retain_old_proofs_for);

    let interval = 60;

    let client = reqwest::Client::new();

    loop {
        let unconfirmed_datums = get_unconfirmed(&pool).await;

        let Ok(unconfirmed) = unconfirmed_datums else {
            log::error!("Submission updater could not fetch unconfirmed.");
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
            continue;
        };

        if !unconfirmed.is_empty() {
            for datum in unconfirmed {
                let url = format!("{}/matches/*@{}", kupo_url, datum.transaction_hash);

                let resp = client.get(&url).send().await;

                let Ok(r) = resp else {
                    log::warn!(
                        "Failed to fetch matches for transaction_id: {}",
                        datum.transaction_hash
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
                    continue;
                };

                let response_result: Result<Vec<KupoUtxo>, reqwest::Error> = r.json().await;
                let Ok(kupo_utxos) = response_result else {
                    log::error!(
                        "Failed to parse kupo transaction! Got {:?}",
                        response_result
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
                    continue;
                };

                let tx_hash = datum.transaction_hash.clone();
                if !kupo_utxos.is_empty() {
                    let slot_no = kupo_utxos[0].created_at.slot_no;
                    let datum = DatumSubmission {
                        confirmed_in_slot: Some(slot_no),
                        ..datum
                    };
                    let result = accept(&pool, vec![datum]).await;
                    log::info!("Permanently accepted datum at transaction {}.", tx_hash);
                    let Ok(_) = result else {
                        log::error!("Failed to accept datum with transaction_id: {}", tx_hash);
                        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
                        continue;
                    };
                } else {
                    let now = Utc::now().naive_utc();
                    let age = now.signed_duration_since(datum.created_at).num_minutes();

                    if age > 2 {
                        let result = reject(&pool, vec![datum]).await;
                        let Ok(_) = result else {
                            log::error!("Failed to reject datum with transaction_id: {}", tx_hash);
                            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
                            continue;
                        };
                    }
                }
            }
        }

        let Ok(_) = cleanup_old_proofs(&pool, number_of_datums_to_retain_old_proofs_for).await else {
            log::error!("Failed to cleanup old proofs.");
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
            continue;
        };

        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
    }
}
