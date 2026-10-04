-- Key/value rather than columns: the next thing anyone wants to configure
-- should not need a migration.
CREATE TABLE project_settings (
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    key        TEXT NOT NULL,
    value      TEXT NOT NULL,
    PRIMARY KEY (project_id, key)
);
