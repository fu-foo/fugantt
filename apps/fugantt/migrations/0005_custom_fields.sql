-- Project-defined columns. The built-in ones stay as columns on `tasks`:
-- they carry the schedule arithmetic, and every project has them.
CREATE TABLE project_fields (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    label      TEXT NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('text', 'number', 'select')),
    -- Fractional index, same as tasks: reordering columns stays one UPDATE.
    sort_key   TEXT NOT NULL
);

CREATE INDEX project_fields_project ON project_fields (project_id, sort_key);

-- The master list behind a 'select' field.
CREATE TABLE project_field_options (
    field_id TEXT NOT NULL REFERENCES project_fields (id) ON DELETE CASCADE,
    value    TEXT NOT NULL,
    sort_key TEXT NOT NULL,
    PRIMARY KEY (field_id, value)
);

-- Values are held long rather than wide: adding a column must not mean
-- migrating the tasks table.
CREATE TABLE task_field_values (
    task_id  TEXT NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    field_id TEXT NOT NULL REFERENCES project_fields (id) ON DELETE CASCADE,
    value    TEXT NOT NULL,
    PRIMARY KEY (task_id, field_id)
);
