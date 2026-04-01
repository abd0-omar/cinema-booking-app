//! Test-only verifier that accepts known tokens mapped to fixed [`Principal`]s.

use crate::{AccessTokenVerifier, AuthError, Principal};
use std::collections::HashMap;

/// Accepts configured token strings and returns the matching [`Principal`].
#[derive(Clone, Debug)]
pub struct FixedTokenVerifier {
    tokens: HashMap<String, Principal>,
}

impl FixedTokenVerifier {
    /// Single token → principal (convenience for one test identity).
    pub fn new(token: impl Into<String>, principal: Principal) -> Self {
        let mut tokens = HashMap::new();
        tokens.insert(token.into(), principal);
        Self { tokens }
    }

    /// Multiple tokens (e.g. user vs admin) for integration tests.
    pub fn from_pairs(pairs: impl IntoIterator<Item = (String, Principal)>) -> Self {
        Self {
            tokens: pairs.into_iter().collect(),
        }
    }
}

impl AccessTokenVerifier for FixedTokenVerifier {
    fn verify_bearer_token(&self, token: &str) -> Result<Principal, AuthError> {
        self.tokens
            .get(token)
            .cloned()
            .ok_or(AuthError::InvalidToken)
    }
}
