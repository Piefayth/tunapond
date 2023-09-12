
use std::collections::HashMap;

use cardano_multiplatform_lib::{address::Address, chain_crypto::PublicKey, crypto::VRFVKey};
use chrono::{Utc, Duration};
use serde::{Serialize, Deserialize};
use sqlx::SqlitePool;

use crate::{model::{datum_submission::{get_unpaid_datums_oldest, self, get_newest_paid_datum}, proof_of_work::{get_by_time_range, self}, payouts_due::{create_payout, get_unpaid, set_tentatively_paid, PayoutDue, get_tentatively_paid, mark_as_paid, reset_transaction, get_oldest_unverified_payment}}, routes::hashrate::estimate_hashrate, address};

use super::block::KupoUtxo;


const TUNA_PER_DATUM: usize = 5_000_000_000;

// Calculates what miners should be paid and writes that to the DB.
// Does not initiate Cardano transactions.
pub async fn update_payouts(
    pool: &SqlitePool,
) {
    let default_fee: i64 = 25000000;
    let pool_fixed_fee: i64 = std::env::var("POOL_FIXED_FEE")
        .map(|s| s.parse().unwrap_or(default_fee))
        .unwrap_or(default_fee);

    let Ok(maybe_last_paid_datum) = get_newest_paid_datum(pool).await else {
        log::error!("Could not access newest paid datums in payout updater.");
        return ();
    };

    let Ok(unpaid_datums) = get_unpaid_datums_oldest(pool).await else {
        log::error!("Could not access unpaid datums in payout updater.");
        return ();
    };

    if unpaid_datums.len() == 0 {
        log::info!("No payouts need created.");
        return ();
    };

    let oldest_unpaid_time = unpaid_datums[0].created_at;

    let start_time = match maybe_last_paid_datum {
        Some(last_paid_datum) => {
            let newest_paid_time = last_paid_datum.paid_at.unwrap(); // guaranteed by the underlying database query
            std::cmp::min(newest_paid_time, oldest_unpaid_time) // be 100% sure we aren't missing any unpaid datums
        },
        None => {
            // if no datum has ever been paid before, start from the first proof of work
            proof_of_work::get_oldest(pool)
                .await
                .unwrap().unwrap().created_at  // invariant upheld by the fact that we have a datum
        }
    };
    
    let end_time = Utc::now().naive_utc();

    let Ok(proofs) = get_by_time_range(pool, None, start_time, end_time).await else {
        log::error!("Could not access proofs in payout updater.");
        return ();
    };

    let estimated_hashrate_total = estimate_hashrate(&proofs, start_time, end_time);
    log::info!("Payout estimated pool hashrate at {}", estimated_hashrate_total);

    let payout_amount = (unpaid_datums.len() * TUNA_PER_DATUM) - (unpaid_datums.len() * pool_fixed_fee as usize);
    log::info!("Creating payout obligation for $TUNA: {}", payout_amount);

    // Payments use a Pay Per Last N Shares model
        // Get the average hashrate for each miner in the time window, 
        // {ay them a proportion of the datums mined during that window.
    let mut miner_proofs: std::collections::HashMap<i64, Vec<_>> = std::collections::HashMap::new();
    for proof in &proofs {
        miner_proofs.entry(proof.miner_id).or_insert_with(Vec::new).push(proof.clone());
    }

    let mut miner_shares: std::collections::HashMap<i64, f64> = std::collections::HashMap::new();
    for (miner_id, proofs) in &miner_proofs {
        let miner_hashrate = estimate_hashrate(proofs.as_ref(), start_time, end_time);
        let miner_share = miner_hashrate as f64 / estimated_hashrate_total as f64;
        miner_shares.insert(*miner_id, miner_share);
    }

    let individual_payout = TUNA_PER_DATUM - pool_fixed_fee as usize;

    let Ok(mut tx) = pool.begin().await else {
        log::error!("Failed to begin payout transaction.");
        return ();
    };

    for datum in &unpaid_datums {
        for (&miner_id, &miner_share) in &miner_shares {
            let miner_payout_for_datum = (individual_payout as f64 * miner_share) as usize;
            log::info!(
                "Creating individual payout for miner_id {}: {}. Their hashrate share is {}",
                miner_id,
                miner_payout_for_datum,
                miner_share
            );
            
            let Ok(_) = create_payout(&mut tx, miner_id, miner_payout_for_datum as i64, &datum.transaction_hash).await else {
                log::error!("Failed to create payout for miner.");
                return ();
            };
        }
    }
    
    let Ok(_) = datum_submission::mark_as_paid(&mut tx, unpaid_datums).await else {
        log::error!("Failed to mark datums as paid.");
        return ();
    };
    
    let Ok(_) = tx.commit().await else {
        log::error!("Failed to commit payout transaction.");
        return ();
    };
}

// PAYOUT_UPDATE_INTERVAL is effectively a reward buffer.
// If no datums are mined during a period, the hashrate offered during that period will not be compensated.
// Use a larger value to make it more likely that short duration mining activity will be paid out.
// Use a smaller value to allow miners to ramp up to their "true" hash rate more quickly.
// TODO: Make this an env var.
// Or use the existing env var?

pub async fn payout_updater(pool: SqlitePool) {
    let default_interval = 300;
    let payout_update_interval: u64 = std::env::var("PAYOUT_UPDATE_INTERVAL")
        .unwrap_or_else(|_| default_interval.to_string())
        .parse()
        .unwrap_or(default_interval);

    loop {
        log::info!("Trying to update payouts");
        let _ = update_payouts(&pool).await;
        let _ = payment_updater(&pool).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(payout_update_interval)).await;
    }
}

