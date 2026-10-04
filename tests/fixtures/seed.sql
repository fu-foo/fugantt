-- The plan the golden tests read. Every date is fixed; the tests pin today to
-- 2026-09-15, so "late" means the same thing on every run.

INSERT INTO projects (id, name, owner_id, revision, created_at, updated_at)
  VALUES ('golden', 'リリース計画', 'shared-account', 7, 1788000000, 1788000000),
         ('empty',  '空の計画',     'shared-account', 0, 1788000000, 1788000000);

INSERT INTO project_members (project_id, user_id, role)
  VALUES ('golden', 'shared-account', 'owner'),
         ('empty',  'shared-account', 'owner');

INSERT INTO tasks (id, project_id, parent_id, sort_key, name, start_date, end_date,
                   actual_start, actual_end, status, waits, targets,
                   progress, assignee, note, updated_at) VALUES
  ('t-req',  'golden', NULL,    'n', '要件定義',         '2026-08-03', '2026-08-14',
   '2026-08-03', '2026-08-18', '完了',   '', '2026-08-14/100',
   100, '山田', '', 1788000000),
  ('t-dev',  'golden', NULL,    'o', '開発',             NULL, NULL,
   NULL, NULL, '未着手', '', '',
   0, '', '', 1788000000),
  ('t-des',  'golden', 't-dev', 'n', '設計',             '2026-08-10', '2026-08-28',
   '2026-08-12', NULL, '待ち',
   '2026-08-17/2026-08-21:他部署の回答待ち' || char(10) || '2026-09-10/',
   '2026-08-20/50' || char(10) || '2026-10-15/90',
   60, '佐藤', '画面は別紙', 1788000000),
  ('t-imp',  'golden', 't-dev', 'o', '実装',             '2026-08-24', '2026-09-25',
   NULL, NULL, '未着手', '', '',
   10, '佐藤', '', 1788000000),
  ('t-test', 'golden', NULL,    'p', 'テスト',           '2026-09-21', '2026-10-09',
   NULL, NULL, '未着手', '', '',
   0, '山田', '', 1788000000),
  ('t-doc',  'golden', NULL,    'q', 'ドキュメント整備', '2026-08-01', '2026-08-20',
   NULL, NULL, '進行中', '', '2026-08-27/50',
   5, '', '', 1788000000),
  ('t-due',  'golden', NULL,    'r', '納品',             NULL, NULL,
   NULL, NULL, '未着手', '', '',
   0, '山田', '', 1788000000);

UPDATE tasks SET due = '2026-10-16' WHERE id = 't-due';
UPDATE tasks SET color = '#b91c1c', background = '#fee2e2' WHERE id = 't-test';

INSERT INTO leaves (id, assignee, start_date, end_date, note, kind, created_at) VALUES
  ('l-1', '佐藤', '2026-09-07', '2026-09-09', '夏休み', 'off', 1788000000),
  ('l-2', '山田', '2026-09-19', '2026-09-19', '休日出勤', 'on', 1788000000);

INSERT INTO project_holidays (project_id, date, name, kind) VALUES
  ('golden', '2026-09-24', '創立記念日', 'add');

INSERT INTO project_fields (id, project_id, label, kind, sort_key) VALUES
  ('f-ticket', 'golden', 'チケット', 'text', 'n');

INSERT INTO task_field_values (task_id, field_id, value) VALUES
  ('t-imp', 'f-ticket', 'DEV-102');

INSERT INTO project_settings (project_id, key, value) VALUES
  ('golden', 'fiscal_year_start', '4'),
  ('golden', 'japanese_era', '1');
