pub mod crypto;
pub mod server;

pub use aether_core as core;
pub use crypto::{VerificationError, verify_signature};

pub type Body = bytes::Bytes;
pub type HmacSha256 = hmac::Hmac<sha2::Sha256>;
pub type Router = axum::Router;
pub type Runtime = tokio::runtime::Runtime;

#[cfg(test)]
mod tests {
    #[test]
    fn smoke_test_gateway() {
        assert!(std::hint::black_box(true));
    }
}
