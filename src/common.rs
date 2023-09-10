use serde::{Serialize, Deserialize};

#[derive(Debug,Serialize, Deserialize)]
pub struct GenericMessageResponse {
    pub message: String
}