use std::str::FromStr;

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

/// Opens the SQLite pool and runs pending migrations.
pub async fn connect(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let in_memory = database_url.contains(":memory:");

    let mut options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true);
    if !in_memory {
        options = options.journal_mode(SqliteJournalMode::Wal);
    }

    // Every connection to `:memory:` is a different database, so keep a single one.
    let pool = SqlitePoolOptions::new()
        .max_connections(if in_memory { 1 } else { 5 })
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
