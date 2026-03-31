//! JWT payload for Trailbase auth tokens.
//!
//! Must stay aligned with upstream `AuthTokenClaims` in Trailbase
//! (`crates/core/src/auth/jwt.rs` in https://github.com/trailbaseio/trailbase).

use serde::Deserialize;

/// `TokenType::Auth` in Trailbase (`repr(u8)` enum: Unknown=0, Auth=1, …).
pub const TRAILBASE_AUTH_TOKEN_TYPE: u8 = 1;

// `iat` / `exp` are validated by jsonwebtoken; we keep them for correct deserialization.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct AuthTokenJwtClaims {
    pub sub: String,
    pub iat: i64,
    pub exp: i64,
    #[serde(rename = "type")]
    pub token_type: u8,
    #[serde(default)]
    pub admin: bool,
    #[serde(default)]
    pub mfa: bool,
    pub email: String,
    pub csrf_token: String,
}
