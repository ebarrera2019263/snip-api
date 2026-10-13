use axum::{Json, extract::State, http::StatusCode};
use chrono::Utc;

use crate::{
    auth::{self, AuthUser},
    error::{AppError, AppResult, ErrorBody},
    extract::ValidatedJson,
    models::{AuthResponse, Credentials, User, UserRow},
    state::AppState,
};

/// Create an account and receive a token.
#[utoipa::path(
    post, path = "/api/auth/register", tag = "auth",
    request_body = Credentials,
    responses(
        (status = 201, description = "Account created", body = AuthResponse),
        (status = 409, description = "Email already registered", body = ErrorBody),
        (status = 422, description = "Invalid input", body = ErrorBody),
    )
)]
pub async fn register(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<Credentials>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    let email = input.email.trim().to_lowercase();
    let password_hash = auth::hash_password(input.password).await?;

    let user = sqlx::query_as::<_, UserRow>(
        "INSERT INTO users (email, password_hash, created_at) VALUES (?, ?, ?) RETURNING *",
    )
    .bind(&email)
    .bind(&password_hash)
    .bind(Utc::now())
    .fetch_one(&state.db)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            AppError::Conflict("an account with this email already exists".into())
        }
        _ => e.into(),
    })?;

    let token = auth::create_token(
        user.id,
        &state.config.jwt_secret,
        state.config.jwt_ttl_hours,
    )?;
    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            user: user.into(),
        }),
    ))
}

/// Exchange email and password for a token.
#[utoipa::path(
    post, path = "/api/auth/login", tag = "auth",
    request_body = Credentials,
    responses(
        (status = 200, description = "Logged in", body = AuthResponse),
        (status = 401, description = "Wrong email or password", body = ErrorBody),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<Credentials>,
) -> AppResult<Json<AuthResponse>> {
    let user = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE email = ?")
        .bind(input.email.trim().to_lowercase())
        .fetch_optional(&state.db)
        .await?
        // Same error for unknown email and wrong password: don't reveal which accounts exist.
        .ok_or(AppError::Unauthorized)?;

    if !auth::verify_password(input.password, user.password_hash.clone()).await? {
        return Err(AppError::Unauthorized);
    }

    let token = auth::create_token(
        user.id,
        &state.config.jwt_secret,
        state.config.jwt_ttl_hours,
    )?;
    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

/// Return the authenticated user.
#[utoipa::path(
    get, path = "/api/auth/me", tag = "auth",
    security(("bearer" = [])),
    responses(
        (status = 200, body = User),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<User>> {
    let row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?")
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?
        // Valid token for a user that no longer exists.
        .ok_or(AppError::Unauthorized)?;
    Ok(Json(row.into()))
}
