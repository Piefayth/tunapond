use sqlx::{Postgres, Pool};

// Define the Miner structure to map with the database table.
pub struct Miner {
    pub id: i32,
    pub address: String,
    pub pkh: String,
}

// Function to create a new miner.
pub async fn create_miner(pool: &Pool<Postgres>, pkh: String, address: String) -> Result<i32, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let rec = sqlx::query!(
        r#"
        INSERT INTO miners
        (pkh, address)
        VALUES ($1, $2)
        RETURNING id
        "#,
        pkh, address
    )
    .fetch_one(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(rec.id)
}

// Optionally: Function to get a miner by its primary key hash (pkh).
pub async fn get_miner_by_pkh(pool: &Pool<Postgres>, pkh: &str) -> Result<Option<Miner>, sqlx::Error> {
    sqlx::query_as!(
        Miner,
        r#"
        SELECT id, address, pkh
        FROM miners
        WHERE pkh = $1
        "#,
        pkh
    )
    .fetch_optional(pool)
    .await
}