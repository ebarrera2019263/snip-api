use std::sync::Arc;

use sqlx::SqlitePool;

use crate::config::Config;

/// Shared state handed to every handler. Cheap to clone.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
}
