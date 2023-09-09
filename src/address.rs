use core::fmt;
use cardano_multiplatform_lib as C;

use actix_web::HttpResponse;

#[derive(Debug)]
pub struct AddressParseError(pub String);

impl fmt::Display for AddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl actix_web::error::ResponseError for AddressParseError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::BadRequest().body(format!(r###"{{ "message": "{}" }}"###, &self.0))
    }
}


pub fn pkh_from_address(address: &str) -> Result<String, AddressParseError> {
    let c_address = C::address::Address::from_bech32(address).map_err(address_error("Input address should have been in bech32 format."))?;
    let keyhash = c_address.payment_cred().or_else(|| c_address.staking_cred())
        .ok_or(AddressParseError(String::from("Could not derive credentials from address.")))?
        .to_keyhash()
        .ok_or(AddressParseError(String::from("Could not derive keyhash from address.")))?
        .to_hex();

    Ok(keyhash)
}

fn address_error<E>(msg: &'static str) -> impl FnOnce(E) -> AddressParseError {
    |_| AddressParseError(msg.to_string())
}