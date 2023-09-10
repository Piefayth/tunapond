use std::sync::Arc;

use actix_web::{ post, web, HttpResponse, Responder, ResponseError};
use serde::{Serialize, Deserialize};
use sqlx::SqlitePool;

use crate::{routes::common::GenericMessageResponse, address, service::{proof_of_work::{submit_proof_of_work, SubmitProofOfWorkError, SubmitProofOfWorkResponse}, block::BlockService}};

#[derive(Serialize, Deserialize)]
pub struct Submission {
    pub address: String,
    pub entries: Vec<SubmissionEntry>
}

#[derive(Serialize, Deserialize)]
pub struct SubmissionEntry {
    pub sha: String,
    pub nonce: String
}

#[post("/submit")]
async fn submit(
    pool: web::Data<SqlitePool>,
    block_service: web::Data<Arc<BlockService>>,
    submission: web::Json<Submission>,
) -> impl Responder {
    let maybe_pkh = address::pkh_from_address(&submission.address);

    let Ok(pkh) = maybe_pkh else {
        return HttpResponse::BadRequest().json(
            GenericMessageResponse { 
                message: format!("Could not create a valid public key hash for address {}", submission.address)
            }
        )
    };
    
    let result = submit_proof_of_work(&pool, &block_service, pkh, &submission).await;

    match result {
        Ok(submission_response) => {
            HttpResponse::Ok().json(submission_response)
        },
        Err(e) => {
            match e {
                SubmitProofOfWorkError::DatabaseError(_) => {
                    HttpResponse::InternalServerError().json(
                        GenericMessageResponse { 
                            message: String::from("Unexpected database error.")
                        }
                    )
                },
                SubmitProofOfWorkError::NoCurrentSession => {
                    HttpResponse::BadRequest().json(
                        GenericMessageResponse { 
                            message: format!("No current session found for {}", &submission.address)
                        }
                    )
                },
                SubmitProofOfWorkError::BlockServiceFailure(_) => {
                    HttpResponse::InternalServerError().json(
                        GenericMessageResponse { 
                            message: format!("Could not verify submission - BlockService is down.")
                        }
                    )
                },
                SubmitProofOfWorkError::PlutusParseError(_) => {
                    HttpResponse::InternalServerError().json(
                        GenericMessageResponse { 
                            message: format!("Could not verify submission - unable to parse plutus data.")
                        }
                    )
                },
                SubmitProofOfWorkError::InvalidTargetState => {
                    HttpResponse::InternalServerError().json(
                        GenericMessageResponse { 
                            message: format!("Could not verify submission - unable to create a valid target state.")
                        }
                    )
                },
                SubmitProofOfWorkError::SubmissionError(_) => {
                    HttpResponse::InternalServerError().json(
                        GenericMessageResponse { 
                            message: format!("Failed to submit a valid block!")
                        }
                    )
                }
            }
        },
    }
}