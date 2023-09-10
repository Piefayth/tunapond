use chrono::NaiveDateTime;
use serde::{Serialize, Deserialize};

use crate::service::block::ReadableBlock;

#[derive(Debug, Serialize, Deserialize)]
pub struct GenericMessageResponse {
    pub message: String
}

#[derive(Serialize, Deserialize)]
pub struct Registration {
    pub address: String,
    pub key: String,
    pub payload: String,
    pub signature: String,
}

#[derive(Serialize, Deserialize)]
pub struct RegistrationResponse {
    pub address: String,
    pub message: String,
    pub session_id: i64,
    pub start_time: NaiveDateTime,
    pub current_block: ReadableBlock
}