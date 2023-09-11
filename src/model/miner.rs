use sqlx::sqlite::SqlitePool;

// Define the Miner structure to map with the database table.
pub struct Miner {
    pub id: i64,
    pub pkh: String,
}

// Function to create a new miner.
pub async fn create_miner(pool: &SqlitePool, pkh: String, address: String) -> Result<u64, sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO miners
        (pkh, address)
        VALUES (?, ?)
        "#,
        pkh, address
    )
    .execute(pool)
    .await
    .map(|r| r.rows_affected())
}

// Function to get all miners.
pub async fn get_miners(pool: &SqlitePool) -> Result<Vec<Miner>, sqlx::Error> {
    sqlx::query_as!(
        Miner,
        r#"
        SELECT id, pkh
        FROM miners
        "#,
    )
    .fetch_all(pool)
    .await
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