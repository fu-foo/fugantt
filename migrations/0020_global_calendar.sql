-- 会社の事実は全体に、計画の事実はプロジェクトに。
--
-- 祝日・担当者の色・休暇は、どのプロジェクトから見ても同じもののはずだった。
-- プロジェクトごとに持たせると、新しい計画を作るたびに貼り直すことになり、貼り
-- 直されなかったほうが静かに間違う。

-- 祝日の本体。プロジェクトはここからの差分だけを持つ。
CREATE TABLE app_holidays (
  -- 'YYYY-MM-DD'
  date TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT ''
);

-- 'add'  … このプロジェクトだけの休み（全体には無い日）
-- 'skip' … 全体では休みだが、この現場は動く日
--
-- 既存の行はすべて 'add'。全体の一覧は空から始まるので、いまあるプロジェクトの
-- 見え方は 1 日も変わらない。
ALTER TABLE project_holidays ADD COLUMN kind TEXT NOT NULL DEFAULT 'add';

-- 担当者の名簿と色。アカウントの無い名前（協力会社, 他部署）もここに載る。
CREATE TABLE assignees (
  name       TEXT PRIMARY KEY,
  color      TEXT NOT NULL DEFAULT '',
  background TEXT NOT NULL DEFAULT ''
);

-- 同じ名前が複数のプロジェクトで別の色を持っていたら、色の付いているほうを採る
-- （MAX は空文字より実際の値を選ぶ）。一度きりの合流なので、これで足りる。
INSERT INTO assignees (name, color, background)
  SELECT name, MAX(color), MAX(background) FROM project_assignees GROUP BY name;

-- プロジェクトが持つのは「この計画に誰がいるか」だけ。
CREATE TABLE project_assignees_new (
  project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
  name       TEXT NOT NULL,
  PRIMARY KEY (project_id, name)
);

INSERT INTO project_assignees_new (project_id, name)
  SELECT project_id, name FROM project_assignees;

DROP TABLE project_assignees;
ALTER TABLE project_assignees_new RENAME TO project_assignees;

-- 休暇と出社。人が休むのはプロジェクトの都合ではない。
CREATE TABLE leaves (
  id         TEXT PRIMARY KEY,
  assignee   TEXT NOT NULL,
  start_date TEXT NOT NULL,
  end_date   TEXT NOT NULL,
  note       TEXT NOT NULL DEFAULT '',
  kind       TEXT NOT NULL DEFAULT 'off',
  created_at INTEGER NOT NULL
);

-- 同じ人の同じ期間が複数のプロジェクトに入っていれば、それは同じ休みなので 1 件に。
INSERT INTO leaves (id, assignee, start_date, end_date, note, kind, created_at)
  SELECT id, assignee, start_date, end_date, note, kind, created_at
    FROM assignee_leaves
   GROUP BY assignee, start_date, end_date, kind;

DROP TABLE assignee_leaves;

CREATE INDEX leaves_assignee ON leaves (assignee, start_date);
