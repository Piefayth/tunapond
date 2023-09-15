use sqlx::sqlite::SqlitePool;
use chrono::NaiveDateTime;

pub struct DatumSubmission {
    pub transaction_hash: String,
    pub sha: String,
    pub created_at: NaiveDateTime,
    pub rejected: bool,
    pub block_number: i64,
    pub confirmed_in_slot: Option<i64>,
    pub confirmed_at: Option<NaiveDateTime>
}

pub async fn create(
    pool: &SqlitePool,
    transaction_hash: String,
    sha: String,
    block_number: i64,
) -> Result<u64, sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO datum_submissions
        (transaction_hash, sha, block_number, created_at, rejected)
        VALUES (?, ?, ?, datetime('now'), FALSE)
        "#,
        transaction_hash, sha, block_number
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
            SET rejected = false, confirmed_in_slot = ?, confirmed_at = datetime('now')
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
               ds.rejected, ds.created_at, ds.confirmed_at, pow.block_number
        FROM datum_submissions AS ds
        JOIN proof_of_work AS pow ON ds.sha = pow.sha
        WHERE ds.confirmed_in_slot IS NULL AND ds.rejected = false
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_newest_confirmed_datum(pool: &SqlitePool) -> Result<Option<DatumSubmission>, sqlx::Error> {
    sqlx::query_as!(
        DatumSubmission,
        r#"
        SELECT ds.transaction_hash, ds.sha, ds.confirmed_in_slot,
               ds.rejected, ds.created_at, ds.confirmed_at, pow.block_number
        FROM datum_submissions AS ds
        JOIN proof_of_work AS pow ON ds.sha = pow.sha
        ORDER BY confirmed_in_slot DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
}
