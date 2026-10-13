pub mod auth;
pub mod links;
pub mod redirect;

use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{error::AppResult, state::AppState};

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
