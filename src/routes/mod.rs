pub mod auth;
pub mod links;
pub mod redirect;

use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    demo,
    error::{AppError, AppResult, ErrorBody},
    state::AppState,
};

#[derive(Serialize, ToSchema)]
pub struct Health {
    #[schema(example = "ok")]
    status: &'static str,
    #[schema(example = "0.1.0")]
    version: &'static str,
}

/// Liveness check that also verifies the database is reachable.
#[utoipa::path(
    get, path = "/health", tag = "health",
    responses((status = 200, body = Health))
)]
pub async fn health(State(state): State<AppState>) -> AppResult<Json<Health>> {
    sqlx::query("SELECT 1").execute(&state.db).await?;
    Ok(Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    }))
}

#[derive(Serialize, ToSchema)]
pub struct DemoAccount {
    #[schema(example = "demo@snip.dev")]
    email: &'static str,
    #[schema(example = "demo-password")]
    password: &'static str,
}

/// Credentials of the public demo account, when the server runs in demo mode.
#[utoipa::path(
    get, path = "/api/demo", tag = "auth",
    responses(
        (status = 200, body = DemoAccount),
        (status = 404, description = "Demo mode is off", body = ErrorBody),
    )
)]
pub async fn demo_account(State(state): State<AppState>) -> AppResult<Json<DemoAccount>> {
    if !state.config.demo_mode {
        return Err(AppError::NotFound);
    }
    Ok(Json(DemoAccount {
        email: demo::EMAIL,
        password: demo::PASSWORD,
    }))
}
