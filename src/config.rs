use std::env;

/// Runtime configuration, loaded from environment variables (and `.env` in development).
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_ttl_hours: i64,
    /// Public base URL used to build short links, without trailing slash.
    pub base_url: String,
    pub port: u16,
    /// Directory with the built dashboard (`web/dist`). When set, the API also serves it.
    pub static_dir: Option<String>,
    /// Seed a demo account on startup and expose its credentials at `/api/demo`.
    pub demo_mode: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let jwt_secret =
            env::var("JWT_SECRET").map_err(|_| "JWT_SECRET must be set".to_string())?;
        if jwt_secret.len() < 32 {
            return Err("JWT_SECRET must be at least 32 characters long".into());
        }

        Ok(Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://snip.db".into()),
            jwt_secret,
            jwt_ttl_hours: parse_or("JWT_TTL_HOURS", 24)?,
            // Render sets RENDER_EXTERNAL_URL to the service's public URL.
            base_url: env::var("BASE_URL")
                .or_else(|_| env::var("RENDER_EXTERNAL_URL"))
                .unwrap_or_else(|_| "http://localhost:8080".into())
                .trim_end_matches('/')
                .to_string(),
            port: parse_or("PORT", 8080)?,
            static_dir: env::var("STATIC_DIR").ok().filter(|dir| !dir.is_empty()),
            demo_mode: parse_or("DEMO_MODE", false)?,
        })
    }
}

fn parse_or<T: std::str::FromStr>(key: &str, default: T) -> Result<T, String> {
    match env::var(key) {
        Ok(raw) => raw
            .parse()
            .map_err(|_| format!("{key} has an invalid value: {raw}")),
        Err(_) => Ok(default),
    }
}
