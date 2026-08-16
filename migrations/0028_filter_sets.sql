-- 絞り込み条件に名前をつけて置いておく。
--
-- 「遅れているものだけ」「自分の担当だけ」は毎日同じ手つきで打ち直されている。
-- 共通（user_id が NULL）はチームの見方、利用者ごとはその人の見方。同じ名前が
-- 両方にあってもよい: 一覧では別の見出しに並ぶ。
CREATE TABLE filter_sets (
  id         TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
  -- NULL は共通。誰のものでもないので、誰でも使えて、消せるのは作った人と管理者。
  user_id    TEXT REFERENCES users (id) ON DELETE CASCADE,
  name       TEXT NOT NULL,
  conditions TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX filter_sets_project ON filter_sets (project_id);
