use axum::{
    extract::{Path, State},
    http::{HeaderMap, header},
    response::Redirect,
};
use chrono::Utc;

use crate::{
    error::{AppError, AppResult, ErrorBody},
    models::LinkRow,
    state::AppState,
};

/// Header values are client-controlled; cap what we store.
const MAX_HEADER_LEN: usize = 512;

fn header_value(headers: &HeaderMap, name: header::HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.chars().take(MAX_HEADER_LEN).collect())
}

/// Follow a short link. Records the click and redirects to the target URL.
#[utoipa::path(
    get, path = "/{code}", tag = "redirect",
    params(("code" = String, Path, description = "Short code")),
    responses(
        (status = 307, description = "Redirect to the target URL"),
        (status = 404, description = "Unknown code", body = ErrorBody),
        (status = 410, description = "Link disabled or expired", body = ErrorBody),
    )
)]
pub async fn follow(
    State(state): State<AppState>,
    Path(code): Path<String>,
    headers: HeaderMap,
) -> AppResult<Redirect> {
    let link = sqlx::query_as::<_, LinkRow>("SELECT * FROM links WHERE code = ?")
        .bind(&code)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    let now = Utc::now();
    if !link.is_available(now) {
        return Err(AppError::Gone);
    }

    // Record the click and bump the counter atomically.
    let mut tx = state.db.begin().await?;
    sqlx::query(
        "INSERT INTO clicks (link_id, clicked_at, referrer, user_agent) VALUES (?, ?, ?, ?)",
    )
    .bind(link.id)
    .bind(now)
    .bind(header_value(&headers, header::REFERER))
    .bind(header_value(&headers, header::USER_AGENT))
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE links SET click_count = click_count + 1 WHERE id = ?")
        .bind(link.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Redirect::temporary(&link.target_url))
}
