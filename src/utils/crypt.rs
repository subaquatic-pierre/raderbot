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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_hmac() {
        // Test with valid secret and message
        let secret = "your_secret_key";
        let message = "some_message";
        let result = sign_hmac(secret, message);
        assert_eq!(result.len(), 64); // Check if the result is a valid SHA256 HMAC

        // Test with another valid secret and message
        let secret = "another_secret_key";
        let message = "another_message";
        let result = sign_hmac(secret, message);
        assert_eq!(result.len(), 64);

        // Test with an empty secret
        let empty_secret = "";
        let message = "some_message";
        let result = sign_hmac(empty_secret, message);
        assert_eq!(result.len(), 64);

        // Test with an empty message
        let secret = "your_secret_key";
        let empty_message = "";
        let result = sign_hmac(secret, empty_message);
        assert_eq!(result.len(), 64);
    }
}
