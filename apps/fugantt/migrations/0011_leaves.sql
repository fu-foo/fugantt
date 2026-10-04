-- 担当者ごとの休暇。
--
-- Kept per project and keyed by the assignee's name rather than a user id: the
-- 担当者 column is free text, and a plan often names people who have no account
-- here (協力会社, 他部署).
CREATE TABLE assignee_leaves (
  id         TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
  assignee   TEXT NOT NULL,
  start_date TEXT NOT NULL,
  end_date   TEXT NOT NULL,
  note       TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL
);

CREATE INDEX assignee_leaves_project ON assignee_leaves (project_id, assignee);
