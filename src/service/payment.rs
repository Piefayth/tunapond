
use cardano_multiplatform_lib::{address::Address, chain_crypto::PublicKey, crypto::VRFVKey};
use chrono::Utc;
use serde::{Serialize, Deserialize};
use sqlx::SqlitePool;

use crate::{model::{datum_submission::{get_unpaid_datums_oldest, self}, proof_of_work::get_by_time_range, payouts_due::{create_payout, get_unpaid, set_tentatively_paid, PayoutDue, get_tentatively_paid, mark_as_paid, reset_transaction_hash, get_oldest_unverified_payment}}, routes::hashrate::estimate_hashrate, address};

use super::block::KupoTransaction;

const PAYOUT_PERIOD_MS: i64 = 120_000; // TODO: use env var, 2 minutes defualt for now, too fast
const TUNA_PER_DATUM: usize = 5_000_000_000;

pub async fn update_payouts(
    pool: &SqlitePool,
) {
    let default_fee: i64 = 25000000;
    let pool_fixed_fee: i64 = std::env::var("POOL_FIXED_FEE")
        .map(|s| s.parse().unwrap_or(default_fee))
        .unwrap_or(default_fee);

    let Ok(unpaid_datums) = get_unpaid_datums_oldest(pool).await else {
        log::error!("Could not access unpaid datums in payout updater.");
        return ();
    };

    if unpaid_datums.len() == 0 {
        log::info!("No payouts need created.");
        return ();
    };

    let start_time = unpaid_datums.first().unwrap().created_at;
    let end_time = Utc::now().naive_utc();
    if (start_time.timestamp_millis() - end_time.timestamp_millis()).abs() < PAYOUT_PERIOD_MS {
        log::info!("Last payout update was too recent.");
        return ();
    }

    let Ok(proofs) = get_by_time_range(pool, None, start_time, end_time).await else {
        log::error!("Could not access proofs in payout updater.");
        return ();
    };

    let estimated_hashrate_total = estimate_hashrate(&proofs, start_time, end_time);
    log::info!("Payout estimated pool hashrate at {}", estimated_hashrate_total);

    let payout_amount = (unpaid_datums.len() * TUNA_PER_DATUM) - (unpaid_datums.len() * pool_fixed_fee as usize);
    log::info!("Creating payout obligation for $TUNA: {}", payout_amount);

    let mut miner_proofs: std::collections::HashMap<i64, Vec<_>> = std::collections::HashMap::new();
    for proof in &proofs {
        miner_proofs.entry(proof.miner_id).or_insert_with(Vec::new).push(proof.clone());
    }
    
    let Ok(mut tx) = pool.begin().await else {
        log::error!("Failed to begin payout transaction.");
        return ();
    };
    
    for (miner_id, proofs) in &miner_proofs {
        let miner_hashrate = estimate_hashrate(proofs.as_ref(), start_time, end_time);
        let miner_share = miner_hashrate as f64 / estimated_hashrate_total as f64;
        let miner_payout = (payout_amount as f64 * miner_share) as usize;
        log::info!("Estimated payout for miner_id {}: {}. Their hashrate for the payment period was {}", miner_id, miner_payout, miner_hashrate);
        // Create a new payout entry for the miner.
        let Ok(_) = create_payout(&mut tx, *miner_id, miner_payout as i64).await else {
            log::error!("Failed to create payout for miner.");
            return ();
        };
    }
    
    // Mark these datums paid.
    let Ok(_) = datum_submission::mark_as_paid(&mut tx, unpaid_datums).await else {
        log::error!("Failed to mark datums as paid.");
        return ();
    };
    
    let Ok(_) = tx.commit().await else {
        log::error!("Failed to commit payout transaction.");
        return ();
    };
}

// Calculates what miners should be paid and writes that to the DB.
// Does not initiate transactions.
pub async fn payout_updater(pool: SqlitePool) {
    let default_interval = 300;
    let payout_update_interval: u64 = std::env::var("PAYOUT_UPDATE_INTERVAL")
        .unwrap_or_else(|_| default_interval.to_string())
        .parse()
        .unwrap_or(default_interval);

    loop {
        log::info!("Trying to update payouts");
        let _ = update_payouts(&pool).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(payout_update_interval)).await;
    }
}

