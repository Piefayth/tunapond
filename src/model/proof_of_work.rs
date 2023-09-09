use sqlx::sqlite::SqlitePool;
use chrono::NaiveDateTime;

use crate::routes::submit::SubmissionEntry;

pub struct ProofOfWork {
    pub mining_session_id: i64,
    pub sha: String,
    pub nonce: String,
    pub created_at: NaiveDateTime,
}

pub async fn create(
    pool: &SqlitePool,
    session_id: i64,
    new_pows: &Vec<SubmissionEntry>,
) -> Result<u64, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mut success_count = 0;

    for new_pow in new_pows.iter() {
        match sqlx::query!(
            r#"
            INSERT INTO proof_of_work
            (mining_session_id, sha, nonce, created_at)
            VALUES (?, ?, ?, datetime('now'))
            "#,
            session_id, new_pow.sha, new_pow.nonce
        )
        .execute(tx.as_mut())
        .await {
            Ok(_) => success_count += 1,
            Err(e) => {
                log::debug!("Rejected a sha due to {:?}", e);
            }
        }
    }

    // Commit the transaction regardless of individual failures.
    tx.commit().await?;

    Ok(success_count) // Returns the number of successful insertions.
}

pub async fn get(
    pool: &SqlitePool,
    mining_session_id: i64,
) -> Result<Vec<ProofOfWork>, sqlx::Error> {
    sqlx::query_as!(
        ProofOfWork,
        r#"
        SELECT mining_session_id, sha, nonce, created_at
        FROM proof_of_work
        WHERE mining_session_id = ?
        "#,
        mining_session_id
    )
    .fetch_all(pool)
    .await
}
