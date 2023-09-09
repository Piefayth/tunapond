use std::sync::Arc;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use crate::model::mining_session::NewMiningSession;
use crate::routes::common::{Registration, RegistrationResponse, GenericMessageResponse};
use crate::service::block::{BlockService};
use crate::service::mining_session::{CreateMiningSessionError, create_session};
use crate::{signature_verifier, address};
use sqlx::{SqlitePool};

// TODO!!
// To prevent (de)registration replays, the registration payload should contain
// a timestamp that gets verified on the server.
#[post("/register")]
pub async fn register(
    pool: web::Data<SqlitePool>,
    block_service: web::Data<Arc<BlockService>>,
    registration: web::Json<Registration>,
) -> impl Responder {
    let verify_result = signature_verifier::verify_message(&registration.address, &registration.key, &registration.payload, &registration.signature);
    // TODO: How do we un-nest this?

    match verify_result {
        Ok(is_ok) => {
            if !is_ok {
                HttpResponse::BadRequest().json(                            
                    GenericMessageResponse {
                        message: format!("Could not verify the signature of the registration.")
                    }
                )
            } else {
                let maybe_pkh = address::pkh_from_address(&registration.address);

                let Ok(pkh) = maybe_pkh else {
                    return HttpResponse::BadRequest().json(
                        GenericMessageResponse { 
                            message: format!("Could not create a valid public key hash for address {}", registration.address)
                        }
                    )
                };

                let service: &Arc<BlockService> = &block_service;

                let latest_block = match service.get_latest() {
                    Ok(block) => {
                        block
                    }
                    Err(_) => {
                        return HttpResponse::InternalServerError().json(
                            GenericMessageResponse { 
                                message: format!("Could not fetch latest block.")
                            }
                        )
                    }
                };

                let now = chrono::Utc::now().naive_utc();
                let session_create_result = create_session(&pool, NewMiningSession {
                    public_key_hash: &pkh,
                    start_time: now,
                    currently_mining_block: latest_block.block_number
                }).await;

                match session_create_result {
                    Ok(session_id) => {
                        HttpResponse::Ok().json(
                            RegistrationResponse {
                                address: String::from(&registration.address),
                                message: format!("Started a new mining session."),
                                session_id,
                                start_time: now,
                                current_block: latest_block.into()
                            }
                        )
                    },
                    Err(CreateMiningSessionError::SessionAlreadyExists) => {
                        HttpResponse::BadRequest().json(
                            GenericMessageResponse { 
                                message: format!("Mining session already exists for {}.", registration.address)
                            }
                        )
                    }
                    Err(CreateMiningSessionError::DatabaseError(_)) => {
                        HttpResponse::InternalServerError().json(
                            GenericMessageResponse { 
                                message: String::from("Unexpected database error.")
                            }
                        )
                    }
                }
            }
        }
        Err(e) => {
            e.error_response()
        }
    }
}