#[derive(Debug, Serialize)]
pub struct DenoPayment {
    datum_transaction_hashes: Vec<String>,
    payments: HashMap<String, u64> // <Address, Amount>
}

#[derive(Deserialize)]
pub struct DenoPaymentResponse {
    tx_hash: String,
    message: String,
}

pub async fn create_payments(pool: &SqlitePool) {
    let Ok(unpaid_payouts) = get_unpaid(&pool).await else {
        log::error!("Could not get due payments from database.");
        return ();
    };

    if unpaid_payouts.len() == 0 {
        log::info!("Wanted to update payments, but no payouts are unfulfilled.");
        return ();
    }

    let mut all_datum_hashes: Vec<String> = Vec::new();
    let mut payments: HashMap<String, u64> = HashMap::new();

    for payout in unpaid_payouts.iter() {
        // Collect all datum_transaction_hashes
        if !all_datum_hashes.contains(&payout.datum_transaction_hash) {
            all_datum_hashes.push(payout.datum_transaction_hash.clone());
        }
        
        // Sum payments for each address
        *payments.entry(payout.address.clone()).or_insert(0u64) += payout.owed as u64;
    }

    log::debug!("Attempting to pay for datums {:?} with payments: {:?}", all_datum_hashes, payments);

    let deno_payment = DenoPayment {
        datum_transaction_hashes: all_datum_hashes,
        payments,
    };

    let response = reqwest::Client::new()
        .post("http://localhost:22123/payment")
        .json(&deno_payment)
        .send()
        .await;

    let parsed_response = match response {
        Ok(resp) => resp.json::<DenoPaymentResponse>().await,
        Err(_) => {
            log::error!("Failed to submit payment.");
            return;
        }
    };

    match parsed_response {
        Ok(resp) => {
            for payout in unpaid_payouts {
                // TODO: These should be in a transaction
                let Ok(_) = set_tentatively_paid(&pool, payout.id, &resp.tx_hash).await else {
                    log::error!("Failed to update the transaction hash for payout id {}", payout.id);
                    continue;
                };
            }
            log::info!("Submitted a payment for all datums, tx hash: {}", resp.tx_hash)
        },
        Err(_) => {
            log::error!("Failed to parse the response.");
        }
    }
}

async fn payment_updater(pool: &SqlitePool) {
    if std::env::var("DISABLE_PAYMENTS").is_ok() {
        log::warn!("Payments have been disabled. Skipping payment.");
        return
    }

    log::info!("Trying to create payments");

    let oldest_unverified = get_oldest_unverified_payment(&pool).await;

    match oldest_unverified {
        Ok(Some(_)) => {
            // There is an unverified payment. Skip for now.
            log::info!("Not creating any new payments. Last payment yet to be verified.");
        }
        Ok(None) => {
            // Not waiting on any old payments, can make more
            let _ = create_payments(&pool).await;
        }
        Err(e) => {
            log::error!("Failed to fetch oldest unverified payment: {}", e);
        }
    }
}

pub async fn payment_verifier(pool: SqlitePool) {
    let kupo_url = std::env::var("KUPO_URL").expect("KUPO_URL is not set.");
    // TODO: Configurable verifier interval
    const PAYMENT_VERIFIER_UPDATE_INTERVAL_SECONDS: u64 = 5;
    const MINUTES_UNTIL_TRANSACTION_INVALID: i64 = 2; 
    let client = reqwest::Client::new();

    loop {
        let tentatively_paid = get_tentatively_paid(&pool).await;
        let Ok(tentative_payments) = tentatively_paid else {
            log::error!("Payment updater could not fetch tentatively paid payments.");
            continue;
        };

        for payment in tentative_payments {
            let tx_hash = payment.transaction_hash.clone().unwrap();
            let url = format!("{}/matches/*@{}", kupo_url, payment.transaction_hash.unwrap()); // We know transaction_hash is Some because of the query.

            let resp = client.get(&url).send().await;


            let Ok(r) = resp else {
                log::warn!("Failed to fetch matches for transaction_id: {}", tx_hash);               
                continue;
            };
            
            let response_result: Result<Vec<KupoUtxo>, reqwest::Error> = r.json().await;
            let Ok(kupo_transactions) = response_result else {
                log::error!("Failed to parse kupo transaction! Got {:?}", response_result);
                continue;
            };
            // TODO: One miner could've gotten paid many times in a single transaction
            // We only need to check kupo for each transaction once
            if !kupo_transactions.is_empty() {
                let paid_result = mark_as_paid(&pool, payment.id).await;
                log::info!("Finalized payment transaction for miner {} with transaction hash {}.", payment.miner_id, tx_hash);

                let Ok(_) = paid_result else {
                    log::error!("Error marking payment with ID {} as paid", payment.id);
                    continue;
                };
            } else {
                let now = Utc::now().naive_utc();
                let transaction_time = payment.transaction_time.unwrap();
                let age = now.signed_duration_since(transaction_time).num_minutes();
    
                if age > MINUTES_UNTIL_TRANSACTION_INVALID {
                    let reset_result = reset_transaction(&pool, payment.id).await;
                    let Ok(_) = reset_result else {
                        log::error!("Error resetting transaction hash for payment with ID {}", payment.id);
                        continue;
                    };
                }
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(PAYMENT_VERIFIER_UPDATE_INTERVAL_SECONDS)).await;
    }
}
