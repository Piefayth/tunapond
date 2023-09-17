use chrono::NaiveDateTime;
use sqlx::{Postgres, Pool};

use crate::{service::proof_of_work::ProcessedSubmissionEntry};

#[derive(Clone)]
pub struct ProofOfWork {
    pub miner_id: i32,
    pub miner_address: String,
    pub block_number: i32,
    pub sha: String,
    pub nonce: String,
    pub created_at: NaiveDateTime,
}

pub async fn create(
    pool: &Pool<Postgres>,
    miner_id: i32,
    block_number: i32,
    new_pows: &Vec<ProcessedSubmissionEntry>,
) -> Result<u64, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mut success_count = 0;

    for new_pow in new_pows.iter() {
        let hex_sha = hex::encode(new_pow.sha);
        let hex_nonce = hex::encode(new_pow.nonce);

        match sqlx::query!(
            r#"
            INSERT INTO proof_of_work
            (miner_id, block_number, sha, nonce, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            "#,
            miner_id, block_number, hex_sha, hex_nonce
        )
        .execute(&mut tx)
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
    pool: &Pool<Postgres>,
    miner_id: Option<i32>,
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
) -> Result<Vec<ProofOfWork>, sqlx::Error> {
    match miner_id {
        Some(id) => {
            sqlx::query_as!(
                ProofOfWork,
                r#"
                SELECT miner_id, miners.address as miner_address, block_number, sha, nonce, created_at
                FROM proof_of_work
                JOIN miners on miner_id = miners.id
                WHERE miner_id = $1 AND created_at BETWEEN $2 AND $3
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
                SELECT miner_id, miners.address as miner_address, block_number, sha, nonce, created_at
                FROM proof_of_work
                JOIN miners on miner_id = miners.id
                WHERE created_at BETWEEN $1 AND $2
                "#,
                start_time, end_time
            )
            .fetch_all(pool)
            .await
        }
    }
}

pub async fn get_oldest(pool: &Pool<Postgres>) -> Result<Option<ProofOfWork>, sqlx::Error> {
    sqlx::query_as!(
        ProofOfWork,
        r#"
        SELECT miner_id, miners.address as miner_address, block_number, sha, nonce, created_at
        FROM proof_of_work
        JOIN miners on miner_id = miners.id
        ORDER BY created_at ASC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
}

pub async fn cleanup_old_proofs(pool: &Pool<Postgres>, num_to_retain: i64) -> Result<(), sqlx::Error> {
    let offset = num_to_retain - 1;
    let date_of_nth_oldest_confirmed_datum_result = sqlx::query!(
        r#"
        SELECT confirmed_at
        FROM datum_submissions
        WHERE confirmed_at IS NOT NULL
        ORDER BY confirmed_at DESC
        LIMIT 1 OFFSET $1
        "#,
        offset
    )
    .fetch_optional(pool)
    .await?;

    if let Some(row) = date_of_nth_oldest_confirmed_datum_result {
        let oldest_datum_date = row.confirmed_at;
        
        sqlx::query!(
            r#"
            DELETE FROM proof_of_work
            WHERE created_at < $1
            AND (sha, block_number) NOT IN (
                SELECT sha, block_number
                FROM datum_submissions
            )
            "#,
            oldest_datum_date
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}