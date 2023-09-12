use sqlx::sqlite::SqlitePool;

// Define the Miner structure to map with the database table.
pub struct Miner {
    pub id: i64,
    pub pkh: String,
}

// Function to create a new miner.
pub async fn create_miner(pool: &SqlitePool, pkh: String, address: String) -> Result<i64, sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        r#"
        INSERT INTO miners
        (pkh, address)
        VALUES (?, ?)
        "#,
        pkh, address
    )
    .execute(tx.as_mut())
    .await?;

    let id: (i64,) = sqlx::query_as("SELECT last_insert_rowid()")
        .fetch_one(tx.as_mut())
        .await?;

    tx.commit().await?;

    Ok(id.0)
}
// Optionally: Function to get a miner by its primary key hash (pkh).
pub async fn get_miner_by_pkh(pool: &SqlitePool, pkh: &str) -> Result<Option<Miner>, sqlx::Error> {
    sqlx::query_as!(
        Miner,
        r#"
        SELECT id, pkh
        FROM miners
        WHERE pkh = ?
        "#,
        pkh
    )
    .fetch_optional(pool)
    .await
}