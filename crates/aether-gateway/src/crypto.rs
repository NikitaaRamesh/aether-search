use hmac::{Hmac, Mac};
use sha2::Sha256;

const SIGNATURE_PREFIX: &str = "sha256=";
const SHA256_HEX_LENGTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationError {
    InvalidHeaderFormat,
    InvalidHex,
    KeyInitializationError,
    SignatureMismatch,
}

impl std::fmt::Display for VerificationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidHeaderFormat => "signature header must start with `sha256=`",
            Self::InvalidHex => "signature must contain exactly 64 hexadecimal characters",
            Self::KeyInitializationError => "failed to initialize the HMAC key",
            Self::SignatureMismatch => "signature does not match the payload",
        };

        formatter.write_str(message)
    }
}

impl std::error::Error for VerificationError {}

pub fn verify_signature(
    secret: &[u8],
    signature_header: &str,
    body: &[u8],
) -> Result<(), VerificationError> {
    let encoded_signature = signature_header
        .strip_prefix(SIGNATURE_PREFIX)
        .ok_or(VerificationError::InvalidHeaderFormat)?;
    let signature = decode_sha256_hex(encoded_signature)?;

    let mut mac = Hmac::<Sha256>::new_from_slice(secret)
        .map_err(|_| VerificationError::KeyInitializationError)?;
    mac.update(body);
    mac.verify_slice(&signature)
        .map_err(|_| VerificationError::SignatureMismatch)
}

fn decode_sha256_hex(encoded: &str) -> Result<[u8; 32], VerificationError> {
    let encoded = encoded.as_bytes();
    if encoded.len() != SHA256_HEX_LENGTH {
        return Err(VerificationError::InvalidHex);
    }

    let mut decoded = [0_u8; 32];
    let (pairs, remainder) = encoded.as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    for (output, pair) in decoded.iter_mut().zip(pairs) {
        let high = decode_hex_nibble(pair[0])?;
        let low = decode_hex_nibble(pair[1])?;
        *output = (high << 4) | low;
    }

    Ok(decoded)
}

fn decode_hex_nibble(value: u8) -> Result<u8, VerificationError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(VerificationError::InvalidHex),
    }
}

#[cfg(test)]
mod tests {
    use super::{VerificationError, verify_signature};

    const RFC_4231_SIGNATURE: &str =
        "sha256=b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7";
    const RFC_4231_SECRET: &[u8] = &[0x0b; 20];
    const RFC_4231_BODY: &[u8] = b"Hi There";

    #[test]
    fn verify_valid_signature() {
        assert_eq!(
            verify_signature(RFC_4231_SECRET, RFC_4231_SIGNATURE, RFC_4231_BODY),
            Ok(())
        );
    }

    #[test]
    fn reject_tampered_payload() {
        let mut tampered_body = *b"Hi There";
        tampered_body[0] ^= 1;

        assert_eq!(
            verify_signature(RFC_4231_SECRET, RFC_4231_SIGNATURE, &tampered_body),
            Err(VerificationError::SignatureMismatch)
        );
    }

    #[test]
    fn reject_invalid_header_prefix() {
        assert_eq!(
            verify_signature(
                RFC_4231_SECRET,
                RFC_4231_SIGNATURE
                    .strip_prefix("sha256=")
                    .expect("test signature has the expected prefix"),
                RFC_4231_BODY,
            ),
            Err(VerificationError::InvalidHeaderFormat)
        );
    }

    #[test]
    fn reject_malformed_hex() {
        assert_eq!(
            verify_signature(
                RFC_4231_SECRET,
                "sha256=b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cffz",
                RFC_4231_BODY,
            ),
            Err(VerificationError::InvalidHex)
        );
        assert_eq!(
            verify_signature(RFC_4231_SECRET, "sha256=00", RFC_4231_BODY),
            Err(VerificationError::InvalidHex)
        );
    }
}
