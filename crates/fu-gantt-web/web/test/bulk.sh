#!/bin/sh
#
# 計測用の、大きい使い捨てプロジェクトを作る。
#
#   sh test/bulk.sh <db> <owner email> 500
#
# 1年に散らした N 件、3人で回した担当者、1/3 は実施開始あり。
# 名前は「負荷テスト N」なので、seed.sh を回せば消える。
set -e
DB="$1"; EMAIL="$2"; N="$3"
USER_ID=$(sqlite3 "$DB" "SELECT id FROM users WHERE email = '$EMAIL'")
PID="load-$N"

sqlite3 "$DB" "DELETE FROM tasks WHERE project_id = '$PID';
               DELETE FROM project_members WHERE project_id = '$PID';
               DELETE FROM project_settings WHERE project_id = '$PID';
               DELETE FROM projects WHERE id = '$PID';
               INSERT INTO projects (id, name, owner_id, revision, created_at, updated_at)
                 VALUES ('$PID', '負荷テスト $N', '$USER_ID', 0, strftime('%s','now'), strftime('%s','now'));
               INSERT INTO project_members (project_id, user_id, role) VALUES ('$PID', '$USER_ID', 'owner');"

# 1年に散らした N 件。1割は親、担当者は3人で回す。
python3 - "$DB" "$PID" "$N" <<'PY'
import sqlite3, sys, datetime, random
db, pid, n = sys.argv[1], sys.argv[2], int(sys.argv[3])
con = sqlite3.connect(db)
random.seed(1)
start = datetime.date(2026, 4, 1)
rows = []
people = ["山田", "佐藤", ""]

# 並び順の鍵は a〜z しか使わない（src/sortkey.rs）。数字で埋めると、その計画に
# 行を1つ足しただけでサーバーが落ちる——測る前に、測る道具が嘘をつく。
def key(i):
    out = ""
    for _ in range(5):
        i, digit = divmod(i, 26)
        out = chr(ord("a") + digit) + out
    return out

for i in range(n):
    at = start + datetime.timedelta(days=(i * 300) // max(n - 1, 1))
    end = at + datetime.timedelta(days=random.randint(2, 20))
    rows.append((
        f"{pid}-t{i:05d}", pid, None, key(i), f"タスク {i + 1}",
        at.isoformat(), end.isoformat(),
        at.isoformat() if i % 3 else None, None,
        random.choice([0, 10, 40, 60, 100]), "", "未着手", people[i % 3], "", "", "", "", "",
        int(datetime.datetime.now().timestamp()),
    ))
con.executemany(
    """INSERT INTO tasks (id, project_id, parent_id, sort_key, name, start_date, end_date,
                          actual_start, actual_end, progress, tags, status, assignee, note,
                          waits, targets, color, background, updated_at)
       VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)""", rows)
con.commit()
print(f"{pid}: {n} 件")
PY
