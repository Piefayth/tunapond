use sqlx::sqlite::SqlitePool;
use chrono::NaiveDateTime;

use crate::routes::submit::SubmissionEntry;

#[derive(Clone)]
pub struct ProofOfWork {
    pub miner_id: i64,
    pub block_number: i64,
    pub sha: String,
    pub nonce: String,
    pub created_at: NaiveDateTime,
}

pub async fn create(
    pool: &SqlitePool,
    miner_id: i64,
    block_number: i64,
    new_pows: &Vec<&SubmissionEntry>,
) -> Result<u64, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mut success_count = 0;

    for new_pow in new_pows.iter() {
        match sqlx::query!(
            r#"
            INSERT INTO proof_of_work
            (miner_id, block_number, sha, nonce, created_at)
            VALUES (?, ?, ?, ?, datetime('now'))
            "#,
            miner_id, block_number, new_pow.sha, new_pow.nonce
        )
        .execute(tx.as_mut())
        .await {
            Ok(_) => success_count += 1,
            Err(e) => {
                log::warn!("Rejected a sha due to {:?}", e);
            }
        }
    }

    tx.commit().await?;

    Ok(success_count) // Returns the number of successful insertions.
}

pub async fn get_by_time_range(
    pool: &SqlitePool,
    miner_id: Option<i64>,
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
) -> Result<Vec<ProofOfWork>, sqlx::Error> {
    match miner_id {
        Some(id) => {
            sqlx::query_as!(
                ProofOfWork,
                r#"
                SELECT miner_id, block_number, sha, nonce, created_at
                FROM proof_of_work
                WHERE miner_id = ? AND created_at BETWEEN ? AND ?
                "#,
                id, start_time, end_time
            )
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as!(
                ProofOfWork,
                r#"
                SELECT miner_id, block_number, sha, nonce, created_at
                FROM proof_of_work
                WHERE created_at BETWEEN ? AND ?
                "#,
                start_time, end_time
            )
            .fetch_all(pool)
            .await
        }
    }
}

pub async fn get(
    pool: &SqlitePool,
    miner_id: i64,
) -> Result<Vec<ProofOfWork>, sqlx::Error> {
    sqlx::query_as!(
        ProofOfWork,
        r#"
        SELECT miner_id, block_number, sha, nonce, created_at
        FROM proof_of_work
        WHERE miner_id = ?
        "#,
        miner_id
    )
    .fetch_all(pool)
    .await
}
