//! Password hashing (Argon2id) and JWT issuing/validation.

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use axum::{extract::FromRequestParts, http::request::Parts};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// User id.
    sub: i64,
    exp: i64,
    iat: i64,
}

/// Hashes a password on a blocking thread: Argon2 is intentionally CPU-heavy
/// and would otherwise stall the async runtime.
pub async fn hash_password(password: String) -> AppResult<String> {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|hash| hash.to_string())
            .map_err(|e| AppError::Internal(format!("hashing failed: {e}")))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

pub async fn verify_password(password: String, hash: String) -> AppResult<bool> {
    tokio::task::spawn_blocking(move || {
        let parsed =
            PasswordHash::new(&hash).map_err(|e| AppError::Internal(format!("bad hash: {e}")))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

pub fn create_token(user_id: i64, secret: &str, ttl_hours: i64) -> AppResult<String> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user_id,
        iat: now,
        exp: now + ttl_hours * 3600,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("token encoding failed: {e}")))
}

/// Extractor for authenticated routes: add `user: AuthUser` to a handler's
/// arguments and the request is rejected with 401 unless it carries a valid
/// `Authorization: Bearer <token>` header.
#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub id: i64,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> AppResult<Self> {
        let token = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;

        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized)?;

        Ok(Self {
            id: data.claims.sub,
        })
    }
}
