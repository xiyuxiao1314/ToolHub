-- User-selected launch entries are distinct from recognized tool installations.
CREATE TABLE program_entries (
    id TEXT PRIMARY KEY,
    identity TEXT NOT NULL UNIQUE,
    entry_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
