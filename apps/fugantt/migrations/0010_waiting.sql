-- 「待ち」を状態に加える。SQLite は CHECK 制約を変更できないので、
-- テーブルを作り直す。ついでに制約自体を外す — 値の検証は API 側にあり、
-- そちらのほうが理由を説明できる。
CREATE TABLE tasks_new (
    id           TEXT PRIMARY KEY,
    project_id   TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    parent_id    TEXT REFERENCES tasks_new (id) ON DELETE CASCADE,
    sort_key     TEXT NOT NULL,
    name         TEXT NOT NULL DEFAULT '',
    start_date   TEXT,
    end_date     TEXT,
    actual_start TEXT,
    actual_end   TEXT,
    progress     INTEGER NOT NULL DEFAULT 0 CHECK (progress BETWEEN 0 AND 100),
    tags         TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT '未着手',
    assignee     TEXT NOT NULL DEFAULT '',
    note         TEXT NOT NULL DEFAULT '',
    -- 待ちの詳細。どれも任意で、状態だけで使える。
    wait_reason  TEXT NOT NULL DEFAULT '',
    wait_target  TEXT NOT NULL DEFAULT '',
    wait_start   TEXT,
    wait_until   TEXT,
    updated_at   INTEGER NOT NULL,
    updated_by   TEXT REFERENCES users (id) ON DELETE SET NULL
);

INSERT INTO tasks_new
    (id, project_id, parent_id, sort_key, name, start_date, end_date,
     actual_start, actual_end, progress, tags, status, assignee, note,
     updated_at, updated_by)
SELECT id, project_id, parent_id, sort_key, name, start_date, end_date,
       actual_start, actual_end, progress, tags, status, assignee, note,
       updated_at, updated_by
  FROM tasks;

DROP TABLE tasks;
ALTER TABLE tasks_new RENAME TO tasks;

CREATE INDEX tasks_project_sort ON tasks (project_id, sort_key);
CREATE INDEX tasks_parent ON tasks (parent_id);
