-- 選択肢の色。
--
-- A master list is a set of states as much as the ステータス column is, and the
-- same reading applies: a colour is faster to scan than a word. The `kind`
-- check goes with it — the list of kinds is now the app's business, not the
-- schema's, and SQLite cannot alter a constraint in place.
ALTER TABLE project_field_options ADD COLUMN color TEXT NOT NULL DEFAULT '';
ALTER TABLE project_field_options ADD COLUMN background TEXT NOT NULL DEFAULT '';

CREATE TABLE project_fields_new (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    label      TEXT NOT NULL,
    kind       TEXT NOT NULL,
    sort_key   TEXT NOT NULL
);

INSERT INTO project_fields_new (id, project_id, label, kind, sort_key)
    SELECT id, project_id, label, kind, sort_key FROM project_fields;

DROP TABLE project_fields;
ALTER TABLE project_fields_new RENAME TO project_fields;

CREATE INDEX project_fields_project ON project_fields (project_id, sort_key);
