use sqlx::{Postgres, Pool};

// Define the Miner structure to map with the database table.
pub struct Miner {
    pub id: i32,
    pub address: String,
    pub pkh: String,
    pub sampling_difficulty: i32,
}

// Function to create a new miner.
pub async fn create_miner(pool: &Pool<Postgres>, pkh: String, address: String) -> Result<Miner, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let miner = sqlx::query_as!(
        Miner,
        r#"
        INSERT INTO miners
        (pkh, address)
        VALUES ($1, $2)
        RETURNING id, address, pkh, sampling_difficulty
        "#,
        pkh, address
    )
    .fetch_one(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(miner)
}

// Optionally: Function to get a miner by its primary key hash (pkh).
pub async fn get_miner_by_pkh(pool: &Pool<Postgres>, pkh: &str) -> Result<Option<Miner>, sqlx::Error> {
    sqlx::query_as!(
        Miner,
        r#"
        SELECT id, address, pkh, sampling_difficulty
        FROM miners
        WHERE pkh = $1
        "#,
        pkh
    )
    .fetch_optional(pool)
    .await
}

pub async fn update_sampling_difficulty_by_pkh(pool: &Pool<Postgres>, pkh: &str, new_difficulty: u8) -> Result<Miner, sqlx::Error> {
    let result = sqlx::query_as!(
        Miner,
        r#"
        UPDATE miners
        SET sampling_difficulty = $1
        WHERE pkh = $2
        RETURNING id, address, pkh, sampling_difficulty
        "#,
        new_difficulty as i32, 
        pkh
    )
    .fetch_one(pool)
    .await?;

    Ok(result)
}