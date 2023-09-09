use core::fmt;

use actix_web::HttpResponse;
use cardano_multiplatform_lib as C;
use cardano_message_signing as M;
use M::{utils::{FromBytes, ToBytes}, COSESign1, COSEKey};

#[derive(Debug)]
pub struct SignatureVerificationError(pub String);

impl fmt::Display for SignatureVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl actix_web::error::ResponseError for SignatureVerificationError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::BadRequest().body(format!(r###"{{ "message": "{}" }}"###, &self.0))
    }
}

pub fn verify_message(
    address: &str,
    key: &str,
    payload: &str,
    signature: &str,
) -> Result<bool, SignatureVerificationError> {
    let input_address = C::address::Address::from_bech32(address).map_err(signature_error("Input address should have been in bech32 format."))?;
    
    let signature_bytes = hex::decode(signature).map_err(signature_error("Signature was not valid hex."))?;
    let signature = COSESign1::from_bytes(signature_bytes).map_err(signature_error("Signature was not valid COSE."))?;
    
    let key_bytes = hex::decode(key).map_err(signature_error("Public key was not valid hex."))?;
    let cose_key = COSEKey::from_bytes(key_bytes).map_err(signature_error("Key was not valid COSE."))?;

    let protected_headers = signature.headers().protected().deserialized_headers();
    let signed_address = protected_headers.header(&M::Label::new_text(String::from("address")))
        .ok_or(SignatureVerificationError(String::from("No address found in the headers of the signature.")))?;
    
    let signed_address_bytes = signed_address.as_bytes()
        .ok_or(SignatureVerificationError(String::from("Signature contained an address that could not be interpreted as bytes.")))?;

    let cose_algorithm = protected_headers.algorithm_id()
        .ok_or(SignatureVerificationError(String::from("Signature did not specify a COSE algorithm_id.")))?
        .as_int()
        .ok_or(SignatureVerificationError(String::from("Signature's specified COSE algorithm_id was not an integer.")))?
        .as_i32()
        .ok_or(SignatureVerificationError(String::from("Could not parse signature  COSE algorithm as i32.")))?;

    let key_algorithm = cose_key.algorithm_id()
        .ok_or(SignatureVerificationError(String::from("Key did not specify a COSE algorithm_id.")))?
        .as_int()
        .ok_or(SignatureVerificationError(String::from("Key's specified COSE algorithm_id was not an integer.")))?
        .as_i32()
        .ok_or(SignatureVerificationError(String::from("Could not parse key COSE algorithm as i32.")))?;

    let key_curve = cose_key.header(&M::Label::new_int(
        &M::utils::Int::new_negative(
            M::utils::BigNum::from_str("1").unwrap())
        ))
        .ok_or(SignatureVerificationError(String::from("Could not derive key curve from key.")))?
        .as_int()
        .ok_or(SignatureVerificationError(String::from("Key's specified curve was not an integer.")))?
        .as_i32()
        .ok_or(SignatureVerificationError(String::from("Could not parse key COSE algorithm as i32.")))?;

    let key_type = cose_key.key_type().as_int()
        .ok_or(SignatureVerificationError(String::from("Could not derive key type from key.")))?
        .as_i32()
        .ok_or(SignatureVerificationError(String::from("Could not parse key COSE algorithm as i32.")))?;

    let public_key_bytes = cose_key.header(&M::Label::new_int(
        &M::utils::Int::new_negative(
            M::utils::BigNum::from_str("2").unwrap())
        )
    )
    .ok_or(SignatureVerificationError(String::from("Could not get public key curve from key.")))?
    .as_bytes()
    .ok_or(SignatureVerificationError(String::from("Could not interpret public key header as bytes.")))?;

    let public_key = C::crypto::PublicKey::from_bytes(&public_key_bytes).map_err(signature_error("Could not interpret public key as PublicKey."))?;
    let cose_payload = signature.payload().ok_or(SignatureVerificationError(String::from("No payload included in signed message.")))?;

    let ed25519 = C::crypto::Ed25519Signature::from_bytes(signature.signature())
        .map_err(signature_error("Could not parse Ed25519 signature from signature's signature. (Oof.)"))?;

    let data = signature.signed_data(None, None)
        .map_err(signature_error("There should have been data in the signature's signed data."))?
        .to_bytes();


    let input_address_hex = hex::encode(input_address.to_bytes());
    let signed_address_hex = hex::encode(signed_address_bytes);

    let input_address_keyhash = input_address.payment_cred().or_else(|| input_address.staking_cred())
        .ok_or(SignatureVerificationError(String::from("Could not derive credentials from address.")))?
        .to_keyhash()
        .ok_or(SignatureVerificationError(String::from("Could not derive keyhash from address.")))?
        .to_hex();
    let signed_address_keyhash = public_key.hash().to_hex();

    Ok (
        signed_address_hex == input_address_hex && 
        signed_address_keyhash == input_address_keyhash &&
        cose_algorithm == key_algorithm &&
        cose_algorithm == -8 &&
        key_curve == 6 &&
        key_type == 1 &&
        cose_payload == payload.as_bytes() &&
        public_key.verify(&data, &ed25519)
    )
}

fn signature_error<E>(msg: &'static str) -> impl FnOnce(E) -> SignatureVerificationError {
    |_| SignatureVerificationError(msg.to_string())
}