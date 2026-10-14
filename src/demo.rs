//! Demo account seeded on startup when `DEMO_MODE=true`.
//!
//! Free hosting tiers often wipe the filesystem on restart, so a fresh
//! SQLite database would be empty. Seeding gives visitors an account with
//! realistic links and 30 days of clicks to explore.

use chrono::{Duration, Utc};
use sqlx::SqlitePool;

use crate::{auth, error::AppResult};

pub const EMAIL: &str = "demo@snip.dev";
pub const PASSWORD: &str = "demo-password";

struct DemoLink {
    code: &'static str,
    target_url: &'static str,
    clicks: u32,
    is_active: bool,
}

const LINKS: &[DemoLink] = &[
    DemoLink {
        code: "react-learn",
        target_url: "https://react.dev/learn",
        clicks: 0,
        is_active: true,
    },
    DemoLink {
        code: "tokio-spawn",
        target_url: "https://tokio.rs/tokio/tutorial/spawning",
        clicks: 22,
        is_active: false,
    },
    DemoLink {
        code: "axum",
        target_url: "https://docs.rs/axum/latest/axum/",
        clicks: 35,
        is_active: true,
    },
    DemoLink {
        code: "source",
        target_url: "https://github.com/ebarrera2019263/snip-api",
        clicks: 60,
        is_active: true,
    },
    DemoLink {
        code: "rust-book",
        target_url: "https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html",
        clicks: 140,
        is_active: true,
    },
];

/// `(referrer, weight)`; `None` is direct traffic.
const REFERRERS: &[(Option<&str>, u64)] = &[
    (Some("https://www.linkedin.com/"), 5),
    (None, 4),
    (Some("https://twitter.com/"), 3),
    (Some("https://github.com/"), 2),
    (Some("https://news.ycombinator.com/"), 1),
];

/// Tiny deterministic PRNG (PCG-style LCG): the demo data looks the same on
/// every boot, and we avoid a `rand` dependency for this one use.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// Uniform float in `[0, 1)`.
    fn unit(&mut self) -> f64 {
        self.next() as f64 / (1u64 << 31) as f64
    }
}

/// Creates the demo account and its data, unless it already exists.
pub async fn seed(pool: &SqlitePool) -> AppResult<()> {
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE email = ?")
        .bind(EMAIL)
        .fetch_optional(pool)
        .await?;
    if exists.is_some() {
        return Ok(());
    }

    let password_hash = auth::hash_password(PASSWORD.to_string()).await?;
    let now = Utc::now();
    let mut rng = Lcg(0x5eed);
    let referrer_total: u64 = REFERRERS.iter().map(|(_, w)| w).sum();

    let mut tx = pool.begin().await?;
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, created_at) VALUES (?, ?, ?) RETURNING id",
    )
    .bind(EMAIL)
    .bind(password_hash)
    .bind(now - Duration::days(35))
    .fetch_one(&mut *tx)
    .await?;

    // Inserted oldest first so the newest link shows on top of the list.
    for (i, link) in LINKS.iter().rev().enumerate() {
        let created_at = now - Duration::days(34 - i as i64);
        let link_id: i64 = sqlx::query_scalar(
            "INSERT INTO links
                (user_id, code, target_url, is_active, click_count, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(user_id)
        .bind(link.code)
        .bind(link.target_url)
        .bind(link.is_active)
        .bind(i64::from(link.clicks))
        .bind(created_at)
        .bind(created_at)
        .fetch_one(&mut *tx)
        .await?;

        for _ in 0..link.clicks {
            // Mostly recent clicks, plus a spike six days ago (a "launch post").
            let days_ago = if rng.below(4) == 0 {
                6
            } else {
                (rng.unit().powi(2) * 30.0) as i64
            };
            let clicked_at =
                now - Duration::days(days_ago) - Duration::seconds(rng.below(80_000) as i64);

            let mut pick = rng.below(referrer_total);
            let referrer = REFERRERS
                .iter()
                .find(|(_, weight)| {
                    if pick < *weight {
                        true
                    } else {
                        pick -= weight;
                        false
                    }
                })
                .and_then(|(referrer, _)| *referrer);

            sqlx::query(
                "INSERT INTO clicks (link_id, clicked_at, referrer, user_agent) VALUES (?, ?, ?, ?)",
            )
            .bind(link_id)
            .bind(clicked_at)
            .bind(referrer)
            .bind("Mozilla/5.0 (demo)")
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}
