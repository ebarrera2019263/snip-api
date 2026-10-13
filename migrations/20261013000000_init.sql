CREATE TABLE users (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    email         TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT    NOT NULL,
    created_at    TEXT    NOT NULL
);

CREATE TABLE links (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    code        TEXT    NOT NULL UNIQUE,
    target_url  TEXT    NOT NULL,
    is_active   INTEGER NOT NULL DEFAULT 1,
    expires_at  TEXT,
    click_count INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);

CREATE INDEX idx_links_user ON links (user_id, created_at DESC);

CREATE TABLE clicks (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    link_id    INTEGER NOT NULL REFERENCES links (id) ON DELETE CASCADE,
    clicked_at TEXT    NOT NULL,
    referrer   TEXT,
    user_agent TEXT
);

CREATE INDEX idx_clicks_link ON clicks (link_id, clicked_at);
