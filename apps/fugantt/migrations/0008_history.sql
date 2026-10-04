-- What changed, when, and by whom. Written alongside every mutation rather
-- than derived afterwards: the previous value is only knowable at the moment
-- it is replaced.
CREATE TABLE changes (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    task_id    TEXT,
    -- Kept as text so a deleted task's history still says what it was about.
    task_name  TEXT NOT NULL DEFAULT '',
    action     TEXT NOT NULL,
    field      TEXT NOT NULL DEFAULT '',
    before     TEXT NOT NULL DEFAULT '',
    after      TEXT NOT NULL DEFAULT '',
    actor      TEXT NOT NULL DEFAULT '',
    at         INTEGER NOT NULL
);

CREATE INDEX changes_project ON changes (project_id, id DESC);
