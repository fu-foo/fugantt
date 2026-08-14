-- 担当者の色。
--
-- Who is on a plan comes from its members and from the names already typed on
-- tasks, so this table is not the list itself — it is what those names look
-- like, plus any name that belongs on the list without belonging to an account
-- (協力会社, 他部署).
CREATE TABLE project_assignees (
  project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
  name       TEXT NOT NULL,
  color      TEXT NOT NULL DEFAULT '',
  background TEXT NOT NULL DEFAULT '',
  PRIMARY KEY (project_id, name)
);
