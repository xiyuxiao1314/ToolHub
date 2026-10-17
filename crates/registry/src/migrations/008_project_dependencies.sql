CREATE TABLE project_dependencies (
 project_root TEXT NOT NULL, package_name TEXT NOT NULL,
 instance_id TEXT REFERENCES tool_instances(id) ON DELETE SET NULL,
 observation_json TEXT NOT NULL, observed_at TEXT NOT NULL,
 PRIMARY KEY(project_root,package_name)
);
CREATE INDEX project_dependencies_instance ON project_dependencies(instance_id);
