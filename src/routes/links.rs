use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{Duration, Utc};

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult, ErrorBody},
    extract::ValidatedJson,
    models::{
        CreateLink, DailyClicks, Link, LinkPage, LinkRow, LinkStats, Pagination, ReferrerClicks,
        UpdateLink,
    },
    state::AppState,
};

const CODE_ALPHABET: [char; 62] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9',
];
const GENERATED_CODE_LEN: usize = 7;
/// 62^7 ≈ 3.5 trillion codes, so collisions are rare; retry a few times anyway.
const MAX_CODE_ATTEMPTS: usize = 5;

fn is_unique_violation(err: &sqlx::Error) -> bool {
    matches!(err, sqlx::Error::Database(db) if db.is_unique_violation())
}

/// Fetches a link owned by `user_id`. Links owned by other users are reported
/// as 404 so their existence isn't leaked.
async fn find_owned(state: &AppState, user_id: i64, id: i64) -> AppResult<LinkRow> {
    sqlx::query_as::<_, LinkRow>("SELECT * FROM links WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)
}

/// Create a short link.
#[utoipa::path(
    post, path = "/api/links", tag = "links",
    security(("bearer" = [])),
    request_body = CreateLink,
    responses(
        (status = 201, description = "Link created", body = Link),
        (status = 409, description = "Custom code already taken", body = ErrorBody),
        (status = 422, description = "Invalid input", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    ValidatedJson(input): ValidatedJson<CreateLink>,
) -> AppResult<(StatusCode, Json<Link>)> {
    let now = Utc::now();
    if input.expires_at.is_some_and(|exp| exp <= now) {
        return Err(AppError::Validation(
            "expires_at: must be in the future".into(),
        ));
    }

    let attempts = if input.code.is_some() {
        1
    } else {
        MAX_CODE_ATTEMPTS
    };
    for _ in 0..attempts {
        let code = input
            .code
            .clone()
            .unwrap_or_else(|| nanoid::nanoid!(GENERATED_CODE_LEN, &CODE_ALPHABET));

        let result = sqlx::query_as::<_, LinkRow>(
            "INSERT INTO links (user_id, code, target_url, expires_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?) RETURNING *",
        )
        .bind(user.id)
        .bind(&code)
        .bind(&input.target_url)
        .bind(input.expires_at)
        .bind(now)
        .bind(now)
        .fetch_one(&state.db)
        .await;

        match result {
            Ok(row) => {
                tracing::info!(link_id = row.id, code = %row.code, "link created");
                return Ok((
                    StatusCode::CREATED,
                    Json(Link::from_row(row, &state.config.base_url)),
                ));
            }
            Err(e) if is_unique_violation(&e) => continue,
            Err(e) => return Err(e.into()),
        }
    }

    Err(match input.code {
        Some(code) => AppError::Conflict(format!("the code '{code}' is already taken")),
        None => AppError::Internal("could not generate a unique code".into()),
    })
}

/// List your links, newest first.
#[utoipa::path(
    get, path = "/api/links", tag = "links",
    security(("bearer" = [])),
    params(Pagination),
    responses((status = 200, body = LinkPage), (status = 401, body = ErrorBody))
)]
pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(pagination): Query<Pagination>,
) -> AppResult<Json<LinkPage>> {
    let (page, per_page) = pagination.resolve();
    let offset = i64::from(page - 1) * i64::from(per_page);

    let rows = sqlx::query_as::<_, LinkRow>(
        "SELECT * FROM links WHERE user_id = ? ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?",
    )
    .bind(user.id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM links WHERE user_id = ?")
        .bind(user.id)
        .fetch_one(&state.db)
        .await?;

    let base_url = &state.config.base_url;
    Ok(Json(LinkPage {
        items: rows
            .into_iter()
            .map(|r| Link::from_row(r, base_url))
            .collect(),
        page,
        per_page,
        total,
    }))
}

/// Get one of your links.
#[utoipa::path(
    get, path = "/api/links/{id}", tag = "links",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Link id")),
    responses((status = 200, body = Link), (status = 404, body = ErrorBody))
)]
pub async fn get_one(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Link>> {
    let row = find_owned(&state, user.id, id).await?;
    Ok(Json(Link::from_row(row, &state.config.base_url)))
}

/// Change a link's destination or enable/disable it.
#[utoipa::path(
    patch, path = "/api/links/{id}", tag = "links",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Link id")),
    request_body = UpdateLink,
    responses(
        (status = 200, body = Link),
        (status = 404, body = ErrorBody),
        (status = 422, body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    ValidatedJson(input): ValidatedJson<UpdateLink>,
) -> AppResult<Json<Link>> {
    let row = sqlx::query_as::<_, LinkRow>(
        "UPDATE links
         SET target_url = COALESCE(?, target_url),
             is_active  = COALESCE(?, is_active),
             updated_at = ?
         WHERE id = ? AND user_id = ?
         RETURNING *",
    )
    .bind(input.target_url)
    .bind(input.is_active)
    .bind(Utc::now())
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(Link::from_row(row, &state.config.base_url)))
}

/// Delete a link and its click history.
#[utoipa::path(
    delete, path = "/api/links/{id}", tag = "links",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Link id")),
    responses((status = 204, description = "Deleted"), (status = 404, body = ErrorBody))
)]
pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let result = sqlx::query("DELETE FROM links WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user.id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Click analytics for one of your links.
#[utoipa::path(
    get, path = "/api/links/{id}/stats", tag = "links",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Link id")),
    responses((status = 200, body = LinkStats), (status = 404, body = ErrorBody))
)]
pub async fn stats(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<LinkStats>> {
    let link = find_owned(&state, user.id, id).await?;

    let last_clicked_at =
        sqlx::query_scalar("SELECT MAX(clicked_at) FROM clicks WHERE link_id = ?")
            .bind(link.id)
            .fetch_one(&state.db)
            .await?;

    // Timestamps are stored as RFC 3339 text, so the first 10 chars are the UTC date.
    let since = Utc::now() - Duration::days(30);
    let daily = sqlx::query_as::<_, DailyClicks>(
        "SELECT substr(clicked_at, 1, 10) AS date, COUNT(*) AS clicks
         FROM clicks WHERE link_id = ? AND clicked_at >= ?
         GROUP BY date ORDER BY date",
    )
    .bind(link.id)
    .bind(since)
    .fetch_all(&state.db)
    .await?;

    let top_referrers = sqlx::query_as::<_, ReferrerClicks>(
        "SELECT referrer, COUNT(*) AS clicks
         FROM clicks WHERE link_id = ?
         GROUP BY referrer ORDER BY clicks DESC, referrer LIMIT 5",
    )
    .bind(link.id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(LinkStats {
        link_id: link.id,
        total_clicks: link.click_count,
        last_clicked_at,
        daily,
        top_referrers,
    }))
}
