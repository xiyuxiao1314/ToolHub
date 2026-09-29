-- ToolHub registry initial schema
CREATE TABLE IF NOT EXISTS tool_definitions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    vendor TEXT,
    categories_json TEXT NOT NULL DEFAULT '[]',
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS environments (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    root_path TEXT,
    parent_id TEXT REFERENCES environments(id),
    origin_json TEXT NOT NULL DEFAULT '{}',
    owner_json TEXT NOT NULL DEFAULT '{}',
    labels_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS tool_instances (
    id TEXT PRIMARY KEY,
    definition_id TEXT NOT NULL REFERENCES tool_definitions(id),
    version TEXT,
    platform TEXT NOT NULL,
    arch TEXT NOT NULL,
    path TEXT NOT NULL,
    canonical_path TEXT,
    environment_id TEXT REFERENCES environments(id),
    origin_json TEXT NOT NULL DEFAULT '{}',
    owner_json TEXT NOT NULL DEFAULT '{}',
    trust_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'available',
    first_seen TEXT NOT NULL,
    last_seen TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_instances_def_path
    ON tool_instances(definition_id, path);

CREATE TABLE IF NOT EXISTS interfaces (
    id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL REFERENCES tool_instances(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    executable TEXT,
    supports_stdin INTEGER NOT NULL DEFAULT 0,
    supports_stdout INTEGER NOT NULL DEFAULT 0,
    supports_batch INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS capabilities (
    id TEXT PRIMARY KEY,
    description TEXT NOT NULL DEFAULT '',
    is_alias INTEGER NOT NULL DEFAULT 0,
    canonical_id TEXT
);

CREATE TABLE IF NOT EXISTS tool_capabilities (
    definition_id TEXT NOT NULL REFERENCES tool_definitions(id) ON DELETE CASCADE,
    capability_id TEXT NOT NULL,
    PRIMARY KEY (definition_id, capability_id)
);

CREATE TABLE IF NOT EXISTS evidence (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_kind TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    source TEXT NOT NULL,
    provenance TEXT NOT NULL,
    confidence REAL NOT NULL,
    summary TEXT NOT NULL,
    detail TEXT,
    observed_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS scan_candidates (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    canonical_path TEXT,
    file_name TEXT,
    size_bytes INTEGER,
    sha256 TEXT,
    version_hint TEXT,
    recognized INTEGER NOT NULL DEFAULT 0,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    discovered_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS scan_sessions (
    id TEXT PRIMARY KEY,
    mode TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    status TEXT NOT NULL,
    coverage_json TEXT NOT NULL DEFAULT '{}',
    errors_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS skills (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    schema TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'instruction',
    manifest_json TEXT NOT NULL,
    path TEXT
);

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    health TEXT NOT NULL DEFAULT 'unknown',
    config_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS discovery_sessions (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    revoked INTEGER NOT NULL DEFAULT 0,
    scopes_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS execution_records (
    id TEXT PRIMARY KEY,
    agent_id TEXT,
    instance_id TEXT,
    capability TEXT,
    executable TEXT,
    args_redacted TEXT,
    cwd_redacted TEXT,
    started_at TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    exit_code INTEGER,
    status TEXT NOT NULL,
    approval_id TEXT
);

CREATE TABLE IF NOT EXISTS policy_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scope TEXT NOT NULL,
    subject TEXT NOT NULL,
    action TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(scope, subject, action)
);

CREATE TABLE IF NOT EXISTS execution_approvals (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    instance_id TEXT NOT NULL,
    executable_sha256 TEXT NOT NULL,
    canonical_executable TEXT NOT NULL,
    args_digest TEXT NOT NULL,
    cwd_digest TEXT NOT NULL,
    env_digest TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    consumed INTEGER NOT NULL DEFAULT 0,
    revoked INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts TEXT NOT NULL,
    kind TEXT NOT NULL,
    summary TEXT NOT NULL,
    agent TEXT,
    payload_json TEXT NOT NULL DEFAULT '{}'
);
