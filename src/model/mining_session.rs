use sqlx::sqlite::SqlitePool;
use chrono::NaiveDateTime;

pub struct NewMiningSession<'a> {
    pub public_key_hash: &'a str,
    pub start_time: NaiveDateTime,
    pub currently_mining_block: i64,
}

pub struct MiningSession {
    pub id: i64,
    pub public_key_hash: String,
    pub start_time: NaiveDateTime,
    pub currently_mining_block: i64,
    pub end_time: Option<NaiveDateTime>,
    pub payment_due: Option<i64>,
    pub payment_transaction: Option<String>,
    pub is_definitely_paid: bool
}

pub async fn create(
    pool: &SqlitePool,
    new_session: NewMiningSession<'_>,
) -> Result<i64, sqlx::Error> {
    let done = sqlx::query!(
        r#"
        INSERT INTO mining_sessions
        VALUES (null, ?, ?, ?, null, null, null, false)
        "#,
        new_session.public_key_hash, new_session.start_time, new_session.currently_mining_block
    )
    .execute(pool)
    .await?;

    Ok(done.last_insert_rowid())
}

pub async fn get_latest(pool: &SqlitePool, public_key_hash: &str) -> Result<Option<MiningSession>, sqlx::Error> {
    sqlx::query_as!(
        MiningSession,
        r#"
        SELECT id, public_key_hash, start_time, currently_mining_block, end_time, payment_due, payment_transaction, is_definitely_paid
        FROM mining_sessions
        WHERE public_key_hash = ?
        ORDER BY start_time DESC
        LIMIT 1
        "#,
        public_key_hash
    )
    .fetch_optional(pool)
    .await
}

pub async fn get_from_id(pool: &SqlitePool, session_id: i64) -> Result<Option<MiningSession>, sqlx::Error> {
    sqlx::query_as!(
        MiningSession,
        r#"
        SELECT id, public_key_hash, start_time, currently_mining_block, end_time, payment_due, payment_transaction, is_definitely_paid
        FROM mining_sessions
        WHERE id = ?
        LIMIT 1
        "#,
        session_id
    )
    .fetch_optional(pool)
    .await
}

pub async fn update_end_time(
    pool: &SqlitePool, 
    public_key_hash: &str, 
    end_time: NaiveDateTime
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE mining_sessions 
        SET end_time = ? 
        WHERE public_key_hash = ?
        "#,
        end_time, public_key_hash
    )
    .execute(pool)
    .await
    .map(|_| ())
}