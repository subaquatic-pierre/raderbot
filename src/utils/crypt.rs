use hmac::{Hmac, Mac};
use sha2::Sha256;

pub fn sign_hmac(secret: &str, message: &str) -> String {
    // Create a new HMAC instance with SHA256
    let mut hmac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("Invalid key length");

    // Update the HMAC with the data
    hmac.update(message.as_bytes());

    // Get the resulting HMAC value
    let result = hmac.finalize();

    // Convert the HMAC value to a string
    let hmac_string = hex::encode(result.into_bytes());

    println!("HMAC: {}", hmac_string);

    hmac_string
}
