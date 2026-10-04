-- Timestamps are unix seconds; task dates are 'YYYY-MM-DD' text so they sort
-- lexicographically and survive a round trip through markwhen unchanged.

CREATE TABLE users (
    id            TEXT PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    INTEGER NOT NULL
);

-- Topcoat issues the session token and hands us its hash; where the record
-- lives is ours to decide.
CREATE TABLE sessions (
    token_hash BLOB PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    expires_at INTEGER NOT NULL
);

CREATE INDEX sessions_user ON sessions (user_id);

CREATE TABLE projects (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    owner_id   TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- Bumped on every mutation so clients can tell whether they are stale.
    revision   INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX projects_owner ON projects (owner_id);

CREATE TABLE project_members (
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    user_id    TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role       TEXT NOT NULL CHECK (role IN ('owner', 'editor', 'viewer')),
    PRIMARY KEY (project_id, user_id)
);

CREATE INDEX project_members_user ON project_members (user_id);

CREATE TABLE tasks (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    parent_id  TEXT REFERENCES tasks (id) ON DELETE CASCADE,
    -- Fractional index: inserting or moving a row is a single UPDATE, never a
    -- renumbering of its siblings.
    sort_key   TEXT NOT NULL,
    name       TEXT NOT NULL DEFAULT '',
    start_date TEXT,
    end_date   TEXT,
    progress   INTEGER NOT NULL DEFAULT 0 CHECK (progress BETWEEN 0 AND 100),
    -- Space-separated markwhen tags, kept so export can give them back.
    tags       TEXT NOT NULL DEFAULT '',
    updated_at INTEGER NOT NULL,
    updated_by TEXT REFERENCES users (id) ON DELETE SET NULL
);

CREATE INDEX tasks_project_sort ON tasks (project_id, sort_key);
CREATE INDEX tasks_parent ON tasks (parent_id);