#[derive(Debug, Serialize)]
pub struct DenoPayment {
    address: String,
    amount: u64
}

#[derive(Deserialize)]
pub struct DenoPaymentResponse {
    tx_hash: String,
    message: String,
}

// TODO: Somehow split our TUNA utxos so there is no spending contention
pub async fn create_payments(pool: &SqlitePool) {
    let Ok(unpaid_payouts) = get_unpaid(&pool).await else {
        log::error!("Could not get due payments from database.");
        return ();
    };

    if unpaid_payouts.len() == 0 {
        log::info!("Wanted to update payments, but no payouts are unfulfilled.");
        return ();
    }

    // Group unpaid payouts by miner's address
    let mut grouped_payouts: std::collections::HashMap<String, Vec<PayoutDue>> = std::collections::HashMap::new();
    for payout in unpaid_payouts {
        grouped_payouts.entry(payout.address.clone()).or_insert_with(Vec::new).push(payout);
    }

    // Take only the first miner (since we want to pay only one miner at a time)
    if let Some((address, payouts_for_miner)) = grouped_payouts.iter().next() {
        let total_amount: u64 = payouts_for_miner.iter().map(|p| p.owed as u64).sum();

        log::info!("Attempting to pay miner {} with amount {}", address, total_amount);

        let payment = DenoPayment {
            address: address.clone(),
            amount: total_amount,
        };

        let response = reqwest::Client::new()
            .post("http://localhost:22123/payment")
            .json(&payment)
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
                for payout in payouts_for_miner {
                    let Ok(_) = set_tentatively_paid(&pool, payout.id, &resp.tx_hash).await else {
                        log::error!("Failed to update the transaction hash for payout id {}", payout.id);
                        continue;
                    };
                }
                log::info!("Submitted a payment for miner {}, tx hash: {}", address, resp.tx_hash)
            },
            Err(_) => {
                log::error!("Failed to parse the response.");
            }
        }
    }
}

pub async fn payment_updater(pool: SqlitePool) {
    let default_interval = 60; // one minute
    let payment_update_interval: u64 = std::env::var("PAYMENT_UPDATE_INTERVAL")
        .unwrap_or_else(|_| default_interval.to_string())
        .parse()
        .unwrap_or(default_interval);

    log::info!("Trying to create payments");

    loop {
        let oldest_unverified = get_oldest_unverified_payment(&pool).await;

        match oldest_unverified {
            Ok(Some(_)) => {
                // There is an unverified payment. Skip for now.
                log::info!("Not creating any new payments. Last payment yet to be verified.");
            }
            Ok(None) => {
                // There are no unverified payments. Continue to create payments.
            }
            Err(e) => {
                log::error!("Failed to fetch oldest unverified payment: {}", e);
            }
        }


        let _ = create_payments(&pool).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(payment_update_interval)).await;
    }
}

pub async fn payment_verifier(pool: SqlitePool) {
    let kupo_url = std::env::var("KUPO_URL").expect("KUPO_URL is not set.");
    // TODO: Configurable verifier interval
    let interval = 5;
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
            
            let response_result: Result<Vec<KupoTransaction>, reqwest::Error> = r.json().await;
            let Ok(kupo_transactions) = response_result else {
                log::error!("Failed to parse kupo transaction! Got {:?}", response_result);
                continue;
            };

            if !kupo_transactions.is_empty() {
                let paid_result = mark_as_paid(&pool, payment.id).await;
                log::info!("Finalized payment transaction for miner {} with transaction hash {}.", payment.miner_id, tx_hash);

                let Ok(_) = paid_result else {
                    log::error!("Error marking payment with ID {} as paid", payment.id);
                    continue;
                };
            } else {
                let now = Utc::now().naive_utc();
                let age = now.signed_duration_since(payment.created_at).num_minutes();
    
                if age > 5 {
                    let reset_result = reset_transaction_hash(&pool, payment.id).await;
                    let Ok(_) = reset_result else {
                        log::error!("Error resetting transaction hash for payment with ID {}", payment.id);
                        continue;
                    };
                }
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
    }
}
