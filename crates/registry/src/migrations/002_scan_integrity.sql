CREATE TABLE scan_membership(instance_id TEXT NOT NULL REFERENCES tool_instances(id) ON DELETE CASCADE, provider TEXT NOT NULL, root TEXT NOT NULL, PRIMARY KEY(instance_id,provider,root));
CREATE TABLE settings(key TEXT PRIMARY KEY, value_json TEXT NOT NULL);
CREATE TABLE authorization_revision(id INTEGER PRIMARY KEY CHECK(id=1), revision INTEGER NOT NULL);
INSERT INTO authorization_revision VALUES(1,1);
CREATE TABLE discovery_disclosures(session_id TEXT NOT NULL REFERENCES discovery_sessions(id),candidate_id TEXT NOT NULL REFERENCES scan_candidates(id), PRIMARY KEY(session_id,candidate_id));
