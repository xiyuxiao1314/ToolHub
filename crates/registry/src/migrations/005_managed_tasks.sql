CREATE TABLE managed_tasks (
 id TEXT PRIMARY KEY, owner TEXT NOT NULL, view_json TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE INDEX managed_tasks_owner ON managed_tasks(owner,updated_at);
