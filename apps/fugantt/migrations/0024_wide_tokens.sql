-- Tokens that reach more than one project.
--
-- A per-project token is the right default: a key that opens everything is a
-- key nobody can hand out. But numbers across projects — which plans are late,
-- and by how much — cannot be gathered one key at a time.
--
-- `project_id` NULL means every project. Only an administrator can make one.
CREATE TABLE api_tokens_new (
  id         TEXT PRIMARY KEY,
  project_id TEXT REFERENCES projects (id) ON DELETE CASCADE,
  name       TEXT NOT NULL DEFAULT '',
  role       TEXT NOT NULL DEFAULT 'viewer',
  token_hash BLOB NOT NULL,
  created_at INTEGER NOT NULL,
  created_by TEXT NOT NULL DEFAULT '',
  last_used  INTEGER
);

INSERT INTO api_tokens_new (id, project_id, name, role, token_hash, created_at, created_by, last_used)
  SELECT id, project_id, name, role, token_hash, created_at, created_by, last_used FROM api_tokens;

DROP TABLE api_tokens;
ALTER TABLE api_tokens_new RENAME TO api_tokens;

CREATE INDEX api_tokens_hash ON api_tokens (token_hash);
CREATE INDEX api_tokens_project ON api_tokens (project_id);
