CREATE TABLE artifact_roots (
  id TEXT PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  enabled INTEGER NOT NULL CHECK(enabled IN (0,1))
);
CREATE TABLE artifact_packages (
  id TEXT PRIMARY KEY,
  owner TEXT NOT NULL,
  package_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE artifact_events (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  package_id TEXT NOT NULL,
  at TEXT NOT NULL,
  kind TEXT NOT NULL,
  data_json TEXT NOT NULL
);
CREATE INDEX artifact_packages_owner ON artifact_packages(owner);
