-- Tokens that let something other than a browser read and write one project.
--
-- The point is the loop a person cannot do by hand: read the plan, work out
-- what should change, write it back. Scoped to a single project on purpose —
-- a token that can reach everything is a key nobody can safely hand out.
CREATE TABLE api_tokens (
  id         TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
  -- What it is for, in the owner's words. Shown in the list.
  name       TEXT NOT NULL DEFAULT '',
  -- `viewer` reads, `editor` reads and writes.
  role       TEXT NOT NULL DEFAULT 'viewer',
  -- The SHA-256 of the token. The token itself is shown once and never stored,
  -- so a copy of this database is not a set of working keys.
  token_hash BLOB NOT NULL,
  created_at INTEGER NOT NULL,
  created_by TEXT NOT NULL DEFAULT '',
  -- So a forgotten token can be told from one still in use.
  last_used  INTEGER
);

CREATE INDEX api_tokens_hash ON api_tokens (token_hash);
CREATE INDEX api_tokens_project ON api_tokens (project_id);
