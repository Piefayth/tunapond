use sqlx::sqlite::SqlitePool;
use chrono::NaiveDateTime;

pub struct DatumSubmission {
    pub transaction_hash: String,
    pub sha: String,
    pub is_definitely_accepted: bool,
    pub is_definitely_rejected: bool,
    pub created_at: NaiveDateTime,
}

pub async fn create(
    pool: &SqlitePool,
    transaction_hash: String,
    sha: String,
) -> Result<u64, sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO datum_submissions
        (transaction_hash, sha, is_definitely_accepted, is_definitely_rejected, created_at)
        VALUES (?, ?, false, false, datetime('now'))
        "#,
        transaction_hash, sha
    )
    .execute(pool)
    .await
    .map(|r| r.rows_affected())
}

pub async fn get_unconfirmed(pool: &SqlitePool) -> Result<Vec<DatumSubmission>, sqlx::Error> {
    sqlx::query_as!(
        DatumSubmission,
        r#"
        SELECT transaction_hash, sha, is_definitely_accepted, is_definitely_rejected, created_at
        FROM datum_submissions
        WHERE is_definitely_accepted = false AND is_definitely_rejected = false
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_confirmed(pool: &SqlitePool) -> Result<Vec<DatumSubmission>, sqlx::Error> {
    sqlx::query_as!(
        DatumSubmission,
        r#"
        SELECT transaction_hash, sha, is_definitely_accepted, is_definitely_rejected, created_at
        FROM datum_submissions
        WHERE is_definitely_accepted = true OR is_definitely_rejected = true
        "#,
    )
    .fetch_all(pool)
    .await
}