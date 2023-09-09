use sqlx::SqlitePool;

use crate::model::mining_session::{self,  NewMiningSession};

#[derive(Debug)]
pub enum CreateMiningSessionError {
    DatabaseError(sqlx::Error),
    SessionAlreadyExists,
}

impl From<sqlx::Error> for CreateMiningSessionError {
    fn from(err: sqlx::Error) -> Self {
        CreateMiningSessionError::DatabaseError(err)
    }
}

#[derive(Debug)]
pub enum UpdateMiningSessionError {
    DatabaseError(sqlx::Error),
    SessionNotFound,
    SessionIsOver,
    InvalidInput
}

impl From<sqlx::Error> for UpdateMiningSessionError {
    fn from(err: sqlx::Error) -> Self {
        UpdateMiningSessionError::DatabaseError(err)
    }
}

pub async fn create_session(
    pool: &SqlitePool,
    new_session: NewMiningSession<'_>,
) -> Result<i64, CreateMiningSessionError> {
    let maybe_session = mining_session::get_latest(pool, &new_session.public_key_hash).await?;
    
    match maybe_session {
        Some(session) => {
            match session.end_time {
                Some(_) => {
                    // existing user who has closed their previous session
                    mining_session::create(pool, new_session)
                        .await
                        .map_err(CreateMiningSessionError::from)
                },
                None => {
                    Err(CreateMiningSessionError::SessionAlreadyExists)
                }
            }
        }
        None => {
            // completely new user
            mining_session::create(pool, new_session)
                .await
                .map_err(CreateMiningSessionError::from)
        }
    }
}

pub async fn end_session(
    pool: &SqlitePool,
    pkh: &str,
    end_time: chrono::NaiveDateTime,
) -> Result<(), UpdateMiningSessionError> {
    let maybe_current_session = mining_session::get_latest(pool, pkh).await?;

    match maybe_current_session {
        Some(current_session) => {
            match current_session.end_time {
                Some(_) => {
                    return Err(UpdateMiningSessionError::SessionIsOver)
                },
                None => {
                    if end_time <= current_session.start_time {
                        return Err(UpdateMiningSessionError::InvalidInput)
                    }
                }
            }

            mining_session::update_end_time(pool, pkh, end_time)
                .await
                .map_err(UpdateMiningSessionError::from)
        }
        None => {
            Err(UpdateMiningSessionError::SessionNotFound)
        }
    }
}