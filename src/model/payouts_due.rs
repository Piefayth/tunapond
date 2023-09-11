use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePool;
use std::result::Result;

// Define the PayoutDue structure to map with the database table.
pub struct PayoutDue {
    pub id: i64,
    pub miner_id: i64,
    pub owed: i64,
    pub is_paid: bool,
    pub created_at: NaiveDateTime,
    pub transaction_hash: Option<String>,
    pub address: String
}

pub async fn create_payout(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,  // <- Use transaction here
    miner_id: i64, 
    owed: i64
) -> Result<u64, sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO payouts_due
        (miner_id, owed, is_paid, created_at)
        VALUES (?, ?, false, datetime('now'))
        "#,
        miner_id,
        owed
    )
    .execute(tx.as_mut())  // <- Execute the query within the transaction
    .await
    .map(|r| r.rows_affected())
}

pub async fn get_unpaid(pool: &SqlitePool) -> Result<Vec<PayoutDue>, sqlx::Error> {
    sqlx::query_as!(
        PayoutDue,
        r#"
        SELECT 
            payouts_due.id, 
            payouts_due.miner_id, 
            payouts_due.owed, 
            payouts_due.is_paid, 
            payouts_due.created_at,
            payouts_due.transaction_hash,
            miners.address
        FROM payouts_due
        JOIN miners ON payouts_due.miner_id = miners.id
        WHERE payouts_due.is_paid = false
        AND payouts_due.transaction_hash IS NULL
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_tentatively_paid(pool: &SqlitePool) -> Result<Vec<PayoutDue>, sqlx::Error> {
    sqlx::query_as!(
        PayoutDue,
        r#"
        SELECT 
            payouts_due.id, 
            payouts_due.miner_id, 
            payouts_due.owed, 
            payouts_due.is_paid, 
            payouts_due.created_at,
            payouts_due.transaction_hash,
            miners.address
        FROM payouts_due
        JOIN miners ON payouts_due.miner_id = miners.id
        WHERE payouts_due.is_paid = false
        AND payouts_due.transaction_hash IS NOT NULL
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_oldest_unverified_payment(pool: &SqlitePool) -> Result<Option<PayoutDue>, sqlx::Error> {
    sqlx::query_as!(
        PayoutDue,
        r#"
        SELECT 
            payouts_due.id, 
            payouts_due.miner_id, 
            payouts_due.owed, 
            payouts_due.is_paid, 
            payouts_due.created_at,
            payouts_due.transaction_hash,
            miners.address
        FROM payouts_due
        JOIN miners ON payouts_due.miner_id = miners.id
        WHERE payouts_due.is_paid = false
        AND payouts_due.transaction_hash IS NOT NULL
        ORDER BY payouts_due.created_at ASC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
}

pub async fn set_tentatively_paid(pool: &SqlitePool, id: i64, tx_hash: &str) -> Result<u64, sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE payouts_due
        SET transaction_hash = ?, transaction_time = datetime('now')
        WHERE id = ?
        "#,
        tx_hash,
        id
    )
    .execute(pool)
    .await
    .map(|r| r.rows_affected())
}

pub async fn mark_as_paid(pool: &SqlitePool, id: i64) -> Result<u64, sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE payouts_due
        SET is_paid = true
        WHERE id = ?
        "#,
        id
    )
    .execute(pool)
    .await
    .map(|r| r.rows_affected())
}

pub async fn reset_transaction_hash(pool: &SqlitePool, id: i64) -> Result<u64, sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE payouts_due
        SET transaction_hash = NULL
        WHERE id = ?
        "#,
        id
    )
    .execute(pool)
    .await
    .map(|r| r.rows_affected())
}
