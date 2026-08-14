-- プロジェクトごとのステータス。
--
-- The five built-in values were a guess at what every team calls its states;
-- teams call them different things, colour them differently, and disagree about
-- what percentage "完了" means. So the list is data, per project.
CREATE TABLE project_statuses (
  project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
  position   INTEGER NOT NULL,
  name       TEXT NOT NULL,
  color      TEXT NOT NULL DEFAULT '#f1f5f9',
  -- The progress this state implies, when it implies one at all.
  percent    INTEGER,
  PRIMARY KEY (project_id, name)
);
