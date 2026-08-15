#!/bin/sh
# Rebuilds a known project so the grid test starts from the same state every
# run. Task ids and sort keys are fixed on purpose.
#
#   ./seed.sh <database> <owner email>

set -e

DB="${1:?usage: seed.sh <database> <owner email>}"
EMAIL="${2:?usage: seed.sh <database> <owner email>}"

USER_ID=$(sqlite3 "$DB" "SELECT id FROM users WHERE email = '$EMAIL'")

if [ -z "$USER_ID" ]; then
  echo "seed: no account for $EMAIL" >&2
  exit 1
fi

sqlite3 "$DB" <<SQL
-- The sqlite3 CLI leaves foreign keys off, so cascades do not fire here.
DELETE FROM tasks WHERE project_id = 'test-project';
DELETE FROM project_members WHERE project_id = 'test-project';
DELETE FROM projects WHERE id = 'test-project';

-- 会社の暦は全体に一つあり、プロジェクトを作り直しても残る。休みが1日あるだけで
-- 日数が変わり、日数で重みを付けている集計まで変わるので、種を蒔き直すときは
-- ここも空にする。前の実行が途中で落ちた日にだけ落ちるテスト、の正体だった。
-- 走らせるたびに増えていく使い捨てのプロジェクト。名前で作った ID なので、
-- 名前で片付けられる。
DELETE FROM project_members WHERE project_id IN
  (SELECT id FROM projects WHERE name LIKE 'テスト計画 %' OR name LIKE '全体テスト %' OR name LIKE '既定テスト %' OR name LIKE '空行テスト %' OR name LIKE '横断テスト %');
DELETE FROM project_statuses WHERE project_id IN
  (SELECT id FROM projects WHERE name LIKE 'テスト計画 %' OR name LIKE '全体テスト %' OR name LIKE '既定テスト %' OR name LIKE '空行テスト %' OR name LIKE '横断テスト %');
DELETE FROM projects WHERE name LIKE 'テスト計画 %' OR name LIKE '全体テスト %' OR name LIKE '既定テスト %' OR name LIKE '空行テスト %' OR name LIKE '横断テスト %';

DELETE FROM leaves;
DELETE FROM app_holidays;
DELETE FROM project_holidays WHERE project_id = 'test-project';

INSERT INTO projects (id, name, owner_id, revision, created_at, updated_at)
  VALUES ('test-project', 'リリース計画', '$USER_ID', 0, strftime('%s','now'), strftime('%s','now'));

INSERT INTO project_members (project_id, user_id, role)
  VALUES ('test-project', '$USER_ID', 'owner');

-- 要件定義 finished four days past its plan, and 設計 is sitting in a wait, so
-- the plan/actual bars and the delay breakdown have something to show.
INSERT INTO tasks (id, project_id, parent_id, sort_key, name, start_date, end_date,
                   actual_start, actual_end, status, wait_reason, wait_target,
                   wait_start, wait_until, progress, assignee, updated_at) VALUES
  ('t-req',  'test-project', NULL,    'n', '要件定義',        '2026-08-03', '2026-08-14',
   '2026-08-03', '2026-08-18', '完了',   '',       '',       NULL,         NULL,         100, '山田', strftime('%s','now')),
  ('t-dev',  'test-project', NULL,    'o', '開発',            NULL,         NULL,
   NULL,         NULL,         '未着手', '',       '',       NULL,         NULL,           0, '',     strftime('%s','now')),
  ('t-des',  'test-project', 't-dev', 'n', '設計',            '2026-08-10', '2026-08-28',
   '2026-08-12', NULL,         '待ち',   '他部署', '情シス', '2026-08-18', '2026-08-24',  60, '佐藤', strftime('%s','now')),
  ('t-imp',  'test-project', 't-dev', 'o', '実装',            '2026-08-24', '2026-09-25',
   NULL,         NULL,         '未着手', '',       '',       NULL,         NULL,          10, '佐藤', strftime('%s','now')),
  ('t-test', 'test-project', NULL,    'p', 'テスト',          '2026-09-21', '2026-10-09',
   NULL,         NULL,         '未着手', '',       '',       NULL,         NULL,           0, '山田', strftime('%s','now')),
  ('t-doc',  'test-project', NULL,    'q', 'ドキュメント整備', '2026-08-01', '2026-08-20',
   NULL,         NULL,         '実施中', '',       '',       NULL,         NULL,           5, '',     strftime('%s','now')),
  ('t-rev',  'test-project', NULL,    'r', 'レビュー',        '2026-07-27', '2026-08-06',
   '2026-07-27', NULL,         '待ち',   '顧客',   'A社',   '2026-08-03', '2026-08-12',  40, '山田', strftime('%s','now'));

-- 予定進捗. Nothing is behind unless the plan itself said what it wanted by
-- when, so the fixture has to say it: one promise kept, one missed, one still
-- to come, and rows that promised nothing at all.
UPDATE tasks SET targets = '2026-08-14/100' WHERE id = 't-req';
UPDATE tasks SET targets = '2026-08-12/50
2026-08-24/90' WHERE id = 't-des';
UPDATE tasks SET targets = '2026-08-05/50' WHERE id = 't-doc';
SQL
