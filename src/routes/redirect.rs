use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
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

/// Browsers ask for `text/html`; API clients and curl don't.
fn wants_html(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("text/html"))
}

/// A readable page for people who open a dead link, instead of raw JSON.
/// The copy is static: nothing from the request is echoed back.
fn error_page(status: StatusCode, title: &str, detail: &str) -> Response {
    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · snip</title>
<style>
  body {{ margin: 0; min-height: 100vh; display: grid; place-items: center;
    background: #eef1f5; color: #1b2433; font: 16px/1.5 system-ui, sans-serif; }}
  main {{ max-width: 28rem; padding: 2rem 1.5rem; }}
  .tape {{ display: inline-block; background: #f2c14e; padding: .25rem 1rem;
    font: 600 .95rem ui-monospace, monospace; transform: rotate(-1.5deg);
    clip-path: polygon(0 0, 100% 0, 97% 50%, 100% 100%, 0 100%, 3% 50%); }}
  h1 {{ font-size: 1.75rem; line-height: 1.2; margin: 1.5rem 0 .5rem; }}
  p {{ color: #5a6578; margin: 0 0 1.5rem; }}
  a {{ color: #2346d8; font-weight: 600; }}
</style>
</head>
<body>
<main>
  <span class="tape">snip</span>
  <h1>{title}</h1>
  <p>{detail}</p>
  <a href="/">Create your own short links</a>
</main>
</body>
</html>"#
    );
    (status, Html(html)).into_response()
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
) -> Response {
    match resolve(&state, &code, &headers).await {
        Ok(redirect) => redirect.into_response(),
        Err(AppError::NotFound) if wants_html(&headers) => error_page(
            StatusCode::NOT_FOUND,
            "This short link doesn't exist",
            "Check the address for typos. The link may also have been deleted by its owner.",
        ),
        Err(AppError::Gone) if wants_html(&headers) => error_page(
            StatusCode::GONE,
            "This link is no longer available",
            "Its owner paused it or it reached its expiry date.",
        ),
        Err(err) => err.into_response(),
    }
}

async fn resolve(state: &AppState, code: &str, headers: &HeaderMap) -> AppResult<Redirect> {
    let link = sqlx::query_as::<_, LinkRow>("SELECT * FROM links WHERE code = ?")
        .bind(code)
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
    .bind(header_value(headers, header::REFERER))
    .bind(header_value(headers, header::USER_AGENT))
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE links SET click_count = click_count + 1 WHERE id = ?")
        .bind(link.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Redirect::temporary(&link.target_url))
}
