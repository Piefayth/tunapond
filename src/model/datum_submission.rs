use sqlx::sqlite::SqlitePool;
use chrono::NaiveDateTime;

pub struct DatumSubmission {
    pub transaction_hash: String,
    pub sha: String,
    pub created_at: NaiveDateTime,
    pub rejected: bool,
    pub block_number: i64,
    pub paid_at: Option<NaiveDateTime>,
    pub confirmed_in_slot: Option<i64>,
}

pub async fn create(
    pool: &SqlitePool,
    transaction_hash: String,
    sha: String,
) -> Result<u64, sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO datum_submissions
        (transaction_hash, sha, created_at, rejected)
        VALUES (?, ?, datetime('now'), FALSE)
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
        if submission.confirmed_in_slot.is_none() {
            log::warn!("Tried to confirm a datum, but did not provide a slot within which the datum was confirmed.");
            continue;
        }

        sqlx::query!(
            r#"
            UPDATE datum_submissions
            SET rejected = false, confirmed_in_slot = ?
            WHERE transaction_hash = ?
            "#,
            submission.confirmed_in_slot, submission.transaction_hash
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
        if submission.confirmed_in_slot.is_none() {
            continue;
        };

        sqlx::query!(
            r#"
            UPDATE datum_submissions
            SET rejected = true
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
        SELECT ds.transaction_hash, ds.sha, ds.confirmed_in_slot,
               ds.rejected, ds.created_at, ds.paid_at, pow.block_number
        FROM datum_submissions AS ds
        JOIN proof_of_work AS pow ON ds.sha = pow.sha
        WHERE ds.confirmed_in_slot IS NULL AND ds.rejected = false
        "#,
    )
    .fetch_all(pool)
    .await
}

// Returns only datums that are READY for payment, i.e. have been seen on chain.
pub async fn get_unpaid_datums_oldest(pool: &SqlitePool) -> Result<Vec<DatumSubmission>, sqlx::Error> {
    sqlx::query_as!(
        DatumSubmission,
        r#"
        SELECT ds.transaction_hash, ds.sha,  ds.confirmed_in_slot,
               ds.rejected, ds.created_at, ds.paid_at, pow.block_number
        FROM datum_submissions AS ds
        JOIN proof_of_work AS pow ON ds.sha = pow.sha
        WHERE ds.paid_at IS NULL
        AND ds.confirmed_in_slot IS NOT NULL
        AND ds.rejected = FALSE
        ORDER BY ds.created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_unpaid_datums_after_time(pool: &SqlitePool, after_time: NaiveDateTime) -> Result<Vec<DatumSubmission>, sqlx::Error> {
    sqlx::query_as!(
        DatumSubmission,
        r#"
        SELECT ds.transaction_hash, ds.sha, ds.confirmed_in_slot,
               ds.rejected, ds.created_at, ds.paid_at, pow.block_number
        FROM datum_submissions AS ds
        JOIN proof_of_work AS pow ON ds.sha = pow.sha
        WHERE ds.paid_at IS NULL
        AND ds.confirmed_in_slot IS NOT NULL
        AND ds.rejected = FALSE
        AND ds.paid_at > ?
        ORDER BY ds.created_at ASC
        "#,
        after_time
    )
    .fetch_all(pool)
    .await
}

pub async fn get_newest_paid_datum(pool: &SqlitePool) -> Result<Option<DatumSubmission>, sqlx::Error> {
    sqlx::query_as!(
        DatumSubmission,
        r#"
        SELECT ds.transaction_hash, ds.sha, ds.confirmed_in_slot,
               ds.rejected, ds.created_at, ds.paid_at, pow.block_number
        FROM datum_submissions AS ds
        JOIN proof_of_work AS pow ON ds.sha = pow.sha
        WHERE ds.paid_at IS NOT NULL
        ORDER BY ds.paid_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
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