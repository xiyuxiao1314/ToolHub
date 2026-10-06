CREATE TABLE program_proposals (
    id TEXT PRIMARY KEY,
    identity TEXT NOT NULL,
    principal TEXT NOT NULL,
    proposal_json TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(identity, principal)
);
