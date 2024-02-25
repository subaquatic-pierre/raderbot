use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Generates an HMAC signature using SHA256.
///
/// This function creates a secure HMAC signature for a given message and secret key, utilizing the SHA256 hashing algorithm.
/// It is commonly used for generating secure signatures in authentication processes or data integrity verification.
///
/// # Parameters
///
/// * `secret`: A `&str` representing the secret key used for HMAC generation.
/// * `message`: A `&str` containing the message to be signed.
///
/// # Returns
///
/// A `String` representing the hexadecimal encoded HMAC signature.
///
/// # Panics
///
/// This function will panic if the secret key's length does not meet the requirements of the SHA256 hashing algorithm.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// let secret = "my_secret_key";
/// let message = "Hello, HMAC!";
/// let hmac_signature = sign_hmac(secret, message);
/// println!("Generated HMAC: {}", hmac_signature);
/// ```
pub fn sign_hmac(secret: &str, message: &str) -> String {
    let mut hmac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("Invalid key length");
    hmac.update(message.as_bytes());
    let result = hmac.finalize();
    hex::encode(result.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_hmac() {
        let secret = "testsecret";
        let message = "testmessage";
        let hmac_result = sign_hmac(secret, message);

        // The HMAC result should be a non-empty string of hexadecimal characters.
        assert!(!hmac_result.is_empty());
        assert!(hmac_result.len() % 2 == 0); // Hex strings have an even length.

        // Further tests could include comparing the result against a known HMAC value,
        // but this would require a fixed secret and message, and the expected result pre-calculated.
    }
}
