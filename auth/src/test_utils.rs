//! Test-only verifier that accepts a single known token.

use crate::{AccessTokenVerifier, AuthError, Principal};

/// Accepts exactly one token string and returns a fixed [`Principal`].
#[derive(Clone, Debug)]
pub struct FixedTokenVerifier {
    token: String,
    principal: Principal,
}

impl FixedTokenVerifier {
    pub fn new(token: impl Into<String>, principal: Principal) -> Self {
        Self {
            token: token.into(),
            principal,
        }
    }
}

impl AccessTokenVerifier for FixedTokenVerifier {
    fn verify_bearer_token(&self, token: &str) -> Result<Principal, AuthError> {
        if token == self.token {
            Ok(self.principal.clone())
        } else {
            Err(AuthError::InvalidToken)
        }
    }
}
