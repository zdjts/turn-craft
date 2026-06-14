CREATE TABLE IF NOT EXISTS users (
    id              TEXT PRIMARY KEY NOT NULL,
    username        TEXT UNIQUE NOT NULL,
    password_hash   TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS rooms (
    room_id         TEXT PRIMARY KEY NOT NULL,
    owner_id        TEXT NOT NULL REFERENCES users(id),
    game_type       TEXT NOT NULL,
    engine_state    TEXT NOT NULL,
    actor_slots     TEXT NOT NULL,
    ai_configs      TEXT NOT NULL DEFAULT '{}',
    max_round       INTEGER NOT NULL DEFAULT 16,
    game_config     TEXT NOT NULL DEFAULT '{}',
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS ai_configs (
    room_id         TEXT NOT NULL,
    actor_id        TEXT NOT NULL,
    api_key         TEXT NOT NULL DEFAULT '',
    base_url        TEXT NOT NULL DEFAULT '',
    model           TEXT NOT NULL DEFAULT '',
    max_tokens      INTEGER NOT NULL DEFAULT 2048,
    prompt          TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (room_id, actor_id),
    FOREIGN KEY (room_id) REFERENCES rooms(room_id) ON DELETE CASCADE
);
