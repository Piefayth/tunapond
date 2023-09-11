use sqlx::sqlite::SqlitePool;
use chrono::NaiveDateTime;

pub struct DatumSubmission {
    pub transaction_hash: String,
    pub sha: String,
    pub is_definitely_accepted: bool,
    pub is_definitely_rejected: bool,
    pub created_at: NaiveDateTime,
    pub block_number: i64,  // added this field
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

pub async fn accept(pool: &SqlitePool, submissions: Vec<DatumSubmission>) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    for submission in submissions.iter() {
        sqlx::query!(
            r#"
            UPDATE datum_submissions
            SET is_definitely_accepted = true, is_definitely_rejected = false
            WHERE transaction_hash = ?
            "#,
            submission.transaction_hash
        )
        .execute(tx.as_mut())
        .await?;
    }

    tx.commit().await?;

    Ok(())
}

pub async fn reject(pool: &SqlitePool, submissions: Vec<DatumSubmission>) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    for submission in submissions.iter() {
        sqlx::query!(
            r#"
            UPDATE datum_submissions
            SET is_definitely_accepted = false, is_definitely_rejected = true
            WHERE transaction_hash = ?
            "#,
            submission.transaction_hash
        )
        .execute(tx.as_mut())
        .await?;
    }

    tx.commit().await?;

    Ok(())
}

pub async fn get_unconfirmed(pool: &SqlitePool) -> Result<Vec<DatumSubmission>, sqlx::Error> {
    sqlx::query_as!(
        DatumSubmission,
        r#"
        SELECT ds.transaction_hash, ds.sha, ds.is_definitely_accepted, 
               ds.is_definitely_rejected, ds.created_at, pow.block_number
        FROM datum_submissions AS ds
        JOIN proof_of_work AS pow ON ds.sha = pow.sha
        WHERE ds.is_definitely_accepted = false AND ds.is_definitely_rejected = false
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_confirmed(pool: &SqlitePool) -> Result<Vec<DatumSubmission>, sqlx::Error> {
    sqlx::query_as!(
        DatumSubmission,
        r#"
        SELECT ds.transaction_hash, ds.sha, ds.is_definitely_accepted, 
               ds.is_definitely_rejected, ds.created_at, pow.block_number
        FROM datum_submissions AS ds
        JOIN proof_of_work AS pow ON ds.sha = pow.sha
        WHERE ds.is_definitely_accepted = true OR ds.is_definitely_rejected = true
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_unpaid_datums_oldest(pool: &SqlitePool) -> Result<Vec<DatumSubmission>, sqlx::Error> {
    sqlx::query_as!(
        DatumSubmission,
        r#"
        SELECT ds.transaction_hash, ds.sha, ds.is_definitely_accepted, 
               ds.is_definitely_rejected, ds.created_at, pow.block_number
        FROM datum_submissions AS ds
        JOIN proof_of_work AS pow ON ds.sha = pow.sha
        WHERE ds.paid_at IS NULL
        AND ds.is_definitely_accepted = TRUE
        ORDER BY ds.created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn mark_as_paid(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, submissions: Vec<DatumSubmission>) -> Result<(), sqlx::Error> {
    for submission in submissions.iter() {
        sqlx::query!(
            r#"
            UPDATE datum_submissions
            SET paid_at = datetime('now')
            WHERE transaction_hash = ?
            "#,
            submission.transaction_hash
        )
        .execute(tx.as_mut())
        .await?;
    }

    Ok(())
}