use sqlx::{Postgres, Pool};
use chrono::NaiveDateTime;

pub struct DatumSubmission {
    pub transaction_hash: String,
    pub sha: String,
    pub created_at: NaiveDateTime,
    pub rejected: bool,
    pub block_number: i32,
    pub confirmed_in_slot: Option<i32>,
    pub confirmed_at: Option<NaiveDateTime>
}

pub async fn create(
    pool: &Pool<Postgres>,
    transaction_hash: String,
    sha: String,
    block_number: i32,
) -> Result<u64, sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO datum_submissions
        (transaction_hash, sha, block_number, created_at, rejected)
        VALUES ($1, $2, $3, NOW(), FALSE)
        "#,
        transaction_hash, sha, block_number
    )
    .execute(pool)
    .await
    .map(|r| r.rows_affected())
}

pub async fn accept(pool: &Pool<Postgres>, submissions: Vec<DatumSubmission>) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    for submission in submissions.iter() {
        if submission.confirmed_in_slot.is_none() {
            log::warn!("Tried to confirm a datum, but did not provide a slot within which the datum was confirmed.");
            continue;
        }

        sqlx::query!(
            r#"
            UPDATE datum_submissions
            SET rejected = false, confirmed_in_slot = $1, confirmed_at = NOW()
            WHERE transaction_hash = $2
            "#,
            submission.confirmed_in_slot, submission.transaction_hash
        )
        .execute(&mut tx)
        .await?;
    }

    tx.commit().await?;

    Ok(())
}

pub async fn reject(pool: &Pool<Postgres>, submissions: Vec<DatumSubmission>) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    for submission in submissions.iter() {
        if submission.confirmed_in_slot.is_none() {
            continue;
        };

        sqlx::query!(
            r#"
            UPDATE datum_submissions
            SET rejected = true
            WHERE transaction_hash = $1
            "#,
            submission.transaction_hash
        )
        .execute(&mut tx)
        .await?;
    }

    tx.commit().await?;

    Ok(())
}

pub async fn get_unconfirmed(pool: &Pool<Postgres>) -> Result<Vec<DatumSubmission>, sqlx::Error> {
    sqlx::query_as!(
        DatumSubmission,
        r#"
        SELECT ds.transaction_hash, ds.sha, ds.created_at, ds.rejected, pow.block_number, 
               ds.confirmed_in_slot, ds.confirmed_at
        FROM datum_submissions AS ds
        JOIN proof_of_work AS pow ON ds.sha = pow.sha
        WHERE ds.confirmed_in_slot IS NULL AND ds.rejected = false
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_newest_confirmed_datum(pool: &Pool<Postgres>) -> Result<Option<DatumSubmission>, sqlx::Error> {
    sqlx::query_as!(
        DatumSubmission,
        r#"
        SELECT ds.transaction_hash, ds.sha, ds.confirmed_in_slot,
               ds.rejected, ds.created_at, ds.confirmed_at, pow.block_number
        FROM datum_submissions AS ds
        JOIN proof_of_work AS pow ON ds.sha = pow.sha
        WHERE confirmed_in_slot IS NOT NULL
        ORDER BY confirmed_in_slot DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
}
