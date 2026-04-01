//! Authentication domain: [`Principal`], [`AuthError`], and [`AccessTokenVerifier`].
//!
//! This crate is intentionally free of Trailbase or HTTP dependencies so the rest of the
//! workspace can depend only on these abstractions.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Application role derived from the access token (Trailbase: `admin` claim).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Admin,
}

/// Authenticated subject derived from a validated access token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    /// Stable subject identifier (Trailbase: url-safe base64 user id).
    pub sub: String,
    pub email: String,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub mfa: bool,
    pub csrf_token: String,
}

impl Principal {
    /// Maps [`Principal::is_admin`] to [`Role::Admin`] or [`Role::User`].
    pub fn role(&self) -> Role {
        if self.is_admin {
            Role::Admin
        } else {
            Role::User
        }
    }
}

/// Errors from access-token verification.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid or unverifiable token")]
    InvalidToken,
    #[error("token is not an auth session token")]
    WrongTokenType,
    #[error("malformed authorization header")]
    MalformedAuthorization,
}

/// Verifies a raw JWT string (without the `Bearer ` prefix).
pub trait AccessTokenVerifier: Send + Sync {
    fn verify_bearer_token(&self, token: &str) -> Result<Principal, AuthError>;
}

#[cfg(feature = "test-utils")]
mod test_utils;

#[cfg(feature = "test-utils")]
pub use test_utils::FixedTokenVerifier;
