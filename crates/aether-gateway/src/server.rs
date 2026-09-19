use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
};

use crate::crypto::{VerificationError, verify_signature};

const SIGNATURE_HEADER: &str = "x-hub-signature-256";

#[derive(Clone)]
pub struct AppState {
    pub webhook_secret: bytes::Bytes,
}

pub fn create_router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/webhook", post(handle_webhook))
        .with_state(state)
}

async fn handle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> StatusCode {
    let Some(signature_header) = headers.get(SIGNATURE_HEADER) else {
        return StatusCode::BAD_REQUEST;
    };
    let Ok(signature_header) = signature_header.to_str() else {
        return StatusCode::BAD_REQUEST;
    };

    match verify_signature(
        state.webhook_secret.as_ref(),
        signature_header,
        body.as_ref(),
    ) {
        Ok(()) => StatusCode::ACCEPTED,
        Err(VerificationError::SignatureMismatch) => StatusCode::UNAUTHORIZED,
        Err(VerificationError::InvalidHeaderFormat | VerificationError::InvalidHex) => {
            StatusCode::BAD_REQUEST
        }
        Err(VerificationError::KeyInitializationError) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{HeaderValue, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::{AppState, SIGNATURE_HEADER, create_router};

    const RFC_4231_SIGNATURE: &str =
        "sha256=b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7";
    const RFC_4231_SECRET: &[u8] = &[0x0b; 20];
    const RFC_4231_BODY: &[u8] = b"Hi There";

    #[tokio::test]
    async fn valid_signature_returns_accepted() {
        let request = Request::builder()
            .method("POST")
            .uri("/webhook")
            .header(SIGNATURE_HEADER, RFC_4231_SIGNATURE)
            .body(Body::from(RFC_4231_BODY))
            .expect("request must be valid");

        assert_eq!(send_request(request).await, StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn missing_header_returns_bad_request() {
        let request = Request::builder()
            .method("POST")
            .uri("/webhook")
            .body(Body::from(RFC_4231_BODY))
            .expect("request must be valid");

        assert_eq!(send_request(request).await, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn invalid_header_prefix_returns_bad_request() {
        let request = Request::builder()
            .method("POST")
            .uri("/webhook")
            .header(
                SIGNATURE_HEADER,
                "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
            )
            .body(Body::from(RFC_4231_BODY))
            .expect("request must be valid");

        assert_eq!(send_request(request).await, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn malformed_hex_signature_returns_bad_request() {
        let request = Request::builder()
            .method("POST")
            .uri("/webhook")
            .header(SIGNATURE_HEADER, "sha256=invalid_hex_characters")
            .body(Body::from(RFC_4231_BODY))
            .expect("request must be valid");

        assert_eq!(send_request(request).await, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn non_utf8_header_returns_bad_request() {
        let signature =
            HeaderValue::from_bytes(b"\xff\xfe").expect("opaque header value must be valid");
        let request = Request::builder()
            .method("POST")
            .uri("/webhook")
            .header(SIGNATURE_HEADER, signature)
            .body(Body::from(RFC_4231_BODY))
            .expect("request must be valid");

        assert_eq!(send_request(request).await, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn invalid_signature_returns_unauthorized() {
        let request = Request::builder()
            .method("POST")
            .uri("/webhook")
            .header(
                SIGNATURE_HEADER,
                "sha256=b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff6",
            )
            .body(Body::from(RFC_4231_BODY))
            .expect("request must be valid");

        assert_eq!(send_request(request).await, StatusCode::UNAUTHORIZED);
    }

    async fn send_request(request: Request<Body>) -> StatusCode {
        let state = AppState {
            webhook_secret: bytes::Bytes::from_static(RFC_4231_SECRET),
        };
        let response = create_router(state)
            .oneshot(request)
            .await
            .expect("router must return a response");
        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body must be readable")
            .to_bytes();
        assert!(body.is_empty());

        status
    }
}
