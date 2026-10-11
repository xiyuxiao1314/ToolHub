CREATE TABLE extension_packages (
 id TEXT PRIMARY KEY, package_json TEXT NOT NULL, digest TEXT NOT NULL,
 source TEXT NOT NULL, enabled INTEGER NOT NULL CHECK(enabled IN (0,1)),
 previous_json TEXT, previous_digest TEXT, installed_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
-- Retained on uninstall to preserve provenance of disabled installation records.
CREATE TABLE extension_definitions (definition_id TEXT PRIMARY KEY, extension_id TEXT NOT NULL);
CREATE INDEX extension_definition_owner ON extension_definitions(extension_id);
