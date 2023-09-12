use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePool;
use std::result::Result;

pub struct Payout {
    pub id: i64,
    pub datum_transaction_hash: String,
    pub miner_id: i64,
    pub paid_amount: i64,  // renamed 'owed' to 'paid_amount'
    pub created_at: NaiveDateTime,
}

pub async fn create_payout(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    miner_id: i64, 
    paid_amount: i64,
    datum_tx_hash: &str,
) -> Result<u64, sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO payouts
        (miner_id, datum_transaction_hash, paid_amount, created_at)
        VALUES (?, ?, ?, datetime('now'))
        "#,
        miner_id,
        datum_tx_hash,
        paid_amount,
    )
    .execute(tx.as_mut())
    .await
    .map(|r| r.rows_affected())
}

pub async fn get_payouts_for_miner(
    pool: &SqlitePool,
    miner_id: i64,
) -> Result<Vec<Payout>, sqlx::Error> {
    sqlx::query_as!(
        Payout,
        r#"
        SELECT id, datum_transaction_hash, miner_id, paid_amount, created_at
        FROM payouts
        WHERE miner_id = ?
        "#,
        miner_id
    )
    .fetch_all(pool)
    .await
}