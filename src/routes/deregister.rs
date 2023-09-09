
use actix_web::{ post, web, HttpResponse, Responder, ResponseError};
use crate::routes::common::{Registration, GenericMessageResponse};
use crate::service::mining_session::{end_session, UpdateMiningSessionError};
use crate::{signature_verifier, address};
use sqlx::SqlitePool;

#[post("/deregister")]
async fn deregister(
    pool: web::Data<SqlitePool>,
    registration: web::Json<Registration>,
) -> impl Responder {
    let verify_result = signature_verifier::verify_message(&registration.address, &registration.key, &registration.payload, &registration.signature);

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

                let session_end_result = end_session(&pool, &pkh, chrono::Utc::now().naive_utc()).await;

                match session_end_result {
                    Ok(_) => {
                        HttpResponse::Ok().json(
                            GenericMessageResponse {
                                message: format!("Ended mining session for {}.", registration.address)
                            }
                        )
                    },
                    Err(UpdateMiningSessionError::SessionNotFound) => {
                        HttpResponse::BadRequest().json(
                            GenericMessageResponse { 
                                message: format!("Could not find an in progress mining session for {}.", registration.address)
                            }
                        )
                    },
                    Err(UpdateMiningSessionError::SessionIsOver) => {
                        HttpResponse::BadRequest().json(
                            GenericMessageResponse { 
                                message: format!("The most recent mining session for {} was already closed.", registration.address)
                            }
                        )
                    }
                    Err(UpdateMiningSessionError::InvalidInput) => {
                        HttpResponse::BadRequest().json(
                            GenericMessageResponse { 
                                message: format!("Could not create a valid session update for {} with the given registration.", registration.address)
                            }
                        )
                    }
                    Err(UpdateMiningSessionError::DatabaseError(_)) => {
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