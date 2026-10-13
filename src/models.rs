//! Database rows and request/response bodies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

// ---------- Users ----------

#[derive(Debug, sqlx::FromRow)]
pub struct UserRow {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct User {
    pub id: i64,
    #[schema(example = "ada@example.com")]
    pub email: String,
    pub created_at: DateTime<Utc>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            created_at: row.created_at,
        }
    }
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct Credentials {
    #[validate(email(message = "must be a valid email address"))]
    #[schema(example = "ada@example.com")]
    pub email: String,
    #[validate(length(min = 8, max = 128, message = "must be between 8 and 128 characters"))]
    #[schema(example = "correct-horse-battery")]
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct AuthResponse {
    /// JWT to send as `Authorization: Bearer <token>`.
    pub token: String,
    pub user: User,
}

// ---------- Links ----------

#[derive(Debug, sqlx::FromRow)]
pub struct LinkRow {
    pub id: i64,
    pub user_id: i64,
    pub code: String,
    pub target_url: String,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub click_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LinkRow {
    /// A link redirects only while it is active and not past its expiry date.
    pub fn is_available(&self, now: DateTime<Utc>) -> bool {
        self.is_active && self.expires_at.is_none_or(|exp| exp > now)
    }
}

#[derive(Serialize, ToSchema)]
pub struct Link {
    pub id: i64,
    #[schema(example = "rust-book")]
    pub code: String,
    #[schema(example = "http://localhost:8080/rust-book")]
    pub short_url: String,
    #[schema(example = "https://doc.rust-lang.org/book/")]
    pub target_url: String,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub click_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Link {
    pub fn from_row(row: LinkRow, base_url: &str) -> Self {
        Self {
            short_url: format!("{base_url}/{}", row.code),
            id: row.id,
            code: row.code,
            target_url: row.target_url,
            is_active: row.is_active,
            expires_at: row.expires_at,
            click_count: row.click_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateLink {
    /// Destination URL (http or https).
    #[validate(custom(function = "crate::validation::http_url"))]
    #[schema(example = "https://doc.rust-lang.org/book/")]
    pub target_url: String,
    /// Optional custom code (3-32 chars: letters, digits, `-`, `_`).
    /// A random 7-character code is generated when omitted.
    #[validate(custom(function = "crate::validation::short_code"))]
    #[schema(example = "rust-book")]
    pub code: Option<String>,
    /// Optional expiry date; the link returns `410 Gone` afterwards.
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct UpdateLink {
    #[validate(custom(function = "crate::validation::http_url"))]
    pub target_url: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct Pagination {
    /// Page number, starting at 1.
    #[param(default = 1, minimum = 1)]
    pub page: Option<u32>,
    /// Items per page (max 100).
    #[param(default = 20, minimum = 1, maximum = 100)]
    pub per_page: Option<u32>,
}

impl Pagination {
    /// Returns `(page, per_page)` clamped to sane bounds.
    pub fn resolve(&self) -> (u32, u32) {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page.unwrap_or(20).clamp(1, 100);
        (page, per_page)
    }
}

#[derive(Serialize, ToSchema)]
pub struct LinkPage {
    pub items: Vec<Link>,
    pub page: u32,
    pub per_page: u32,
    pub total: i64,
}

// ---------- Stats ----------

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct DailyClicks {
    #[schema(example = "2026-10-13")]
    pub date: String,
    pub clicks: i64,
}

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct ReferrerClicks {
    /// `null` means direct traffic (no `Referer` header).
    pub referrer: Option<String>,
    pub clicks: i64,
}

#[derive(Serialize, ToSchema)]
pub struct LinkStats {
    pub link_id: i64,
    pub total_clicks: i64,
    pub last_clicked_at: Option<DateTime<Utc>>,
    /// Clicks per day over the last 30 days (days without clicks are omitted).
    pub daily: Vec<DailyClicks>,
    /// Top 5 referrers.
    pub top_referrers: Vec<ReferrerClicks>,
}
