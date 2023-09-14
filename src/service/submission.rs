use std::collections::HashMap;

use cardano_multiplatform_lib::{plutus::{PlutusData, PlutusList, ConstrPlutusData, ExUnitPrices, self}, ledger::{common::{value::{Int, BigInt, BigNum, Value}, utxo::TransactionUnspentOutput}, alonzo::{fees::LinearFee, self}}, builders::tx_builder::{TransactionBuilderConfigBuilder, TransactionBuilder}, UnitInterval, chain_crypto::Ed25519, crypto::{PrivateKey, Bip32PrivateKey, TransactionHash}, address::{StakeCredential, Address}, TransactionInput, error::JsError, TransactionOutput, genesis::network_info::plutus_alonzo_cost_models};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{ service::proof_of_work::get_difficulty, model::{datum_submission::{self, accept, reject, get_unconfirmed, DatumSubmission, get_newest_confirmed_datum}, proof_of_work::{self, get_by_time_range}, miner::get_miner_by_pkh, payouts::create_payout}, routes::hashrate::estimate_hashrate, address::pkh_from_address};

use super::block::{Block, KupoUtxo};

#[derive(Debug)]
pub enum SubmissionError {
    DatabaseError(sqlx::Error),
    JsError(JsError),
    ReqwestError(reqwest::Error)
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
    miner_payments: HashMap<String, usize>,  // <Address, Payment>
    hash_rate: f64,
}
#[derive(Deserialize)]
pub struct DenoSubmissionResponse {
    tx_hash: String,
    message: String,
}

pub async fn submit(
    pool: &SqlitePool,
    current_block: &Block,
    sha: &[u8],
    nonce: &[u8]
) -> Result<(), SubmissionError> {
    let new_diff_data = get_difficulty(sha);

    let default_fee: i64 = 25000000;
    let pool_fixed_fee: i64 = std::env::var("POOL_FIXED_FEE")
        .map(|s| s.parse().unwrap_or(default_fee))
        .unwrap_or(default_fee);

    let total_payout = TUNA_PER_DATUM - pool_fixed_fee as usize;

    let maybe_last_paid_datum = get_newest_confirmed_datum(pool).await?;

    let start_time = match maybe_last_paid_datum {
        Some(last_paid_datum) => {
            let maybe_last_confirmed = last_paid_datum.confirmed_at;
            if maybe_last_confirmed.is_some() {
                maybe_last_confirmed.unwrap()
            } else {
                proof_of_work::get_oldest(pool)
                .await
                .unwrap().unwrap().created_at
            }
        },
        None => {
            // if no datum has ever been paid before, start from the first proof of work
            proof_of_work::get_oldest(pool)
                .await
                .unwrap().unwrap().created_at  // invariant upheld by the fact that we have a datum
        }
    };
    
    let end_time = Utc::now().naive_utc();
    
    let proofs = get_by_time_range(pool, None, start_time, end_time).await?;

    let estimated_hashrate_total = estimate_hashrate(&proofs, start_time, end_time);

    let mut miner_proofs: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
    for proof in &proofs {
        miner_proofs.entry(proof.miner_address.clone()).or_insert_with(Vec::new).push(proof.clone());
    }

    let mut miner_payments: HashMap<String, usize> = HashMap::new();
    for (miner_address, proofs) in &miner_proofs { 
        let miner_hashrate = estimate_hashrate(proofs.as_ref(), start_time, end_time);
        let miner_share = miner_hashrate as f64 / estimated_hashrate_total as f64;
        let miner_payment = (total_payout as f64 * miner_share) as usize;
        miner_payments.insert(miner_address.clone(), miner_payment);
    }

    let submission = DenoSubmission {
        nonce: hex::encode(nonce),
        sha: hex::encode(sha),
        current_block: current_block.clone(),
        new_difficulty: new_diff_data.difficulty_number as i64,
        new_zeroes: new_diff_data.leading_zeroes as i64,
        miner_payments: miner_payments.clone(),
        hash_rate: estimated_hashrate_total
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
        pool, response.tx_hash.clone(), hex::encode(sha)
    ).await?;

    for (miner_address, payment) in &miner_payments {
        let Ok(pkh) = pkh_from_address(miner_address) else {
            continue;
        };
        let Some(miner) = get_miner_by_pkh(pool, &pkh).await? else {
            continue;
        };
    
        let mut tx = pool.begin().await?;
    
        create_payout(&mut tx, miner.id, *payment as i64, &response.tx_hash).await?;
    
        tx.commit().await?;
    }

    Ok(())
}

pub async fn submission_updater(pool: SqlitePool) {
    let kupo_url = std::env::var("KUPO_URL").expect("Cannot instantiate BlockService because KUPO_URL is not set.");
    let interval = 60;

    let client = reqwest::Client::new();

    loop {
        
        let unconfirmed_datums = get_unconfirmed(&pool).await;
        
        let Ok(unconfirmed) = unconfirmed_datums else {
            log::error!("Submission updater could not fetch unconfirmed.");
            return;
        };

        if !unconfirmed.is_empty() {
            for datum in unconfirmed {
                let url = format!("{}/matches/*@{}", kupo_url, datum.transaction_hash);

                let resp = client.get(&url).send().await;

                let Ok(r) = resp else {
                    log::warn!("Failed to fetch matches for transaction_id: {}", datum.transaction_hash);
                    continue;
                };
                
                let response_result: Result<Vec<KupoUtxo>, reqwest::Error> = r.json().await;
                let Ok(kupo_utxos) = response_result else {
                    log::error!("Failed to parse kupo transaction! Got {:?}", response_result);
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
                        return;
                    };
                } else {
                    let now = Utc::now().naive_utc();
                    let age = now.signed_duration_since(datum.created_at).num_minutes();

                    if age > 2 {
                        let result = reject(&pool, vec![datum]).await;
                        let Ok(_) = result else {
                            log::error!("Failed to reject datum with transaction_id: {}", tx_hash);
                            return;
                        };
                    }
                }
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
    }
}