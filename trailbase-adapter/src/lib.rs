//! Trailbase-specific authentication: JWT verification (Ed25519) and helpers for the HTTP client.
//!
//! Run Trailbase as a **sidecar**; configure this app with the sidecar’s JWT **public** PEM
//! (from the Trailbase data directory or a one-time admin export). See `cinema-booking-config`
//! `TrailbaseAuthConfig` and module comments on `TrailbaseClientConfig`.

mod claims;

use cinema_booking_auth::{AccessTokenVerifier, AuthError, Principal};
use claims::{AuthTokenJwtClaims, TRAILBASE_AUTH_TOKEN_TYPE};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use thiserror::Error;
use trailbase_client::Client;

pub use trailbase_client;

/// Builds a [`Client`] for the given Trailbase base URL (no stored tokens).
pub fn trailbase_http_client(
    base_url: impl AsRef<str>,
) -> Result<Client, trailbase_client::Error> {
    Client::new(base_url.as_ref(), None)
}

/// Configuration for creating a [`trailbase_client::Client`].
#[derive(Clone, Debug)]
pub struct TrailbaseClientConfig {
    pub base_url: String,
}

impl TrailbaseClientConfig {
    pub fn connect(&self) -> Result<Client, trailbase_client::Error> {
        trailbase_http_client(&self.base_url)
    }
}

/// Failed to construct a JWT verifier from PEM.
#[derive(Debug, Error)]
pub enum TrailbaseJwtVerifierError {
    #[error("invalid Ed25519 public key PEM: {0}")]
    Key(#[from] jsonwebtoken::errors::Error),
}

/// Verifies Trailbase-issued auth JWTs using the sidecar’s public key.
pub struct TrailbaseJwtVerifier {
    key: DecodingKey,
    validation: Validation,
}

impl TrailbaseJwtVerifier {
    pub fn from_public_key_pem(pem: &[u8]) -> Result<Self, TrailbaseJwtVerifierError> {
        let key = DecodingKey::from_ed_pem(pem)?;
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.validate_exp = true;
        Ok(Self { key, validation })
    }
}

impl AccessTokenVerifier for TrailbaseJwtVerifier {
    fn verify_bearer_token(&self, token: &str) -> Result<Principal, AuthError> {
        let data = decode::<AuthTokenJwtClaims>(token, &self.key, &self.validation)
            .map_err(|_| AuthError::InvalidToken)?;
        let c = data.claims;
        if c.token_type != TRAILBASE_AUTH_TOKEN_TYPE {
            return Err(AuthError::WrongTokenType);
        }
        Ok(Principal {
            sub: c.sub,
            email: c.email,
            is_admin: c.admin,
            mfa: c.mfa,
            csrf_token: c.csrf_token,
        })
    }
}
