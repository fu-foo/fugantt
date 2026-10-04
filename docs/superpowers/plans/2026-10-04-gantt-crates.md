# ガントをクレートに切り出す Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** fugantt から祝日の計算・ガントの計算・グリッドの画面を3クレートに切り出し、fugantt 自身の挙動は変えずに v1.1.0 にする。

**Architecture:** リポジトリを Cargo ワークスペースにする。`apps/fugantt` が DB・認証・ページ・API ハンドラーを持ち続け、`crates/fu-calendar`・`crates/fu-gantt-core`・`crates/fu-gantt-web` を使う側に回る。最初に「固定 DB から取った出力」を記録するテストを作り、以降の全タスクでそれが一致し続けることを安全網にする。

**Tech Stack:** Rust 2024 / Topcoat 0.5 / sqlx 0.8 (SQLite) / jiff 0.2 / TypeScript + esbuild / puppeteer-core

**Spec:** `docs/superpowers/specs/2026-10-04-gantt-crates-design.md`

## Global Constraints

- 利用者から見える挙動を変えない: 実行ファイル名 `fugantt`、`FUGANTT_*` と ini、データの置き場所、DB の中身、`migrations/0001〜0034`、API の形、画面
- `sqlx::migrate!()` はそのまま使う。マイグレーションのファイルは内容を1バイトも変えない（チェックサムが変わると既存 DB が起動しなくなる）
- 突き合わせテスト（Task 2 の `tests/golden.rs`）が一致しなくなったら、コードを直すか取り消す。**`UPDATE_GOLDEN=1` で記録を取り直して通すのは Task 2 の中だけ**
- 1タスク = 1コミット（以上）。複数のタスクを1コミットに混ぜない
- コミットメッセージは既存の流儀: 英語の平叙文1行、必要なら本文。末尾に
  `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`
- 切り出すクレートは `version = "0.1.0"`、`publish = false`
- クレート名: `fu-calendar` / `fu-gantt-core` / `fu-gantt-web`（コード中は `fu_calendar` など）
- `fu-gantt-core` と `fu-gantt-web` は Topcoat に依存しない。`fu-calendar` は依存ゼロ（`jiff`・`chrono` は任意フィーチャー）
- 機能の出し分け、API ハンドラーの共有、マイグレーションの作り直し、アプリ名のパラメーター化はやらない
- 各タスクの終わりに必ず通すもの:
  `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
  （Task 1〜2 はまだワークスペースではないので `--workspace` なし）
- テストはデバッグビルドで走らせる。`cargo test --release` では `FUGANTT_TODAY` が効かず、突き合わせテストは落ちる

## Review Focus

仕様が暗に求めているが、放っておくとどのテストも踏まない入力。上から起きやすい順。各行のテストは持ち主のタスクに入れてある。

1. **`data-api` の末尾にスラッシュがある**（`/g/api/plans/demo/`）。`//grid` を叩かずに動くこと → Task 8 Step 1
2. **固定 DB をテストが書き換える**。起動時にマイグレーションと祝日の投入が走るので、リポジトリの `v1.db` を直接開くと差分が出る。テストは必ず写しを開くこと → Task 2 Step 3（`the_fixture_is_never_written_to`）
3. **`sqlx` フィーチャーなしで `fu-gantt-core` がビルドできない**。fugantt は常にフィーチャーを付けるので、外すと壊れていても気付かない → Task 5 Step 5 と CI
4. **祝日の対象外の年**（2019 以前、2100 以降）と **存在しない日付**（2月30日）。panic せず、春分・秋分を欠いた一覧／`None` を返すこと → Task 4 Step 1
5. **年を省いた日付が「今日」に依存する**（`8/5`、`0805`、`5`）。年末年始に読み違えないよう、今日を引数で渡して確かめること → Task 6 Step 1

---

## ファイル構成（完成時）

```
fugantt/
├─ Cargo.toml                         [workspace]（新規）
├─ Cargo.lock
├─ Dockerfile  .dockerignore  .gitignore  .gitattributes（新規）
├─ README.md  README.en.md  LICENSE  NOTICE
├─ docs/
│  └─ gantt-api.md                    ホストとの約束（新規）
├─ apps/fugantt/
│  ├─ Cargo.toml  build.rs  Topcoat.toml
│  ├─ migrations/                     0001〜0034、無変更
│  ├─ assets/                         theme.css, page.js, favicon.svg（web/ から移す）
│  ├─ src/                            いまの src/ から domain.rs・sortkey.rs を除いたもの
│  │  ├─ clock.rs                     今日の日付（新規）
│  │  └─ holidays.rs                  keep_filled だけ残る
│  └─ tests/
│     ├─ common/mod.rs                サーバーを起動する道具（cli.rs から抜き出す）
│     ├─ cli.rs
│     ├─ golden.rs                    突き合わせテスト（新規）
│     ├─ fixtures/{build.sh, seed.sql, v1.db}
│     └─ golden/*.json, *.html        記録
├─ crates/
│  ├─ fu-calendar/src/lib.rs          Day と japanese()
│  ├─ fu-gantt-core/src/
│  │  ├─ lib.rs
│  │  ├─ domain.rs                    いまの src/domain.rs
│  │  ├─ sortkey.rs                   いまの src/sortkey.rs
│  │  ├─ wire.rs                      グリッドとやり取りする JSON の型（新規）
│  │  └─ text.rs                      セルに打たれた文字の読み取り（新規）
│  └─ fu-gantt-web/
│     ├─ src/lib.rs                   GRID_JS, GRID_CSS, fingerprint()
│     └─ web/                         いまの web/（theme.css・page.js・favicon.svg を除く）
└─ examples/minimal-host/             fugantt 以外からグリッドを動かす見本（新規）
   ├─ Cargo.toml
   ├─ src/main.rs
   └─ tests/smoke.rs
```

---

### Task 1: 今日の日付を1か所から取る

突き合わせテストは「今日」が動くと毎日落ちる。今日を返す関数を1つにまとめ、デバッグビルドに限って環境変数で固定できるようにする。リリースビルドの挙動は変わらない。

**Files:**
- Create: `src/clock.rs`
- Modify: `src/main.rs`（`mod clock;` と 140行目付近）、`src/project.rs:709`、`src/api.rs:730`・`3349`、`src/pages/capacity.rs:43`・`126`、`src/pages/admin.rs:426`

**Interfaces:**
- Produces: `crate::clock::today() -> jiff::civil::Date`。デバッグビルドでは `FUGANTT_TODAY=YYYY-MM-DD` があればその日を返す

- [ ] **Step 1: `src/clock.rs` を書く**

```rust
//! What day it is.
//!
//! "Late" is a question about the calendar day of whoever is reading the plan,
//! so every place that asks takes the answer from here, in the server's local
//! zone rather than UTC.

use jiff::civil::Date;

/// Today.
///
/// A debug build reads `FUGANTT_TODAY` first, so a test can compare what the
/// server says against what it said when the answer was written down. A
/// release build has no such branch: the day is the day.
pub fn today() -> Date {
    #[cfg(debug_assertions)]
    if let Some(pinned) = std::env::var("FUGANTT_TODAY")
        .ok()
        .and_then(|day| day.parse().ok())
    {
        return pinned;
    }

    jiff::Zoned::now().date()
}
```

- [ ] **Step 2: `src/main.rs` の `mod` 一覧に `mod clock;` を足す**（`mod browser;` の次、アルファベット順）

- [ ] **Step 3: 今日を取っている箇所を置き換える**

テスト以外の `Zoned::now().date()` と `Zoned::now().year()` を `crate::clock::today()` に置き換える。`backup.rs` の `strftime` は時刻なので触らない。

| 場所 | 置き換え前 | 置き換え後 |
|---|---|---|
| `src/main.rs:140` | `jiff::Zoned::now().date()` | `clock::today()` |
| `src/project.rs:709` | `Zoned::now().date()` | `crate::clock::today()` |
| `src/api.rs:730` | `jiff::Zoned::now().date().to_string()` | `crate::clock::today().to_string()` |
| `src/api.rs:3349` | `jiff::Zoned::now().date()` | `crate::clock::today()` |
| `src/pages/capacity.rs:43` | `jiff::Zoned::now().date()` | `crate::clock::today()` |
| `src/pages/capacity.rs:126` | `jiff::Zoned::now().date()` | `crate::clock::today()` |
| `src/pages/admin.rs:426` | `jiff::Zoned::now().year()` | `crate::clock::today().year()` |

`src/project.rs` で `use jiff::Zoned;` が未使用になったら消す。`src/api.rs` の `#[cfg(test)] mod tests` の中（3385行目以降）の `jiff::Zoned::now()` はそのまま残す。

- [ ] **Step 4: 残りが無いことを確かめる**

Run: `grep -n "Zoned::now()" src/*.rs src/pages/*.rs src/interop/*.rs`
Expected: `src/backup.rs` の2行と、`src/api.rs` の3385行目以降（テスト）だけ

- [ ] **Step 5: 全部通す**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: PASS。テストの数は増減なし（単体 115、`tests/cli.rs` 4）

- [ ] **Step 6: Commit**

```bash
git add src
git commit -m "Ask one place what day it is

A debug build can be told the day through FUGANTT_TODAY, so a test can
hold the server to an answer written down earlier. A release build has
no such branch.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: 挙動を固定するテスト

固定 DB を相手にサーバーを起動し、API とページの出力を記録と突き合わせる。以降の全タスクの安全網。

**Files:**
- Create: `tests/common/mod.rs`、`tests/golden.rs`、`tests/fixtures/build.sh`、`tests/fixtures/seed.sql`、`tests/fixtures/v1.db`、`tests/golden/*`、`.gitattributes`
- Modify: `tests/cli.rs`（共通の道具を `common` から使う）、`.gitignore`

**Interfaces:**
- Consumes: Task 1 の `FUGANTT_TODAY`
- Produces（`tests/common/mod.rs`）:
  - `pub fn scratch(name: &str) -> PathBuf`
  - `pub fn fugantt(dir: &Path) -> Command`
  - `pub struct Server { pub child: Child, pub port: u16, pub dir: PathBuf }`
  - `Server::start(name: &str, env: &[(&str, &str)]) -> Server`
  - `Server::start_with(name: &str, env: &[(&str, &str)], prepare: impl FnOnce(&Path)) -> Server`
  - `Server::get(&self, path: &str) -> String`（これまでどおり、ヘッダー込みの生の応答）
  - `Server::body(&self, path: &str) -> (u16, String)`（ステータスと本文）
- 固定する今日: `2026-09-15`。認証なしの合言葉: `yes-everyone-on-this-network-can-edit`

- [ ] **Step 1: 共通の道具を `tests/common/mod.rs` に抜き出す**

`tests/cli.rs` の `scratch`・`fugantt`・`Server` を移し、`start_with` と `body` を足す。

```rust
//! Running the executable the way a person runs it, for the tests that do.

#![allow(dead_code)]

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

/// An empty directory to stand in, so no settings file or database of the
/// developer's is picked up.
pub fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("cli-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn fugantt(dir: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fugantt"));
    command
        .current_dir(dir)
        .env("FUGANTT_DB", dir.join("fugantt.db"))
        .env("FUGANTT_OPEN", "0")
        .env_remove("FUGANTT_NO_AUTH")
        .env_remove("FUGANTT_TODAY")
        .env_remove("FUGANTT_CONF")
        .env_remove("HOST");
    command
}

/// A running server, stopped when the test is done with it.
pub struct Server {
    pub child: Child,
    pub port: u16,
    pub dir: PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Server {
    pub fn start(name: &str, env: &[(&str, &str)]) -> Self {
        Self::start_with(name, env, |_| {})
    }

    /// Starts in a fresh directory, after `prepare` has put what it wants there.
    pub fn start_with(name: &str, env: &[(&str, &str)], prepare: impl FnOnce(&Path)) -> Self {
        let dir = scratch(name);
        prepare(&dir);

        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();

        let child = fugantt(&dir)
            .env("PORT", port.to_string())
            .envs(env.iter().copied())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        Self { child, port, dir }
    }

    fn connect(&self) -> TcpStream {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            match TcpStream::connect(("127.0.0.1", self.port)) {
                Ok(stream) => break stream,
                Err(error) if Instant::now() > deadline => panic!("起動しなかった: {error}"),
                Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        }
    }

    /// The raw response, status line and headers included.
    pub fn get(&self, path: &str) -> String {
        String::from_utf8_lossy(&self.raw(path)).into_owned()
    }

    /// The status and the body, with the transfer encoding taken off.
    pub fn body(&self, path: &str) -> (u16, String) {
        let raw = self.raw(path);
        let split = raw
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("ヘッダーの終わりが無い");
        let head = String::from_utf8_lossy(&raw[..split]).to_lowercase();
        let status = head
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .expect("ステータスが読めない");

        let rest = &raw[split + 4..];
        let bytes = if head.contains("transfer-encoding: chunked") {
            unchunk(rest)
        } else {
            rest.to_vec()
        };

        (status, String::from_utf8(bytes).expect("本文が UTF-8 でない"))
    }

    fn raw(&self, path: &str) -> Vec<u8> {
        let mut stream = self.connect();
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAccept: */*\r\nConnection: close\r\n\r\n",
            self.port
        )
        .unwrap();

        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        raw
    }
}

/// Joins a chunked body back together.
fn unchunk(mut rest: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    loop {
        let Some(line_end) = rest.windows(2).position(|window| window == b"\r\n") else {
            break;
        };
        let size = std::str::from_utf8(&rest[..line_end])
            .ok()
            .and_then(|line| usize::from_str_radix(line.split(';').next()?.trim(), 16).ok())
            .unwrap_or(0);
        if size == 0 {
            break;
        }

        let start = line_end + 2;
        out.extend_from_slice(&rest[start..start + size]);
        rest = &rest[start + size + 2..];
    }

    out
}
```

`tests/cli.rs` は先頭を次に変え、`scratch`・`fugantt`・`Server` の定義と使わなくなった `use` を消す。4つのテスト本体は触らない。

```rust
//! The executable itself, run the way a person runs it.

mod common;

use std::io::{BufRead, BufReader};

use common::{Server, fugantt, scratch};

const VERSION: &str = concat!("fugantt ", env!("CARGO_PKG_VERSION"));
```

Run: `cargo test --test cli`
Expected: 4 passed

- [ ] **Step 2: 固定 DB を作る手順を書く**

`tests/fixtures/seed.sql`（作り物のデータ。業務の実データは入れない）:

```sql
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
```

`tests/fixtures/build.sh`:

```sh
#!/bin/sh
# Builds tests/fixtures/v1.db: a database made by the server itself, then
# filled from seed.sql. Run from the package directory, with a debug build:
#
#   cargo build && sh tests/fixtures/build.sh
#
# The file it writes is committed. Rebuild it only when the seed changes, and
# never to make a failing golden test pass.

set -e

HERE=$(cd "$(dirname "$0")" && pwd)
BIN="${FUGANTT_BIN:-$(cargo metadata --format-version 1 --no-deps | sed 's/.*"target_directory":"\([^"]*\)".*/\1/')/debug/fugantt}"
WORK=$(mktemp -d)
DB="$WORK/fugantt.db"
PORT=18611

# The server makes the schema, the shared account and the holidays for the
# years around the pinned day.
( cd "$WORK" && \
  FUGANTT_DB="$DB" FUGANTT_OPEN=0 FUGANTT_TODAY=2026-09-15 PORT=$PORT \
  FUGANTT_NO_AUTH=yes-everyone-on-this-network-can-edit \
  "$BIN" >"$WORK/log" 2>&1 ) &
SERVER=$!
trap 'kill $SERVER 2>/dev/null || true' EXIT

for _ in $(seq 1 100); do
  curl -fsS "http://127.0.0.1:$PORT/" >/dev/null 2>&1 && break
  sleep 0.1
done
curl -fsS "http://127.0.0.1:$PORT/" >/dev/null

kill $SERVER
wait $SERVER 2>/dev/null || true
trap - EXIT

sqlite3 "$DB" <"$HERE/seed.sql"
# One file, nothing left in the write-ahead log.
sqlite3 "$DB" "PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE; VACUUM;"

cp "$DB" "$HERE/v1.db"
rm -rf "$WORK"
echo "wrote $HERE/v1.db"
```

`.gitignore` の `*.db` の行の下に足す:

```
# テストが読む固定データ。これだけは持つ
!tests/fixtures/v1.db
```

`.gitattributes` を新規に作る:

```
*.db binary
tests/golden/* text eol=lf
```

Run: `cargo build && sh tests/fixtures/build.sh && sqlite3 tests/fixtures/v1.db "SELECT COUNT(*) FROM tasks; SELECT value FROM app_settings WHERE key='japan_holiday_years';"`
Expected: `7` と `2025 2026 2027`

`seed.sql` が列名の違いで失敗したら、`sqlite3 <作業中の DB> ".schema <表名>"` で実際の列を確かめて `seed.sql` を直す。表や列を足すためにマイグレーションを触ってはいけない。

- [ ] **Step 3: 突き合わせテストを書く（まだ記録が無いので失敗する）**

`tests/golden.rs`:

```rust
//! What the server says about a fixed database, held against what it said
//! when the answers were written down.
//!
//! The code underneath is being moved between crates. None of that is meant
//! to change a byte of what a person or a program is handed, and this is the
//! test that would notice.
//!
//! To write the answers down afresh: `UPDATE_GOLDEN=1 cargo test --test golden`.
//! That is for a deliberate change to the output, never for a failure.

mod common;

use std::path::{Path, PathBuf};

use common::Server;

const TODAY: &str = "2026-09-15";
const NO_AUTH: &str = "yes-everyone-on-this-network-can-edit";

fn here() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

/// A server reading a copy of the fixture. The committed file is never opened:
/// starting up migrates and fills in holidays, and both write.
fn server(name: &str) -> Server {
    Server::start_with(
        name,
        &[("FUGANTT_NO_AUTH", NO_AUTH), ("FUGANTT_TODAY", TODAY)],
        |dir: &Path| {
            std::fs::copy(here().join("fixtures/v1.db"), dir.join("fugantt.db")).unwrap();
        },
    )
}

/// JSON, laid out one value to a line so a difference reads as a difference.
fn json(server: &Server, path: &str) -> String {
    let (status, body) = server.body(path);
    assert_eq!(status, 200, "{path}: {body}");

    let value: serde_json::Value = serde_json::from_str(&body).expect("JSON でない");
    serde_json::to_string_pretty(&value).unwrap() + "\n"
}

/// A page, with the two things that move for reasons of their own taken out:
/// the hash in each static URL, and the version.
fn html(server: &Server, path: &str) -> String {
    let (status, body) = server.body(path);
    assert_eq!(status, 200, "{path}: {body}");

    let version = concat!("fugantt ", env!("CARGO_PKG_VERSION"));
    unhash(&body.replace(version, "fugantt VERSION")) + "\n"
}

/// `/static/0123456789abcdef/name` → `/static/HASH/name`.
fn unhash(page: &str) -> String {
    const MARK: &str = "/static/";
    let mut out = String::with_capacity(page.len());
    let mut rest = page;

    while let Some(at) = rest.find(MARK) {
        let after = &rest[at + MARK.len()..];
        let hash = after
            .bytes()
            .take_while(u8::is_ascii_hexdigit)
            .count();

        out.push_str(&rest[..at + MARK.len()]);
        if hash == 16 && after[hash..].starts_with('/') {
            out.push_str("HASH");
            rest = &after[hash..];
        } else {
            rest = after;
        }
    }

    out.push_str(rest);
    out
}

fn check(name: &str, actual: &str) {
    let path = here().join("golden").join(name);

    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }

    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{name} の記録が無い。UPDATE_GOLDEN=1 で作る"));

    if expected != actual {
        let was = path.with_extension("actual");
        std::fs::write(&was, actual).unwrap();
        panic!(
            "{name} が記録と違う。\n  diff {} {}",
            path.display(),
            was.display()
        );
    }
}

#[test]
fn the_grid_says_what_it_said() {
    let server = server("golden-grid");

    check("grid.json", &json(&server, "/api/projects/golden/grid"));
    check("grid-empty.json", &json(&server, "/api/projects/empty/grid"));
}

#[test]
fn the_document_says_what_it_said() {
    let server = server("golden-document");

    check("document.json", &json(&server, "/api/projects/golden/document"));
    check(
        "document-tasks.json",
        &json(&server, "/api/projects/golden/document?settings=0"),
    );
}

#[test]
fn the_numbers_say_what_they_said() {
    let server = server("golden-summary");

    check("projects.json", &json(&server, "/api/projects"));
    check("summary.json", &json(&server, "/api/summary"));
}

#[test]
fn the_pages_say_what_they_said() {
    let server = server("golden-pages");

    for (name, path) in [
        ("page-home.html", "/"),
        ("page-project.html", "/projects/golden"),
        ("page-stats.html", "/projects/golden/stats"),
        ("page-capacity.html", "/projects/golden/capacity"),
        ("page-settings.html", "/projects/golden/settings"),
        ("page-history.html", "/projects/golden/history"),
        ("page-admin.html", "/admin"),
    ] {
        check(name, &html(&server, path));
    }
}

/// The day the tests pin is the day the server works from.
#[test]
fn today_is_the_pinned_day() {
    let server = server("golden-today");

    let grid = json(&server, "/api/projects/golden/grid");
    assert!(grid.contains(&format!("\"today\": \"{TODAY}\"")), "{grid}");
}

/// Reading the fixture must leave the committed file exactly as it was.
#[test]
fn the_fixture_is_never_written_to() {
    let fixture = here().join("fixtures/v1.db");
    let before = std::fs::read(&fixture).unwrap();

    let server = server("golden-untouched");
    let _ = server.body("/api/projects/golden/grid");
    drop(server);

    assert!(std::fs::read(&fixture).unwrap() == before, "固定 DB が書き換わった");
    assert!(!fixture.with_extension("db-wal").exists(), "WAL が出来ている");
}

#[test]
fn a_static_hash_is_taken_out_and_nothing_else() {
    assert_eq!(
        unhash(r#"<link href="/static/0123456789abcdef/grid.css">"#),
        r#"<link href="/static/HASH/grid.css">"#
    );
    // Not sixteen hex digits: left alone.
    assert_eq!(unhash("/static/abc/grid.css"), "/static/abc/grid.css");
    assert_eq!(unhash("no urls here"), "no urls here");
}
```

`Cargo.toml` に `[dev-dependencies]` が無ければ足す必要はない（`serde_json` は通常の依存に既にある）。

Run: `cargo test --test golden`
Expected: FAIL。`grid.json の記録が無い。UPDATE_GOLDEN=1 で作る`（記録を使う4本が落ち、`today_is_the_pinned_day`・`the_fixture_is_never_written_to`・`a_static_hash_is_taken_out_and_nothing_else` は通る）

- [ ] **Step 4: 記録を取り、2回続けて同じになることを確かめる**

```bash
UPDATE_GOLDEN=1 cargo test --test golden
cargo test --test golden
cargo test --test golden
```

Expected: 2回目・3回目とも 7 passed。

落ちる場合は、`tests/golden/<名前>.actual` と記録を `diff` で比べ、実行ごとに変わる部分を特定する。ページごとに変わる乱数（CSRF トークン、CSP の nonce など）が原因なら、`html()` の中でその値を固定の文字に置き換える処理を `unhash` と同じ形で足し、`a_static_hash_is_taken_out_and_nothing_else` の隣にそのテストを足す。サーバー側のコードを変えて安定させてはいけない。

- [ ] **Step 5: 記録の中身を目で確かめる**

Run: `grep -c '"id"' tests/golden/grid.json; grep -o '"late[a-z_]*": true' tests/golden/grid.json | sort | uniq -c; grep -c "fg-\|fugantt-grid" tests/golden/page-project.html`
Expected: タスクの `id` が7件以上、遅れの `true` が1件以上、ページに `fugantt-grid` が1件以上。空や数行しかない記録は、何も固定していないのと同じなので原因を調べる

- [ ] **Step 6: 全部通して Commit**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`

```bash
git add tests .gitignore .gitattributes
git commit -m "Hold the server to what it says about a fixed database

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: 器だけワークスペースにする

ファイルを動かすだけ。ロジックには触らない。

**Files:**
- Create: `Cargo.toml`（ワークスペース）
- Move: `Cargo.toml` → `apps/fugantt/Cargo.toml`、`src/` `migrations/` `tests/` `build.rs` `Topcoat.toml` → `apps/fugantt/`
- Modify: `apps/fugantt/src/static_files.rs`（`include_str!` のパス）、`.gitignore`、`.gitattributes`、`.github/workflows/ci.yml`、`.github/workflows/release.yml`、`Dockerfile`

**Interfaces:**
- Produces: ワークスペースの共通設定 `[workspace.package]`（`edition`・`license`・`repository`・`homepage`・`authors`）と `[workspace.dependencies]`（`jiff`・`serde`・`serde_json`・`sqlx`）。以降のクレートは `xxx.workspace = true` で使う

- [ ] **Step 1: 動かす**

```bash
mkdir -p apps/fugantt
git mv Cargo.toml src migrations tests build.rs Topcoat.toml apps/fugantt/
```

- [ ] **Step 2: ルートの `Cargo.toml` を書く**

```toml
[workspace]
resolver = "3"
members = ["apps/fugantt"]

[workspace.package]
edition = "2024"
license = "Apache-2.0"
repository = "https://github.com/fu-foo/fugantt"
homepage = "https://github.com/fu-foo/fugantt"
authors = ["Kazunari Fukagawa"]

[workspace.dependencies]
jiff = "0.2.35"
serde = { version = "1", features = ["derive"] }
serde_json = "1.0.151"
sqlx = { version = "0.8", default-features = false }
```

- [ ] **Step 3: `apps/fugantt/Cargo.toml` の `[package]` を直す**

`[package]` を次に置き換える。`[dependencies]` と `[build-dependencies]` は、下に示す4行以外そのまま。

```toml
[package]
name = "fugantt"
version = "1.0.1"
description = "A Gantt chart built around Japanese practice: plan against actual, counted in working days."
readme = "../../README.md"
edition.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
authors.workspace = true
publish = false
```

`[dependencies]` のうち次の4つをワークスペースの定義に寄せる:

```toml
jiff.workspace = true
serde.workspace = true
serde_json.workspace = true
sqlx = { workspace = true, features = ["runtime-tokio", "sqlite", "migrate", "macros"] }
```

- [ ] **Step 4: `web/` への相対パスを直す**

`apps/fugantt/src/static_files.rs` の4行（`web/` はまだルートにある）:

```rust
const GRID_CSS: &str = include_str!("../../../web/dist/grid.css");
const THEME_CSS: &str = include_str!("../../../web/src/theme.css");
const GRID_JS: &str = include_str!("../../../web/dist/grid.js");
const FAVICON: &str = include_str!("../../../web/dist/favicon.svg");
const PAGE_JS: &str = include_str!("../../../web/src/page.js");
```

- [ ] **Step 5: 周辺のパスを直す**

`.gitignore`: `!tests/fixtures/v1.db` → `!apps/fugantt/tests/fixtures/v1.db`

`.gitattributes`: `tests/golden/* text eol=lf` → `apps/fugantt/tests/golden/* text eol=lf`

`.github/workflows/ci.yml` の最後の2行:

```yaml
      - run: cargo fmt --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
```

`.github/workflows/release.yml` の Build:

```yaml
        run: cargo build --release --locked -p fugantt --target ${{ matrix.target }}
```

`Dockerfile`:

```dockerfile
RUN cargo build --release --locked -p fugantt
```

成果物の場所（`target/<target>/release/fugantt`、`/src/target/release/fugantt`）はワークスペースでも変わらないので、Package と `COPY --from=build` は触らない。

- [ ] **Step 6: 全部通す**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS。単体 115、`cli` 4、`golden` 7

Run: `cargo build --release --locked -p fugantt && ls -la target/release/fugantt`
Expected: 実行ファイルが出来る（`Cargo.lock` が変わっていて `--locked` が落ちる場合は、先に `cargo build` で `Cargo.lock` を更新してコミットに含める）

Run: `git status --short | grep -v "^R " | head`
Expected: 名前の変更（`R`）以外は、上で編集したファイルと `Cargo.toml`・`Cargo.lock` だけ

- [ ] **Step 7: 開発サーバーが新しい場所から動くことを確かめる**

Run: `cd apps/fugantt && cargo-topcoat dev`（起動を確認したら Ctrl+C）
Expected: `fugantt 1.0.1` と `画面: http://127.0.0.1:1861` が出る

`cargo-topcoat dev` がパッケージを見つけられない場合は、`Topcoat.toml` をリポジトリのルートに戻し（`git mv apps/fugantt/Topcoat.toml .`）、ルートから `cargo-topcoat dev` を試す。動いたほうの置き場所と起動の仕方を、Task 10 で README に書く。

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "Make room: a workspace with the program in apps/fugantt

Files moved; no logic touched.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: fu-calendar を切り出す

祝日の計算を、依存ゼロのクレートにする。日付と曜日の計算も自前で持つ。

**Files:**
- Create: `crates/fu-calendar/Cargo.toml`、`crates/fu-calendar/src/lib.rs`
- Modify: `Cargo.toml`（members）、`apps/fugantt/Cargo.toml`、`apps/fugantt/src/holidays.rs`、`.github/workflows/ci.yml`

**Interfaces:**
- Produces:
  - `fu_calendar::Day`（`Copy + Ord + Hash + Display`。`Display` は `YYYY-MM-DD`）
  - `Day::new(year: i16, month: u8, day: u8) -> Option<Day>`、`year()`・`month()`・`day()`
  - `Day::weekday(self) -> u8`（0 = 月曜 … 6 = 日曜）、`Day::next(self) -> Day`
  - `fu_calendar::japanese(year: i16) -> Vec<(Day, &'static str)>`（日付順）
  - フィーチャー `jiff`: `impl From<Day> for jiff::civil::Date`
  - フィーチャー `chrono`: `impl From<Day> for chrono::NaiveDate`

- [ ] **Step 1: クレートとテストを書く（実装が無いので失敗する）**

ルートの `Cargo.toml` の members を `["apps/fugantt", "crates/fu-calendar"]` にする。

`crates/fu-calendar/Cargo.toml`:

```toml
[package]
name = "fu-calendar"
version = "0.1.0"
description = "Japanese public holidays, worked out from the rules rather than listed."
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
publish = false

[features]
jiff = ["dep:jiff"]
chrono = ["dep:chrono"]

[dependencies]
jiff = { workspace = true, optional = true }
chrono = { version = "0.4", default-features = false, optional = true }

[dev-dependencies]
jiff.workspace = true
```

`crates/fu-calendar/src/lib.rs` に、まずテストだけを書く:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn names(year: i16) -> Vec<(String, &'static str)> {
        japanese(year)
            .into_iter()
            .map(|(day, name)| (day.to_string(), name))
            .collect()
    }

    /// The whole of 2026, checked against the Cabinet Office's published list.
    #[test]
    fn twenty_twenty_six_matches_the_published_calendar() {
        assert_eq!(
            names(2026),
            [
                ("2026-01-01".to_owned(), "元日"),
                ("2026-01-12".to_owned(), "成人の日"),
                ("2026-02-11".to_owned(), "建国記念の日"),
                ("2026-02-23".to_owned(), "天皇誕生日"),
                ("2026-03-20".to_owned(), "春分の日"),
                ("2026-04-29".to_owned(), "昭和の日"),
                ("2026-05-03".to_owned(), "憲法記念日"),
                ("2026-05-04".to_owned(), "みどりの日"),
                ("2026-05-05".to_owned(), "こどもの日"),
                ("2026-05-06".to_owned(), "振替休日"),
                ("2026-07-20".to_owned(), "海の日"),
                ("2026-08-11".to_owned(), "山の日"),
                ("2026-09-21".to_owned(), "敬老の日"),
                ("2026-09-22".to_owned(), "国民の休日"),
                ("2026-09-23".to_owned(), "秋分の日"),
                ("2026-10-12".to_owned(), "スポーツの日"),
                ("2026-11-03".to_owned(), "文化の日"),
                ("2026-11-23".to_owned(), "勤労感謝の日"),
            ]
        );
    }

    /// When 5月3日 falls on a Sunday the run stretches to 5月6日: simply taking
    /// the next day would land on 5月4日, which is already a holiday.
    #[test]
    fn a_substitute_skips_over_the_holidays_behind_it() {
        let days = names(2026);

        assert!(
            days.contains(&("2026-05-06".to_owned(), "振替休日")),
            "{days:?}"
        );
    }

    /// Two days between 敬老の日 and 秋分の日 make the シルバーウィーク run.
    #[test]
    fn silver_week_appears_only_when_the_gap_is_one_day() {
        assert!(names(2026).contains(&("2026-09-22".to_owned(), "国民の休日")));
        // 2027 has 敬老の日 on 9/20 and 秋分の日 on 9/23, too far apart for one.
        assert!(!names(2027).iter().any(|(_, name)| *name == "国民の休日"));
    }

    #[test]
    fn the_equinoxes_move_with_the_year() {
        assert!(names(2025).contains(&("2025-03-20".to_owned(), "春分の日")));
        assert!(names(2024).contains(&("2024-03-20".to_owned(), "春分の日")));
        assert!(names(2023).contains(&("2023-03-21".to_owned(), "春分の日")));
    }

    #[test]
    fn every_year_has_at_least_the_sixteen_named_days() {
        for year in 2024..=2030 {
            assert!(japanese(year).len() >= 16, "{year}: {:?}", japanese(year));
        }
    }

    /// The arithmetic here is this crate's own, so it is held against a
    /// library that has had more eyes on it: every day of eighty years.
    #[test]
    fn the_days_and_weekdays_agree_with_jiff() {
        let mut ours = Day::new(2020, 1, 1).unwrap();
        let mut theirs = jiff::civil::date(2020, 1, 1);

        while theirs.year() < 2100 {
            assert_eq!(ours.to_string(), theirs.to_string());
            assert_eq!(
                i8::try_from(ours.weekday()).unwrap(),
                theirs.weekday().to_monday_zero_offset(),
                "{theirs}"
            );

            ours = ours.next();
            theirs = theirs.tomorrow().unwrap();
        }
    }

    #[test]
    fn a_day_that_does_not_exist_is_not_a_day() {
        assert!(Day::new(2026, 2, 30).is_none());
        assert!(Day::new(2026, 13, 1).is_none());
        assert!(Day::new(2026, 0, 1).is_none());
        assert!(Day::new(2026, 4, 0).is_none());
        assert!(Day::new(2024, 2, 29).is_some());
        assert!(Day::new(2026, 2, 29).is_none());
        assert!(Day::new(2100, 2, 29).is_none());
    }

    /// Outside the years the equinox approximation covers there is still an
    /// answer, and it is short two days rather than wrong or a panic.
    #[test]
    fn a_year_past_the_equinox_table_goes_without_them() {
        for year in [1979, 2100] {
            let days = japanese(year);

            assert!(!days.iter().any(|(_, name)| name.ends_with("分の日")), "{year}");
            assert!(days.len() >= 14, "{year}: {days:?}");
        }
    }

    #[test]
    fn the_holidays_come_in_date_order() {
        for year in 2020..=2099 {
            let days = japanese(year);
            assert!(days.windows(2).all(|pair| pair[0].0 < pair[1].0), "{year}");
        }
    }
}
```

Run: `cargo test -p fu-calendar`
Expected: FAIL（`cannot find function japanese`、`cannot find type Day`）

- [ ] **Step 2: 実装を書く**

`crates/fu-calendar/src/lib.rs` のテストの上に置く。`japanese`・`nth_monday`・`equinox`・`add_substitutes`・`add_citizens_holidays` の規則と文言は、`apps/fugantt/src/holidays.rs` にあるものと同じにする（日付の型だけが違う）。

```rust
//! Japanese public holidays.
//!
//! Typing sixteen dates a year by hand is the kind of work a schedule tool
//! should not create. The rules are in 国民の祝日に関する法律, and they are
//! rules rather than a list: two of the days move with the equinoxes, four are
//! "the nth Monday", and two more are produced by holidays landing badly.
//!
//! Valid from 2020, when 天皇誕生日 moved to 2月23日 and 体育の日 became
//! スポーツの日. Earlier years would need the older names and dates.
//!
//! This crate depends on nothing. Whoever uses it has already chosen a date
//! library, and it is not this crate's place to bring a second one: the `jiff`
//! and `chrono` features turn a [`Day`] into theirs.

use std::fmt;

/// A calendar day that exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day {
    year: i16,
    month: u8,
    day: u8,
}

impl Day {
    /// The day, or `None` when the calendar has no such day.
    pub fn new(year: i16, month: u8, day: u8) -> Option<Self> {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let length = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => return None,
        };

        (1..=length)
            .contains(&day)
            .then_some(Self { year, month, day })
    }

    pub fn year(self) -> i16 {
        self.year
    }

    pub fn month(self) -> u8 {
        self.month
    }

    pub fn day(self) -> u8 {
        self.day
    }

    /// 0 for Monday through 6 for Sunday.
    pub fn weekday(self) -> u8 {
        // The first of January 1970 was a Thursday.
        (self.serial() + 3).rem_euclid(7) as u8
    }

    /// The day after.
    pub fn next(self) -> Self {
        Self::from_serial(self.serial() + 1)
    }

    /// Days since the first of January 1970.
    fn serial(self) -> i64 {
        let (month, day) = (i64::from(self.month), i64::from(self.day));
        // Counted from March, so the leap day is the last of its year.
        let year = i64::from(self.year) - i64::from(month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

        era * 146_097 + day_of_era - 719_468
    }

    fn from_serial(serial: i64) -> Self {
        let shifted = serial + 719_468;
        let era = shifted.div_euclid(146_097);
        let day_of_era = shifted - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let from_march = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * from_march + 2) / 5 + 1;
        let month = if from_march < 10 {
            from_march + 3
        } else {
            from_march - 9
        };
        let year = year_of_era + era * 400 + i64::from(month <= 2);

        Self {
            year: year as i16,
            month: month as u8,
            day: day as u8,
        }
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[cfg(feature = "jiff")]
impl From<Day> for jiff::civil::Date {
    fn from(day: Day) -> Self {
        // A `Day` only exists for a day the calendar has.
        jiff::civil::date(day.year, day.month as i8, day.day as i8)
    }
}

#[cfg(feature = "chrono")]
impl From<Day> for chrono::NaiveDate {
    fn from(day: Day) -> Self {
        chrono::NaiveDate::from_ymd_opt(day.year.into(), day.month.into(), day.day.into())
            .expect("a Day is a day the calendar has")
    }
}

const SUNDAY: u8 = 6;

/// Every public holiday in `year`, in date order, with its name.
pub fn japanese(year: i16) -> Vec<(Day, &'static str)> {
    let mut days: Vec<(Day, &'static str)> = Vec::new();

    let mut fixed = |month: u8, day: u8, name: &'static str| {
        if let Some(date) = Day::new(year, month, day) {
            days.push((date, name));
        }
    };

    fixed(1, 1, "元日");
    fixed(2, 11, "建国記念の日");
    fixed(2, 23, "天皇誕生日");
    fixed(4, 29, "昭和の日");
    fixed(5, 3, "憲法記念日");
    fixed(5, 4, "みどりの日");
    fixed(5, 5, "こどもの日");
    fixed(8, 11, "山の日");
    fixed(11, 3, "文化の日");
    fixed(11, 23, "勤労感謝の日");

    // The "happy Monday" holidays: the nth Monday of the month.
    for (month, nth, name) in [
        (1, 2, "成人の日"),
        (7, 3, "海の日"),
        (9, 3, "敬老の日"),
        (10, 2, "スポーツの日"),
    ] {
        if let Some(date) = nth_monday(year, month, nth) {
            days.push((date, name));
        }
    }

    if let Some(date) = equinox(year, Season::Spring) {
        days.push((date, "春分の日"));
    }
    if let Some(date) = equinox(year, Season::Autumn) {
        days.push((date, "秋分の日"));
    }

    days.sort_by_key(|(date, _)| *date);

    add_substitutes(&mut days);
    add_citizens_holidays(&mut days);

    days.sort_by_key(|(date, _)| *date);
    days
}

fn nth_monday(year: i16, month: u8, nth: u8) -> Option<Day> {
    let first = Day::new(year, month, 1)?;

    // Days from the 1st to that month's first Monday.
    let offset = (7 - first.weekday()) % 7;

    Day::new(year, month, 1 + offset + (nth - 1) * 7)
}

enum Season {
    Spring,
    Autumn,
}

/// The equinox days, from the approximation the Cabinet Office publishes.
///
/// Good for 1980–2099.
fn equinox(year: i16, season: Season) -> Option<Day> {
    if !(1980..=2099).contains(&year) {
        return None;
    }

    let base = match season {
        Season::Spring => 20.8431,
        Season::Autumn => 23.2488,
    };

    let years = f64::from(year - 1980);
    let day = (base + 0.242_194 * years - (years / 4.0).floor()).floor() as u8;
    let month = match season {
        Season::Spring => 3,
        Season::Autumn => 9,
    };

    Day::new(year, month, day)
}

/// 振替休日: a holiday on a Sunday moves to the next day that is not itself one.
fn add_substitutes(days: &mut Vec<(Day, &'static str)>) {
    let existing: Vec<Day> = days.iter().map(|(date, _)| *date).collect();
    let mut extra: Vec<(Day, &'static str)> = Vec::new();

    for date in existing.iter().filter(|day| day.weekday() == SUNDAY) {
        let mut candidate = *date;

        // May 3rd lands on a Sunday behind two more holidays, so this walks
        // rather than simply adding a day.
        loop {
            candidate = candidate.next();

            if !existing.contains(&candidate) && !extra.iter().any(|(day, _)| *day == candidate) {
                extra.push((candidate, "振替休日"));
                break;
            }
        }
    }

    days.extend(extra);
}

/// 国民の休日: a single ordinary day held between two holidays becomes one too.
///
/// In practice this is the Tuesday of シルバーウィーク, when 敬老の日 and
/// 秋分の日 fall two days apart.
fn add_citizens_holidays(days: &mut Vec<(Day, &'static str)>) {
    let existing: Vec<Day> = days.iter().map(|(date, _)| *date).collect();
    let mut extra = Vec::new();

    for date in &existing {
        let gap = date.next();
        let after = gap.next();

        if existing.contains(&after) && !existing.contains(&gap) && gap.weekday() != SUNDAY {
            extra.push((gap, "国民の休日"));
        }
    }

    days.extend(extra);
}
```

- [ ] **Step 3: テストを通す**

Run: `cargo test -p fu-calendar && cargo test -p fu-calendar --all-features && cargo clippy -p fu-calendar --all-features --all-targets -- -D warnings`
Expected: 9 passed（2回とも）。clippy の指摘なし

- [ ] **Step 4: fugantt から使う**

`apps/fugantt/Cargo.toml` の `[dependencies]` に足す:

```toml
fu-calendar = { path = "../../crates/fu-calendar" }
```

`apps/fugantt/src/holidays.rs` から次を消す: `japanese`・`nth_monday`・`Season`・`equinox`・`add_substitutes`・`add_citizens_holidays`、および `mod tests` のうち `#[test]` の5本（`twenty_twenty_six_…`〜`every_year_has_…`）と `names` ヘルパー。`#[tokio::test]` の3本と `calendar`・`days` ヘルパーは残す。

ファイル先頭の `use` とモジュールの説明を次にする:

```rust
//! Keeping Japan's public holidays in the installation's calendar.
//!
//! Which days they are is `fu_calendar`'s business. This is the part that
//! writes them down, once a year, without being asked.

use fu_calendar::japanese;
use jiff::civil::Date;
```

`keep_filled` の本体は変えない。`for (date, name) in japanese(year)` の `date.to_string()` は `Day` の `Display` がこれまでと同じ `YYYY-MM-DD` を返すので、そのまま動く。

- [ ] **Step 5: CI にフィーチャーの組み合わせを足す**

`.github/workflows/ci.yml` の `cargo test --workspace` の次に:

```yaml
      # The calendar promises to stand on nothing, and to meet either date
      # library when asked. Both are easy to break without noticing.
      - run: cargo test -p fu-calendar --all-features
      - run: cargo tree -p fu-calendar --edges normal --depth 1
```

- [ ] **Step 6: 全部通す**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS。`fugantt` の単体は 110（115 − 移した5）、`fu-calendar` 9、`cli` 4、`golden` 7

Run: `cargo tree -p fu-calendar --edges normal --depth 1`
Expected: `fu-calendar v0.1.0 (…)` の1行だけ

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Move the holiday rules into a crate that depends on nothing

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: fu-gantt-core に計算を移す

`domain.rs` と `sortkey.rs` を内容そのままで移す。

**Files:**
- Create: `crates/fu-gantt-core/Cargo.toml`、`crates/fu-gantt-core/src/lib.rs`
- Move: `apps/fugantt/src/domain.rs` → `crates/fu-gantt-core/src/domain.rs`、`apps/fugantt/src/sortkey.rs` → `crates/fu-gantt-core/src/sortkey.rs`
- Modify: `Cargo.toml`（members）、`apps/fugantt/Cargo.toml`、`apps/fugantt/src/main.rs`、`.github/workflows/ci.yml`

**Interfaces:**
- Produces:
  - `fu_gantt_core::domain`（いまの `crate::domain` と同じ中身。`build(project_id: &str, revision: i64, today: Date, rows: Vec<TaskRow>, settings: Settings) -> GridData` ほか）
  - `fu_gantt_core::sortkey`（`between(before: Option<&str>, after: Option<&str>) -> String` ほか）
  - フィーチャー `sqlx`: `TaskRow` と `FilterSet` に `sqlx::FromRow` が付く
- fugantt の中では、これまでどおり `crate::domain::…`・`crate::sortkey::…` で参照できる

- [ ] **Step 1: クレートを作ってファイルを動かす**

ルートの `Cargo.toml` の members に `"crates/fu-gantt-core"` を足す。

```bash
mkdir -p crates/fu-gantt-core/src
git mv apps/fugantt/src/domain.rs crates/fu-gantt-core/src/domain.rs
git mv apps/fugantt/src/sortkey.rs crates/fu-gantt-core/src/sortkey.rs
```

`crates/fu-gantt-core/Cargo.toml`:

```toml
[package]
name = "fu-gantt-core"
version = "0.1.0"
description = "The arithmetic of a Gantt chart: plan against actual, counted in working days."
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
publish = false

[features]
# Lets the stored rows be read straight out of a query.
sqlx = ["dep:sqlx"]

[dependencies]
jiff.workspace = true
serde.workspace = true
sqlx = { workspace = true, features = ["derive"], optional = true }
```

`crates/fu-gantt-core/src/lib.rs`:

```rust
//! The arithmetic of a Gantt chart, and nothing that stores one.
//!
//! Hand [`domain::build`] the rows as they are kept and the settings around
//! them, and it answers with what the grid draws: the outline flattened, each
//! summary row added up from what is under it, the days counted the way the
//! project counts them, and what is late said plainly.
//!
//! Where the rows are kept, who may read them and how they arrive over the
//! wire are the host's business.

pub mod domain;
pub mod sortkey;
```

- [ ] **Step 2: `sqlx` の derive をフィーチャーの下に入れる**

`crates/fu-gantt-core/src/domain.rs`:

- 11行目の `use sqlx::FromRow;` を消す
- `TaskRow` の derive を次にする:

```rust
/// A task exactly as it is stored.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct TaskRow {
```

- `FilterSet` の derive を次にする:

```rust
/// A named set of filter conditions.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct FilterSet {
```

- [ ] **Step 3: クレートの外から見えるようにする**

Run: `grep -n "pub(crate)\|pub(super)" crates/fu-gantt-core/src/domain.rs crates/fu-gantt-core/src/sortkey.rs`

出てきたものは `pub` にする。出なければ何もしない。

Run: `cargo build -p fu-gantt-core --all-features 2>&1 | grep -E "^(error|warning)" | sort | uniq -c`

`dead_code` の警告が出た項目は、バイナリーの中では使われていたが `pub` ではなかったもの。fugantt から使われているなら `pub` にし、テスト専用なら `#[cfg(test)]` を付ける。**関数の中身は変えない。**

- [ ] **Step 4: fugantt から使う**

`apps/fugantt/Cargo.toml`:

```toml
fu-gantt-core = { path = "../../crates/fu-gantt-core", features = ["sqlx"] }
```

`apps/fugantt/src/main.rs`: `mod domain;` と `mod sortkey;` の2行を消し、`mod` の並びの下、`use std::error::Error;` の前に足す:

```rust
// The chart's arithmetic lives in its own crate. Named here so the rest of
// the program goes on saying `crate::domain`.
use fu_gantt_core::{domain, sortkey};
```

Run: `cargo build -p fugantt 2>&1 | grep -E "^error" -A6 | head -60`

`private` や `not found` のエラーが出た項目は、`crates/fu-gantt-core` 側で `pub` にする。fugantt 側の呼び出しは書き換えない。

- [ ] **Step 5: フィーチャーなしでもビルドできることを確かめ、CI に足す**

Run: `cargo check -p fu-gantt-core --no-default-features && cargo test -p fu-gantt-core --no-default-features && cargo tree -p fu-gantt-core --no-default-features --edges normal --depth 1`
Expected: PASS。依存は `jiff` と `serde` だけ

`.github/workflows/ci.yml` に足す:

```yaml
      # fugantt always turns `sqlx` on, so nothing else would notice the core
      # no longer building without it.
      - run: cargo test -p fu-gantt-core --no-default-features
```

- [ ] **Step 6: 全部通す**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS。`fu-gantt-core` 34（domain 28 + sortkey 6）、`fugantt` の単体 76（110 − 34）、`fu-calendar` 9、`cli` 4、`golden` 7

Run: `git diff --stat -M HEAD -- crates/fu-gantt-core/src/domain.rs crates/fu-gantt-core/src/sortkey.rs`
Expected: 名前の変更として認識され、変わった行は derive と `pub` の数行だけ

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Move the chart's arithmetic into fu-gantt-core

domain.rs and sortkey.rs, as they were. Reading a row out of a query is
behind a feature, so a host that keeps its rows some other way does not
have to bring sqlx.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: グリッドとやり取りする型と、セルの文字の読み取りを移す

グリッドは日付を `8/5` や `0805` のような打ったままの文字で送る。それを読む関数と、JSON の型を `fu-gantt-core` に置く。`api.rs` には同じ名前・同じエラー文の薄い包みを残すので、ハンドラーの本体は変わらない。

**Files:**
- Create: `crates/fu-gantt-core/src/wire.rs`、`crates/fu-gantt-core/src/text.rs`
- Modify: `crates/fu-gantt-core/src/lib.rs`、`apps/fugantt/src/api.rs`、`apps/fugantt/src/project.rs`（`Move`）、`apps/fugantt/src/live.rs`（`Change`）

**Interfaces:**
- Consumes: `fu_gantt_core::domain::{GridData, TaskView}`、`crate::clock::today()`
- Produces（`fu_gantt_core::wire`）:
  - 入力: `CellEdit { field: CellField, field_id: Option<String>, value: String, expect: Option<String> }`、`CellField`（enum、`snake_case`）、`InsertTask { after: Option<String> }`、`MoveRequest { action: Move }`、`Move`（`Indent`・`Outdent`・`Up`・`Down`）、`PlaceRequest { parent: Option<String>, after: Option<String> }`、`SaveFilterSet { name: String, conditions: String, shared: bool }`、`RemoveFilterSet { id: String }`、`LeaveList { leaves: Vec<LeaveEntry> }`、`LeaveEntry { assignee, start, end, note, kind }`
  - 出力: `Mutation { grid: Option<GridData>, patch: Option<Patch>, task_id: Option<String>, note: Option<&'static str> }`、`Patch { revision, rows, after, moved, removed, range_start, range_end, total }`、`Moved { id, after, depth }`、`LiveChange { revision, task_id, actor, client, kind }`、`CELL: &str = "cell"`、`PLAN: &str = "plan"`
- Produces（`fu_gantt_core::text`）:
  - `pub enum TextError { Colour, TargetShape, TargetPercent, WaitShape, WaitDate, Date }`
  - `pub fn normalize_width(value: &str) -> String`
  - `pub fn flexible_date(value: &str, today: Date) -> Option<Date>`
  - `pub fn date(value: &str, today: Date) -> Result<Option<String>, TextError>`
  - `pub fn waits(value: &str, today: Date) -> Result<String, TextError>`
  - `pub fn targets(value: &str, today: Date) -> Result<String, TextError>`
  - `pub fn colour(value: &str) -> Result<String, TextError>`

- [ ] **Step 1: `text.rs` のテストを書く（実装が無いので失敗する）**

`crates/fu-gantt-core/src/lib.rs` に `pub mod text;` と `pub mod wire;` を足す。

`crates/fu-gantt-core/src/text.rs` に、まずテストだけを書く:

```rust
#[cfg(test)]
mod tests {
    use jiff::civil::date as day;

    use super::*;

    #[test]
    fn a_date_is_read_however_it_was_typed() {
        let today = day(2026, 9, 15);

        for typed in ["2026-08-05", "20260805", "2026/8/5", "2026年8月5日", "8/5", "0805", "805", "８／５"] {
            assert_eq!(flexible_date(typed, today), Some(day(2026, 8, 5)), "{typed}");
        }

        // One or two digits are a day of this month.
        assert_eq!(flexible_date("5", today), Some(day(2026, 9, 5)));
        assert_eq!(flexible_date("05", today), Some(day(2026, 9, 5)));

        for typed in ["", "abc", "2026-02-30", "1234567", "13/1"] {
            assert_eq!(flexible_date(typed, today), None, "{typed}");
        }
    }

    /// The year and the month left out are today's — whatever today is. Read
    /// on the last day of a year and on the first day of the next, the same
    /// keys mean days a year apart.
    #[test]
    fn what_is_left_out_is_taken_from_the_day_it_is_read() {
        let old_year = day(2026, 12, 31);
        let new_year = day(2027, 1, 1);

        assert_eq!(flexible_date("1/5", old_year), Some(day(2026, 1, 5)));
        assert_eq!(flexible_date("1/5", new_year), Some(day(2027, 1, 5)));
        assert_eq!(flexible_date("3", old_year), Some(day(2026, 12, 3)));
        assert_eq!(flexible_date("3", new_year), Some(day(2027, 1, 3)));
        // Nothing is carried forward: the 31st of a month that has thirty days
        // is not a day.
        assert_eq!(flexible_date("31", day(2026, 9, 15)), None);
    }

    #[test]
    fn an_empty_cell_clears_the_date() {
        let today = day(2026, 9, 15);

        assert_eq!(date("", today), Ok(None));
        assert_eq!(date("  ", today), Ok(None));
        assert_eq!(date("8/5", today), Ok(Some("2026-08-05".to_owned())));
        assert_eq!(date("nope", today), Err(TextError::Date));
    }

    #[test]
    fn waits_are_ranges_with_an_optional_reason() {
        let today = day(2026, 9, 15);

        assert_eq!(waits("8/17〜8/21", today), Ok("2026-08-17/2026-08-21".to_owned()));
        assert_eq!(
            waits("8/17〜8/21 他部署, 9/1〜", today),
            Ok("2026-08-17/2026-08-21:他部署\n2026-09-01/".to_owned())
        );
        // Written backwards is still that range.
        assert_eq!(waits("8/21〜8/17", today), Ok("2026-08-17/2026-08-21".to_owned()));
        assert_eq!(waits("", today), Ok(String::new()));
        assert_eq!(waits("8/17", today), Err(TextError::WaitShape));
        assert_eq!(waits("x〜8/17", today), Err(TextError::WaitDate));
    }

    #[test]
    fn targets_are_a_date_and_a_percentage() {
        let today = day(2026, 9, 15);

        assert_eq!(targets("2026-08-20 30%", today), Ok("2026-08-20/30".to_owned()));
        assert_eq!(
            targets("8/28 100, 8/20 30％", today),
            Ok("2026-08-20/30\n2026-08-28/100".to_owned())
        );
        // The same date twice is one promise revised.
        assert_eq!(targets("8/20 30, 8/20 40", today), Ok("2026-08-20/40".to_owned()));
        // Nothing to split a date from a percentage on.
        assert_eq!(targets("0820", today), Err(TextError::TargetShape));
        assert_eq!(targets("8/20 abc", today), Err(TextError::TargetShape));
        assert_eq!(targets("8/20 101", today), Err(TextError::TargetPercent));
        assert_eq!(targets("x 30", today), Err(TextError::WaitDate));
    }

    #[test]
    fn a_colour_is_six_hex_digits_or_nothing() {
        assert_eq!(colour(""), Ok(String::new()));
        assert_eq!(colour("#B91C1C"), Ok("#b91c1c".to_owned()));
        assert_eq!(colour("b91c1c"), Ok("#b91c1c".to_owned()));
        assert_eq!(colour("#fff"), Err(TextError::Colour));
        assert_eq!(colour("red"), Err(TextError::Colour));
    }
}
```

Run: `cargo test -p fu-gantt-core text::`
Expected: FAIL（`cannot find function flexible_date` など）

- [ ] **Step 2: `text.rs` の実装を書く**

`apps/fugantt/src/api.rs` の次の関数を、`text.rs` のテストの上に **切り取って貼る**: `normalize_width`（1311行目付近）、`parse_colour`、`parse_targets`、`parse_waits`、`split_reason`、`with_reason`、`split_dash_range`、`wait_date`、`parse_date`、`flexible_date`（3155〜3384行目付近）。関数に付いている説明のコメントも一緒に移す。そのうえで、次の変更だけを加える。読み取りの規則（分割の文字、桁数の扱い、並べ替え）は1文字も変えない。

ファイルの先頭:

```rust
//! Reading what somebody typed into a cell.
//!
//! The grid sends the text as it was typed — `8/5`, `0805`, `8/17〜8/21 他部署`
//! — and reading it is the server's job, so that every host reads it the same
//! way. Nothing here knows what language the person reads: a refusal says what
//! kind it is, and the host puts it into words.
//!
//! "Today" is handed in. A date with the year left out means this year, and
//! which year that is depends on who is asking and when.

use jiff::civil::Date;

/// Why a cell's text could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextError {
    /// Not `#rrggbb`.
    Colour,
    /// 予定進捗 was not a date and a percentage.
    TargetShape,
    /// 予定進捗 named a percentage outside 0–100.
    TargetPercent,
    /// 待ち was not a range.
    WaitShape,
    /// A day inside a 待ち or 予定進捗 could not be read.
    WaitDate,
    /// A date cell could not be read.
    Date,
}
```

関数ごとの変更:

| 元（`api.rs`） | 先（`text.rs`） | 変えるところ |
|---|---|---|
| `fn normalize_width(value: &str) -> String` | `pub fn normalize_width(value: &str) -> String` | `pub` を付けるだけ |
| `fn parse_colour(value, l) -> Result<String>` | `pub fn colour(value: &str) -> Result<String, TextError>` | `return Err(bad_request(…).into())` → `return Err(TextError::Colour)` |
| `fn parse_targets(value, l) -> Result<String>` | `pub fn targets(value: &str, today: Date) -> Result<String, TextError>` | 形の誤り2か所 → `TextError::TargetShape`、範囲外 → `TextError::TargetPercent`、`wait_date(date.trim(), l)?` → `wait_date(date.trim(), today)?` |
| `fn parse_waits(value, l) -> Result<String>` | `pub fn waits(value: &str, today: Date) -> Result<String, TextError>` | 形の誤り → `TextError::WaitShape`、`wait_date(…, l)?` → `wait_date(…, today)?` |
| `fn wait_date(text, l) -> Result<Date>` | `fn wait_date(text: &str, today: Date) -> Result<Date, TextError>` | 本体は `flexible_date(text, today).ok_or(TextError::WaitDate)` |
| `fn parse_date(value, l) -> Result<Option<String>>` | `pub fn date(value: &str, today: Date) -> Result<Option<String>, TextError>` | `flexible_date(value)` → `flexible_date(value, today)`、誤り → `TextError::Date` |
| `pub fn flexible_date(value: &str) -> Option<Date>` | `pub fn flexible_date(value: &str, today: Date) -> Option<Date>` | `let today = crate::clock::today();` の行を消す（引数になった） |
| `split_reason`・`with_reason`・`split_dash_range` | 同名のまま、非公開 | 変更なし |

Run: `cargo test -p fu-gantt-core text::`
Expected: 6 passed

テストが落ちたら、移す前の `api.rs` の関数（`git show HEAD:apps/fugantt/src/api.rs`）と1行ずつ比べる。**テストの期待値のほうが現行の挙動と違っていたら、テストを現行の挙動に合わせる**（このタスクは挙動を変えない）。

- [ ] **Step 3: `api.rs` に同じ名前の薄い包みを置く**

`apps/fugantt/src/api.rs` の、関数を切り取った場所に次を置く。ハンドラー本体（`parse_date(value, l)?` などの呼び出し）は書き換えない。

```rust
use fu_gantt_core::text::{self, TextError, normalize_width};

/// A refusal from the reader, in the words the person reads.
fn refused(error: TextError, l: crate::i18n::Lang) -> topcoat::Error {
    bad_request(l.t(match error {
        TextError::Colour => "色は #rrggbb の形式で指定してください。",
        TextError::TargetShape => "予定進捗は「8/20 30%」のように日付と％で入力してください。",
        TextError::TargetPercent => "進捗は0〜100で入力してください。",
        TextError::WaitShape => "待ちは「8/17〜8/21」のように範囲で入力してください。",
        TextError::WaitDate => "待ちの日付は「8/17」か「2026-08-17」の形式です。",
        TextError::Date => {
            "日付は 5・805・0805・20260805・8/5・2026-08-05 のように入力してください。"
        }
    }))
    .into()
}

fn parse_colour(value: &str, l: crate::i18n::Lang) -> Result<String> {
    text::colour(value).map_err(|error| refused(error, l))
}

fn parse_targets(value: &str, l: crate::i18n::Lang) -> Result<String> {
    text::targets(value, crate::clock::today()).map_err(|error| refused(error, l))
}

fn parse_waits(value: &str, l: crate::i18n::Lang) -> Result<String> {
    text::waits(value, crate::clock::today()).map_err(|error| refused(error, l))
}

fn parse_date(value: &str, l: crate::i18n::Lang) -> Result<Option<String>> {
    text::date(value, crate::clock::today()).map_err(|error| refused(error, l))
}

/// A date, however somebody typed it. See [`text::flexible_date`].
pub fn flexible_date(value: &str) -> Option<Date> {
    text::flexible_date(value, crate::clock::today())
}
```

`use` はファイル先頭の `use` 群に入れる。`topcoat::Error` という型名がコンパイルで通らなければ、`bad_request(…).into()` が返す先の型（`topcoat::Result<T>` の `Err` 側）を `cargo doc -p topcoat --open` か rust-analyzer で確かめ、その名前を `refused` の戻り値に書く。エラー文の6つは、移す前の `api.rs` にあった文字列と1文字も違わないこと。

Run: `git show HEAD:apps/fugantt/src/api.rs | grep -o 'l\.t("[^"]*")' | sort -u > /tmp/before.txt; grep -c "" /tmp/before.txt`
移した6つの文言が `refused` の中に揃っているかを目で突き合わせる。

- [ ] **Step 4: `wire.rs` を書き、`api.rs`・`project.rs`・`live.rs` の定義を置き換える**

次の型を **切り取って** `crates/fu-gantt-core/src/wire.rs` に貼る。説明のコメントも一緒に移す。型とフィールドに `pub` を付け、名前を右の列のとおりにする。

| 元 | 先（`wire.rs`） |
|---|---|
| `api.rs` `struct Mutation` | `pub struct Mutation` |
| `api.rs` `struct Patch` | `pub struct Patch` |
| `api.rs` `struct Moved` | `pub struct Moved` |
| `api.rs` `enum Field` | `pub enum CellField` |
| `api.rs` `struct CellEdit` | `pub struct CellEdit`（`field: CellField`） |
| `api.rs` `struct InsertTask` | `pub struct InsertTask` |
| `api.rs` `struct MoveRequest` | `pub struct MoveRequest`（`action: Move`） |
| `api.rs` `struct PlaceRequest` | `pub struct PlaceRequest` |
| `api.rs` `struct SaveFilterSet` | `pub struct SaveFilterSet` |
| `api.rs` `struct LeaveList`・`struct LeaveEntry` | `pub struct LeaveList`・`pub struct LeaveEntry` |
| `project.rs` `pub enum Move` | `pub enum Move` |
| `live.rs` `pub struct Change`・`pub const CELL`・`pub const PLAN` | `pub struct LiveChange`・`pub const CELL`・`pub const PLAN` |

`wire.rs` の先頭と、新しく足す型:

```rust
//! What the grid and its host say to each other.
//!
//! The grid is one script that talks JSON to about ten addresses. These are
//! the shapes it sends and the shapes it expects back, so a host can be held
//! to them by the compiler rather than by reading the script.
//!
//! `docs/gantt-api.md` says which address takes which.

use serde::{Deserialize, Serialize};

use crate::domain::{GridData, TaskView};

/// Which saved set of filter conditions to throw away.
#[derive(Debug, Deserialize)]
pub struct RemoveFilterSet {
    pub id: String,
}
```

入力の型には `#[derive(Debug, Deserialize)]`、出力の型には `#[derive(Debug, Serialize)]` が付いている状態にする（元に `Debug` が無いものには足す）。`#[serde(...)]` の属性は元のまま。

呼び出し側の置き換え:

`apps/fugantt/src/api.rs` の `use` 群に足す:

```rust
use fu_gantt_core::wire::{
    CellEdit, CellField as Field, InsertTask, LeaveEntry, LeaveList, MoveRequest, Moved, Mutation,
    Patch, PlaceRequest, RemoveFilterSet, SaveFilterSet,
};
```

`CellField as Field` としておくので、`Field::Start` などの既存の記述は書き換えない。未使用の警告が出た名前（`LeaveEntry` など）は `use` から外す。

`remove_filter_set`（`#[route(POST "/api/projects/{project_id}/filters/remove")]`）の引数を `Json(which): Json<Named>` から `Json(which): Json<RemoveFilterSet>` にする。`Named` をほかのハンドラーが使っていなければ（`grep -n "Named" apps/fugantt/src/api.rs`）定義ごと消す。使っていれば残す。

`apps/fugantt/src/project.rs`: `pub enum Move { … }` を消し、代わりに

```rust
pub use fu_gantt_core::wire::Move;
```

`apps/fugantt/src/live.rs`: `Change`・`CELL`・`PLAN` の定義を消し、代わりに

```rust
pub use fu_gantt_core::wire::{CELL, LiveChange as Change, PLAN};
```

`api.rs` に `impl Field { … }` があった場合（`grep -n "impl Field" apps/fugantt/src/api.rs`）は、外部クレートの型には固有の `impl` を書けないので、その関数を `field_label` と同じ形の自由関数（`fn 名前(field: Field, …)`）に直し、呼び出しを `field.名前(…)` から `名前(field, …)` に変える。

- [ ] **Step 5: 型の形が変わっていないことをテストで固定する**

`crates/fu-gantt-core/src/wire.rs` の末尾:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// The grid writes these by hand, so the names are a promise.
    #[test]
    fn the_grid_s_requests_are_read() {
        let edit: CellEdit =
            serde_json::from_str(r#"{"field":"actual_start","value":"8/5"}"#).unwrap();
        assert!(matches!(edit.field, CellField::ActualStart));
        assert_eq!(edit.value, "8/5");
        assert_eq!(edit.field_id, None);
        assert_eq!(edit.expect, None);

        let edit: CellEdit = serde_json::from_str(
            r#"{"field":"custom","field_id":"f-1","value":"x","expect":"y"}"#,
        )
        .unwrap();
        assert!(matches!(edit.field, CellField::Custom));
        assert_eq!(edit.field_id.as_deref(), Some("f-1"));
        assert_eq!(edit.expect.as_deref(), Some("y"));

        let insert: InsertTask = serde_json::from_str(r#"{"after":null}"#).unwrap();
        assert_eq!(insert.after, None);

        let request: MoveRequest = serde_json::from_str(r#"{"action":"outdent"}"#).unwrap();
        assert!(matches!(request.action, Move::Outdent));

        let place: PlaceRequest =
            serde_json::from_str(r#"{"parent":"t-1","after":null}"#).unwrap();
        assert_eq!(place.parent.as_deref(), Some("t-1"));

        let leaves: LeaveList = serde_json::from_str(
            r#"{"leaves":[{"assignee":"佐藤","start":"9/7","end":"9/9"}]}"#,
        )
        .unwrap();
        assert_eq!(leaves.leaves[0].note, "");
        assert_eq!(leaves.leaves[0].kind, "");
    }

    /// What is absent is left out rather than sent as null: the grid tells a
    /// patch from a whole plan by which key is there.
    #[test]
    fn an_answer_leaves_out_what_it_does_not_carry() {
        let answer = Mutation {
            grid: None,
            patch: None,
            task_id: Some("t-1".to_owned()),
            note: None,
        };

        assert_eq!(serde_json::to_string(&answer).unwrap(), r#"{"task_id":"t-1"}"#);

        let change = LiveChange {
            revision: 3,
            task_id: None,
            actor: "山田".to_owned(),
            client: None,
            kind: CELL,
        };

        assert_eq!(
            serde_json::to_string(&change).unwrap(),
            r#"{"revision":3,"task_id":null,"actor":"山田","client":null,"kind":"cell"}"#
        );
    }
}
```

`crates/fu-gantt-core/Cargo.toml` に足す:

```toml
[dev-dependencies]
serde_json.workspace = true
```

Run: `cargo test -p fu-gantt-core wire::`
Expected: 2 passed。期待した JSON と違う場合は、移す前の `#[serde(...)]` 属性を落としていないか確かめる（期待値のほうを変えない）

- [ ] **Step 6: 全部通す**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo test -p fu-gantt-core --no-default-features`
Expected: PASS。`fu-gantt-core` 42（34 + text 6 + wire 2）、`fugantt` の単体 76（`api.rs` の6本はそのまま包みを通して通る）、`golden` 7

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Put what the grid says, and how it is read, beside the arithmetic

The shapes the grid sends and expects are public types now, and reading
what was typed into a cell is a plain function of the text and the day.
The handlers keep wrappers of the same names, with the same refusals.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: fu-gantt-web に画面を移す

`web/` をクレートに入れ、埋め込みをそこから取る。画面のコードはまだ変えない。

**Files:**
- Create: `crates/fu-gantt-web/Cargo.toml`、`crates/fu-gantt-web/src/lib.rs`
- Move: `web/` → `crates/fu-gantt-web/web/`、ただし `web/src/theme.css`・`web/src/page.js`・`web/dist/favicon.svg` → `apps/fugantt/assets/`
- Modify: `Cargo.toml`（members）、`apps/fugantt/Cargo.toml`、`apps/fugantt/src/static_files.rs`、`.gitignore`、`.dockerignore`

**Interfaces:**
- Produces:
  - `fu_gantt_web::GRID_JS: &str`、`fu_gantt_web::GRID_CSS: &str`
  - `fu_gantt_web::fingerprint(body: &str) -> String`（16桁の16進。FNV-1a）
  - `fu_gantt_web::grid_js_hash() -> &'static str`、`fu_gantt_web::grid_css_hash() -> &'static str`

- [ ] **Step 1: 動かす**

ルートの `Cargo.toml` の members に `"crates/fu-gantt-web"` を足す。

```bash
mkdir -p crates/fu-gantt-web/src apps/fugantt/assets
git mv web/src/theme.css web/src/page.js web/dist/favicon.svg apps/fugantt/assets/
git mv web crates/fu-gantt-web/web
[ -d web/node_modules ] && mv web/node_modules crates/fu-gantt-web/web/ && rmdir web || true
```

Run: `grep -rn "theme.css\|page.js\|favicon" crates/fu-gantt-web/web/src crates/fu-gantt-web/web/test crates/fu-gantt-web/web/package.json`
Expected: 該当なし。出た場合は、その参照が移した3ファイルを指していないかを確かめ、指していれば相対パスを `apps/fugantt/assets/` に直す

- [ ] **Step 2: クレートとテストを書く**

`crates/fu-gantt-web/Cargo.toml`:

```toml
[package]
name = "fu-gantt-web"
version = "0.1.0"
description = "The Gantt grid: one script and one stylesheet, ready to be served."
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
publish = false

[dependencies]
```

`crates/fu-gantt-web/src/lib.rs`:

```rust
//! The grid, as the two files a page needs.
//!
//! Put an element on the page and load the script:
//!
//! ```html
//! <link rel="stylesheet" href="…/grid.css">
//! <div id="fugantt-grid" data-project="abc" data-api="/somewhere/abc"></div>
//! <script src="…/grid.js" defer></script>
//! ```
//!
//! The script draws into the element and talks JSON to the addresses under
//! `data-api`. What those addresses take and give back is in
//! `docs/gantt-api.md`, and as types in `fu-gantt-core`.
//!
//! Nothing here serves a request. How a file reaches a browser is the host's
//! own business, and every framework has its own way; this crate is the bytes
//! and a name for them that changes when they do.
//!
//! The files are built by `npm run build` in `web/` and committed, so that
//! depending on this crate never asks for Node.

use std::sync::LazyLock;

/// The grid island.
pub const GRID_JS: &str = include_str!("../web/dist/grid.js");

/// Its stylesheet. Complete on its own, in a light palette; a host that has
/// colours of its own sets the `--fg-*` properties on `.fg-grid`.
pub const GRID_CSS: &str = include_str!("../web/dist/grid.css");

/// A short digest of the contents, so a changed file gets a changed URL.
///
/// FNV-1a rather than a cryptographic hash: nothing here is a secret, and the
/// only job is to differ when the bytes differ.
pub fn fingerprint(body: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;

    for byte in body.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    format!("{hash:016x}")
}

/// [`fingerprint`] of [`GRID_JS`], worked out once.
pub fn grid_js_hash() -> &'static str {
    static HASH: LazyLock<String> = LazyLock::new(|| fingerprint(GRID_JS));
    &HASH
}

/// [`fingerprint`] of [`GRID_CSS`], worked out once.
pub fn grid_css_hash() -> &'static str {
    static HASH: LazyLock<String> = LazyLock::new(|| fingerprint(GRID_CSS));
    &HASH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_files_made_it_in() {
        assert!(GRID_JS.contains("fugantt-grid"), "グリッドの JS が入っていない");
        assert!(GRID_CSS.contains(".fg-grid"), "グリッドの CSS が入っていない");
    }

    /// A changed file must not keep the old URL, or browsers will hold the old
    /// bytes for a year.
    #[test]
    fn the_digest_moves_with_the_contents() {
        assert_ne!(fingerprint("a"), fingerprint("b"));
        assert_eq!(fingerprint("a"), fingerprint("a"));
        assert_eq!(grid_js_hash().len(), 16);
        assert_eq!(grid_js_hash(), fingerprint(GRID_JS));
        assert_ne!(grid_js_hash(), grid_css_hash());
    }

    /// The digest fugantt put in its URLs before this crate existed. If this
    /// moves, every installation's cached files are orphaned for no reason.
    #[test]
    fn the_digest_is_the_one_it_always_was() {
        assert_eq!(fingerprint(""), "cbf29ce484222325");
        assert_eq!(fingerprint("a"), "af63dc4c8601ec8c");
    }
}
```

Run: `cargo test -p fu-gantt-web`
Expected: 3 passed

- [ ] **Step 3: fugantt から使う**

`apps/fugantt/Cargo.toml`:

```toml
fu-gantt-web = { path = "../../crates/fu-gantt-web" }
```

`apps/fugantt/src/static_files.rs`:

- `GRID_CSS` と `GRID_JS` の `const` を消し、`use fu_gantt_web::{GRID_CSS, GRID_JS, fingerprint};` を足す
- 自前の `fn fingerprint` を消す
- 残りの3つのパスを直す:

```rust
/// Dark mode and the colour tokens the pages read. Hand-written. It also ties
/// the grid's `--fg-*` properties to those tokens, which is how the grid takes
/// this program's colours.
const THEME_CSS: &str = include_str!("../assets/theme.css");
const FAVICON: &str = include_str!("../assets/favicon.svg");
const PAGE_JS: &str = include_str!("../assets/page.js");
```

- テスト `the_url_moves_with_the_contents` の最初の2行（`fingerprint` 同士の比較）は `fu-gantt-web` に移ったので消し、`assert!(grid_js().starts_with("/static/"));` だけ残す

モジュール先頭の説明の「These three files」など、ファイルの数や出どころに触れている文が実態と合わなくなっていたら直す。

- [ ] **Step 4: 周辺のパスを直す**

`.gitignore`: `/web/node_modules` → `/crates/fu-gantt-web/web/node_modules`。`web/dist is committed on purpose` のコメントのパスも同じく直す

`.dockerignore`: `/web/node_modules` → `/crates/fu-gantt-web/web/node_modules`

`crates/fu-gantt-web/web/test/grid.test.mjs` の冒頭コメントにある実行例を直す:

```
 * Needs a dev server (`cargo-topcoat dev`, from apps/fugantt) and the database
 * it is using:
 *
 *   FUGANTT_DB=../../../apps/fugantt/fugantt.db node test/grid.test.mjs
```

（Task 3 Step 7 で `Topcoat.toml` をルートに戻した場合は、その置き場所に合わせて書く。）

- [ ] **Step 5: 画面のビルドが新しい場所で同じものを出すことを確かめる**

```bash
cd crates/fu-gantt-web/web
[ -d node_modules ] || npm ci
npm run typecheck
npm run build
cd -
git status --short crates/fu-gantt-web/web/dist
```

Expected: `typecheck` が通り、`dist/` に差分が出ない（同じソースから同じ出力）

- [ ] **Step 6: 全部通す**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS。`fu-gantt-web` 3、`golden` 7（ページの記録は、静的ファイルのハッシュを抜いてあるので変わらない）

Run: `cargo build --release --locked -p fugantt`
Expected: ビルドできる

- [ ] **Step 7: ブラウザーのテストを通す**

端末を2つ使う。1つめで開発サーバーを起動し（Task 3 Step 7 で確かめた場所から `cargo-topcoat dev`）、2つめで:

```bash
cd crates/fu-gantt-web/web && FUGANTT_DB=<開発サーバーが使っている DB の絶対パス> node test/grid.test.mjs
```

Expected: 失敗 0。移す前（`git stash` せず、`main` を別の作業ツリーで動かして比べる）と同じ本数が通る

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "Move the grid's script and stylesheet into fu-gantt-web

The pages' own files — theme, favicon, page.js — stay with the program.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 8: グリッドに API の基点を渡せるようにする

`data-api` と `data-filter-count` を足す。どちらも無ければ現行と同じに動く。

**Files:**
- Modify: `crates/fu-gantt-web/web/src/grid.ts`、`crates/fu-gantt-web/web/dist/grid.js`（ビルドで更新）
- Create: `crates/fu-gantt-web/web/src/base.ts`、`crates/fu-gantt-web/web/test/base.test.mjs`
- Modify: `crates/fu-gantt-web/web/package.json`（`test:unit`）、`.github/workflows/ci.yml`

**Interfaces:**
- Produces（ホストが書く HTML の属性）:
  - `data-project`（必須）: プロジェクトの識別子。ブラウザーに覚えさせる折りたたみ状態のキーにも使う
  - `data-api`（任意）: API の基点。末尾のスラッシュは無視する。無ければ `/api/projects/{data-project を URL エンコードしたもの}`
  - `data-filter-count`（任意）: 絞り込み件数を書く要素の CSS セレクター。無ければ `#fugantt-filter-count`
- Produces（`base.ts`）: `export function apiBase(given: string | undefined, projectId: string): string`

- [ ] **Step 1: 基点の組み立てのテストを書く（実装が無いので失敗する）**

`crates/fu-gantt-web/web/test/base.test.mjs`:

```js
/**
 * Where the grid's API lives. No browser needed: this is string handling, and
 * the one place a host's markup can send every request to the wrong address.
 *
 *   npm run test:unit
 */

import assert from "node:assert/strict";
import { test } from "node:test";
import { build } from "esbuild";

const { outputFiles } = await build({
  entryPoints: [new URL("../src/base.ts", import.meta.url).pathname],
  bundle: true,
  format: "esm",
  write: false,
});
const { apiBase } = await import(
  `data:text/javascript;base64,${Buffer.from(outputFiles[0].text).toString("base64")}`
);

test("with nothing said, the address is the one fugantt has always used", () => {
  assert.equal(apiBase(undefined, "abc"), "/api/projects/abc");
  assert.equal(apiBase("", "abc"), "/api/projects/abc");
  assert.equal(apiBase("   ", "abc"), "/api/projects/abc");
});

test("the project's id is encoded on the way into the default address", () => {
  assert.equal(apiBase(undefined, "計画 1"), "/api/projects/%E8%A8%88%E7%94%BB%201");
  assert.equal(apiBase(undefined, "a/b"), "/api/projects/a%2Fb");
});

test("a host's own address is used as it stands", () => {
  assert.equal(apiBase("/g/api/plans/demo", "abc"), "/g/api/plans/demo");
  assert.equal(apiBase("https://example.test/x/abc", "abc"), "https://example.test/x/abc");
});

test("a trailing slash does not become a doubled one", () => {
  assert.equal(apiBase("/g/api/plans/demo/", "abc"), "/g/api/plans/demo");
  assert.equal(apiBase("/g/api/plans/demo///", "abc"), "/g/api/plans/demo");
  assert.equal(apiBase("  /g/api/plans/demo/  ", "abc"), "/g/api/plans/demo");
});
```

`crates/fu-gantt-web/web/package.json` の `scripts` に足す:

```json
    "test:unit": "node --test test/base.test.mjs"
```

Run: `cd crates/fu-gantt-web/web && npm run test:unit`
Expected: FAIL（`src/base.ts` が無い）

- [ ] **Step 2: `base.ts` を書く**

`crates/fu-gantt-web/web/src/base.ts`:

```ts
/**
 * Where this grid's API lives.
 *
 * A host says so with `data-api` on the element. One that says nothing gets
 * the address fugantt has always used, built from the project's id.
 */
export function apiBase(given: string | undefined, projectId: string): string {
  const base = (given ?? "").trim().replace(/\/+$/, "");

  return base !== "" ? base : `/api/projects/${encodeURIComponent(projectId)}`;
}
```

Run: `npm run test:unit`
Expected: 4 passed

- [ ] **Step 3: `grid.ts` の URL を基点から組み立てる**

`grid.ts` の `import "./grid.css";` の次に:

```ts
import { apiBase } from "./base";
```

`Grid` のコンストラクター（1104行目付近）に引数を1つ足す:

```ts
  constructor(
    private readonly root: HTMLElement,
    private readonly projectId: string,
    /** Where the API lives, without a trailing slash. */
    private readonly api: string,
    private data: GridData,
  ) {
```

クラスの中の URL を置き換える（macOS の `sed`）:

```bash
cd crates/fu-gantt-web/web
sed -i '' 's#`/api/projects/${encodeURIComponent(this.projectId)}/#`${this.api}/#g' src/grid.ts
```

`start()`（ファイル末尾）を次にする:

```ts
async function start(): Promise<void> {
  const root = document.getElementById("fugantt-grid");
  if (!root) return;

  const projectId = root.dataset["project"];
  if (!projectId) return;

  const api = apiBase(root.dataset["api"], projectId);

  try {
    const response = await fetch(`${api}/grid`, {
      headers: { accept: "application/json" },
    });

    if (!response.ok) throw new Error(`HTTP ${response.status}`);

    new Grid(root, projectId, api, (await response.json()) as GridData);
  } catch (error) {
    root.replaceChildren(
      element("p", "fg-empty", t("スケジュールを読み込めませんでした。再読み込みしてください。")),
    );
    console.error("fugantt: failed to load the grid", error);
  }
}
```

Run: `grep -n "/api/projects" src/grid.ts src/base.ts`
Expected: `src/base.ts` の1行だけ（`grid.ts` にはコメントを除いて残らない。コメントに残っている場合は、いまの挙動に合うように直す）

Run: `grep -c 'this\.api' src/grid.ts`
Expected: 置き換え前に `grep -c '/api/projects/${encodeURIComponent(this.projectId)}' ` で数えた件数（調査時点で約30）と同じ

- [ ] **Step 4: 絞り込み件数の要素をセレクターで受け取る**

`updateFilterCount()`（1289行目付近）の最初の2行を次にする:

```ts
  /** Wires the header's filter box, which lives outside the island's markup. */
  private updateFilterCount(): void {
    // The host says where its counter is. One that does not gets the id
    // fugantt's own header uses.
    const where = this.root.dataset["filterCount"]?.trim() || "#fugantt-filter-count";
    const label = document.querySelector<HTMLElement>(where);
    if (!label) return;
```

- [ ] **Step 5: ビルドして、既定の経路が変わっていないことを確かめる**

```bash
npm run typecheck && npm run test:unit && npm run build
cd -
cargo test --workspace
```

Expected: PASS。`golden` 7（fugantt のページは属性を足していないので、HTML の記録は変わらない）

端末を2つ使い、Task 7 Step 7 と同じ手順でブラウザーのテストを通す。

Expected: 失敗 0。`fugantt-filter-count` を見ている2件（`grid.test.mjs` の722行目と2602行目付近）を含めて通る

- [ ] **Step 6: CI に足す**

`.github/workflows/ci.yml` の steps の末尾に:

```yaml
      # The one part of the grid that can be checked without a browser, and
      # that the committed bundle is what the source builds.
      - uses: actions/setup-node@v4
        with:
          node-version: 22
      - run: npm ci
        working-directory: crates/fu-gantt-web/web
      - run: npm run typecheck && npm run test:unit && npm run build
        working-directory: crates/fu-gantt-web/web
      - run: git diff --exit-code crates/fu-gantt-web/web/dist
```

ファイル冒頭のコメント（「What can be checked without a browser.」）は実態に合っているのでそのまま。

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Let a host say where the grid's API lives

data-api names the base address, and data-filter-count the element the
filter count is written into. A page that says neither is fugantt's own,
and behaves as it did.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 9: fugantt 以外からグリッドを動かす見本

DB も認証も持たない最小のホスト。`fu-gantt-core` と `fu-gantt-web` だけで、別の基点（`/g/api/plans/demo`）でグリッドが動くことを確かめる。HTTP サーバーには、すでにビルドに入っている Topcoat を使う（新しい依存を増やさないため）。

**Files:**
- Create: `examples/minimal-host/Cargo.toml`、`examples/minimal-host/src/main.rs`、`examples/minimal-host/tests/smoke.rs`
- Modify: `Cargo.toml`（members）、`Dockerfile` は変更なし（`-p fugantt` なので見本はビルドされない）

**Interfaces:**
- Consumes: `fu_gantt_core::domain::{build, GridData, Settings, TaskRow}`、`fu_gantt_core::sortkey::between`、`fu_gantt_core::text::date`、`fu_gantt_core::wire::{CellEdit, CellField, InsertTask, Move, MoveRequest, Mutation}`、`fu_gantt_web::{GRID_CSS, GRID_JS}`
- Produces: 実行ファイル `minimal-host`。`PORT`（既定はTopcoat の既定）で待ち受け、次に答える
  - `GET /` ページ
  - `GET /g/grid.js`、`GET /g/grid.css`
  - `GET /g/api/plans/demo/grid`
  - `POST /g/api/plans/demo/tasks`（行の追加）
  - `POST /g/api/plans/demo/tasks/{task_id}`（名前・開始・終了・予定期間・進捗の変更）
  - `POST /g/api/plans/demo/tasks/{task_id}/move`（上下の入れ替え）

- [ ] **Step 1: 煙テストを書く（実行ファイルが無いので失敗する）**

ルートの `Cargo.toml` の members に `"examples/minimal-host"` を足す。

`examples/minimal-host/Cargo.toml`:

```toml
[package]
name = "minimal-host"
version = "0.0.0"
description = "The smallest thing that can show the grid: no database, no accounts."
edition.workspace = true
license.workspace = true
publish = false

[dependencies]
fu-gantt-core = { path = "../../crates/fu-gantt-core" }
fu-gantt-web = { path = "../../crates/fu-gantt-web" }
jiff.workspace = true
serde_json.workspace = true
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
topcoat = "0.5"
```

`examples/minimal-host/tests/smoke.rs`:

```rust
//! The example, asked the things the grid asks.

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Host {
    child: Child,
    port: u16,
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Host {
    fn start() -> Self {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();

        let child = Command::new(env!("CARGO_BIN_EXE_minimal-host"))
            .env("PORT", port.to_string())
            .env("HOST", "127.0.0.1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        Self { child, port }
    }

    /// The status and the body of one request.
    fn ask(&self, method: &str, path: &str, body: Option<&str>) -> (u16, String) {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut stream = loop {
            match TcpStream::connect(("127.0.0.1", self.port)) {
                Ok(stream) => break stream,
                Err(error) if Instant::now() > deadline => panic!("起動しなかった: {error}"),
                Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        };

        let body = body.unwrap_or("");
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            self.port,
            body.len()
        )
        .unwrap();

        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();

        let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = String::from_utf8_lossy(&raw[..split]).to_lowercase();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        let rest = &raw[split + 4..];
        let bytes = if head.contains("transfer-encoding: chunked") {
            unchunk(rest)
        } else {
            rest.to_vec()
        };

        (status, String::from_utf8(bytes).unwrap())
    }

    fn json(&self, method: &str, path: &str, body: Option<&str>) -> serde_json::Value {
        let (status, text) = self.ask(method, path, body);
        assert_eq!(status, 200, "{method} {path}: {text}");
        serde_json::from_str(&text).unwrap()
    }
}

fn unchunk(mut rest: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    while let Some(line_end) = rest.windows(2).position(|w| w == b"\r\n") {
        let size = std::str::from_utf8(&rest[..line_end])
            .ok()
            .and_then(|line| usize::from_str_radix(line.trim(), 16).ok())
            .unwrap_or(0);
        if size == 0 {
            break;
        }

        let start = line_end + 2;
        out.extend_from_slice(&rest[start..start + size]);
        rest = &rest[start + size + 2..];
    }

    out
}

const API: &str = "/g/api/plans/demo";

fn names(grid: &serde_json::Value) -> Vec<String> {
    grid["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|task| task["name"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn the_page_points_the_grid_at_this_host_s_own_address() {
    let host = Host::start();
    let (status, page) = host.ask("GET", "/", None);

    assert_eq!(status, 200);
    assert!(page.contains(r#"id="fugantt-grid""#), "{page}");
    assert!(page.contains(&format!(r#"data-api="{API}""#)), "{page}");

    let (status, script) = host.ask("GET", "/g/grid.js", None);
    assert_eq!(status, 200);
    assert!(script.contains("fugantt-grid"));
}

/// Nothing answers at the address fugantt itself uses: the grid is here
/// because it was told where to look.
#[test]
fn fugantt_s_own_address_is_not_here() {
    let host = Host::start();

    assert_eq!(host.ask("GET", "/api/projects/demo/grid", None).0, 404);
}

#[test]
fn the_plan_is_read_added_to_edited_and_reordered() {
    let host = Host::start();

    let grid = host.json("GET", &format!("{API}/grid"), None);
    assert_eq!(names(&grid), ["設計", "実装", "テスト"]);
    let first = grid["tasks"][0]["id"].as_str().unwrap().to_owned();

    // A row added after the first lands second.
    let added = host.json(
        "POST",
        &format!("{API}/tasks"),
        Some(&format!(r#"{{"after":"{first}"}}"#)),
    );
    let new = added["task_id"].as_str().unwrap().to_owned();
    assert_eq!(names(&added["grid"]).len(), 4);
    assert_eq!(added["grid"]["tasks"][1]["id"], new);

    // Named, and given dates the way a person types them.
    host.json(
        "POST",
        &format!("{API}/tasks/{new}"),
        Some(r#"{"field":"name","value":"レビュー"}"#),
    );
    let edited = host.json(
        "POST",
        &format!("{API}/tasks/{new}"),
        Some(r#"{"field":"schedule","value":"2026-10-05/20261009"}"#),
    );
    let row = &edited["grid"]["tasks"][1];
    assert_eq!(row["name"], "レビュー");
    assert_eq!(row["start_date"], "2026-10-05");
    assert_eq!(row["end_date"], "2026-10-09");

    // Moved up, it is first.
    let moved = host.json(
        "POST",
        &format!("{API}/tasks/{new}/move"),
        Some(r#"{"action":"up"}"#),
    );
    assert_eq!(names(&moved["grid"])[0], "レビュー");

    // The revision moved with every write, so a watcher would know.
    assert!(moved["grid"]["revision"].as_i64() > grid["revision"].as_i64());
}

#[test]
fn text_that_is_not_a_date_is_refused_and_changes_nothing() {
    let host = Host::start();
    let grid = host.json("GET", &format!("{API}/grid"), None);
    let first = grid["tasks"][0]["id"].as_str().unwrap();

    let (status, _) = host.ask(
        "POST",
        &format!("{API}/tasks/{first}"),
        Some(r#"{"field":"start","value":"あした"}"#),
    );
    assert_eq!(status, 400);

    let after = host.json("GET", &format!("{API}/grid"), None);
    assert_eq!(after["tasks"][0]["start_date"], grid["tasks"][0]["start_date"]);
    assert_eq!(after["revision"], grid["revision"]);
}
```

`start_date`・`end_date`・`name`・`id`・`revision` は `GridData`／`TaskView` が JSON に出すキー。違う名前で出ている場合は `apps/fugantt/tests/golden/grid.json` を見て合わせる。

Run: `cargo test -p minimal-host`
Expected: FAIL（`src/main.rs` が無い）

- [ ] **Step 2: 見本のホストを書く**

`examples/minimal-host/src/main.rs`。Topcoat の `use` は `apps/fugantt/src/api.rs` と `apps/fugantt/src/main.rs` の書き方に合わせてある。

```rust
//! The smallest thing that can show the grid.
//!
//! No database and no accounts: three rows held in memory, and the handful of
//! addresses the grid needs to draw them and change them. It is here to show
//! what a host has to supply, and to prove the grid runs somewhere that is not
//! fugantt — at an address of this program's own choosing.
//!
//!     cargo run -p minimal-host
//!
//! What it leaves out, a real host answers too: placing a row by dragging,
//! deleting, one-row patches, live updates, saved filters and leave. See
//! `docs/gantt-api.md`.

use std::sync::{Mutex, PoisonError};

use fu_gantt_core::{
    domain::{self, GridData, Settings, TaskRow},
    sortkey, text,
    wire::{CellEdit, CellField, InsertTask, Move, MoveRequest, Mutation},
};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Body, Response, Router, RouterBuilderDiscoverExt,
        content::Json,
        error::{bad_request, not_found},
        raw_path_params, route,
    },
};

/// Where the grid is told to look. Deliberately not `/api/projects/…`.
const API: &str = "/g/api/plans/demo";

struct Plan(Mutex<State>);

struct State {
    revision: i64,
    next_id: u32,
    rows: Vec<TaskRow>,
}

fn row(id: &str, sort_key: &str, name: &str, start: &str, end: &str) -> TaskRow {
    let date = |text: &str| (!text.is_empty()).then(|| text.to_owned());

    TaskRow {
        id: id.to_owned(),
        parent_id: None,
        sort_key: sort_key.to_owned(),
        name: name.to_owned(),
        start_date: date(start),
        end_date: date(end),
        due: None,
        actual_start: None,
        actual_end: None,
        progress: 0,
        tags: String::new(),
        status: String::new(),
        assignee: String::new(),
        note: String::new(),
        waits: String::new(),
        targets: String::new(),
        color: String::new(),
        background: String::new(),
    }
}

impl State {
    fn new() -> Self {
        let first = sortkey::between(None, None);
        let second = sortkey::between(Some(&first), None);
        let third = sortkey::between(Some(&second), None);

        Self {
            revision: 0,
            next_id: 4,
            rows: vec![
                row("t-1", &first, "設計", "2026-10-01", "2026-10-09"),
                row("t-2", &second, "実装", "2026-10-12", "2026-10-30"),
                row("t-3", &third, "テスト", "2026-11-02", "2026-11-13"),
            ],
        }
    }

    /// The whole plan, worked out afresh.
    fn grid(&self) -> GridData {
        let mut rows = self.rows.clone();
        rows.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));

        domain::build(
            "demo",
            self.revision,
            jiff::Zoned::now().date(),
            rows,
            Settings::default(),
        )
    }

    /// The rows in the order they are drawn.
    fn ordered(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.rows.len()).collect();
        order.sort_by(|a, b| self.rows[*a].sort_key.cmp(&self.rows[*b].sort_key));
        order
    }

    /// What every write answers with: the new plan, and the row it was about.
    fn changed(&mut self, task_id: &str, note: Option<&'static str>) -> Mutation {
        if note.is_none() {
            self.revision += 1;
        }

        Mutation {
            grid: Some(self.grid()),
            patch: None,
            task_id: Some(task_id.to_owned()),
            note,
        }
    }
}

fn plan(cx: &Cx) -> std::sync::MutexGuard<'_, State> {
    app_context::<Plan>(cx)
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

fn task_id(cx: &Cx) -> Result<String> {
    raw_path_params(cx)
        .iter()
        .find(|(key, _)| *key == "task_id")
        .map(|(_, value)| value.to_string())
        .ok_or_else(|| not_found().into())
}

fn file(content_type: &str, body: &'static str) -> Result<Response> {
    Ok(Response::builder()
        .header("Content-Type", content_type)
        .body(Body::from(body))?)
}

#[route(GET "/")]
async fn page(_cx: &Cx) -> Result<Response> {
    let html = format!(
        r##"<!doctype html>
<html lang="ja">
<meta charset="utf-8">
<title>minimal-host</title>
<link rel="stylesheet" href="/g/grid.css">
<style>
  body {{ margin: 0; font-family: system-ui, sans-serif; }}
  header {{ padding: 8px 16px; }}
  #fugantt-grid {{ height: calc(100vh - 48px); display: flex; flex-direction: column; }}
</style>
<header>minimal-host <span id="count"></span></header>
<div id="fugantt-grid" data-project="demo" data-api="{API}" data-filter-count="#count"></div>
<script src="/g/grid.js" defer></script>
</html>"##
    );

    Ok(Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(html))?)
}

#[route(GET "/g/grid.js")]
async fn grid_js(_cx: &Cx) -> Result<Response> {
    file("text/javascript; charset=utf-8", fu_gantt_web::GRID_JS)
}

#[route(GET "/g/grid.css")]
async fn grid_css(_cx: &Cx) -> Result<Response> {
    file("text/css; charset=utf-8", fu_gantt_web::GRID_CSS)
}

#[route(GET "/g/api/plans/demo/grid")]
async fn grid(cx: &Cx) -> Result<Json<GridData>> {
    Ok(Json(plan(cx).grid()))
}

#[route(POST "/g/api/plans/demo/tasks")]
async fn insert(cx: &Cx, Json(insert): Json<InsertTask>) -> Result<Json<Mutation>> {
    let mut plan = plan(cx);
    let order = plan.ordered();

    // After the named row, or at the end.
    let at = insert
        .after
        .as_deref()
        .and_then(|after| order.iter().position(|row| plan.rows[*row].id == after));
    let (before, after) = match at {
        Some(at) => (Some(order[at]), order.get(at + 1).copied()),
        None => (order.last().copied(), None),
    };
    let key = sortkey::between(
        before.map(|row| plan.rows[row].sort_key.as_str()),
        after.map(|row| plan.rows[row].sort_key.as_str()),
    );

    let id = format!("t-{}", plan.next_id);
    plan.next_id += 1;
    plan.rows.push(row(&id, &key, "", "", ""));

    Ok(Json(plan.changed(&id, None)))
}

#[route(POST "/g/api/plans/demo/tasks/{task_id}")]
async fn edit(cx: &Cx, Json(edit): Json<CellEdit>) -> Result<Json<Mutation>> {
    let id = task_id(cx)?;
    let today = jiff::Zoned::now().date();
    let mut plan = plan(cx);

    // Read before anything is written, so a refusal leaves the row as it was.
    let read = |value: &str| {
        text::date(value, today).map_err(|_| bad_request("日付として読めません。"))
    };

    enum Write {
        Name(String),
        Start(Option<String>),
        End(Option<String>),
        Both(Option<String>, Option<String>),
        Progress(i64),
    }

    let write = match edit.field {
        CellField::Name => Write::Name(edit.value.trim().to_owned()),
        CellField::Start => Write::Start(read(&edit.value)?),
        CellField::End => Write::End(read(&edit.value)?),
        CellField::Schedule => {
            let (start, end) = edit.value.split_once('/').unwrap_or((&edit.value, ""));
            Write::Both(read(start)?, read(end)?)
        }
        CellField::Progress => Write::Progress(
            edit.value
                .trim()
                .trim_end_matches('%')
                .parse::<i64>()
                .ok()
                .filter(|percent| (0..=100).contains(percent))
                .ok_or_else(|| bad_request("進捗は0〜100で入力してください。"))?,
        ),
        _ => return Err(bad_request("この見本では変えられない列です。").into()),
    };

    let Some(task) = plan.rows.iter_mut().find(|row| row.id == id) else {
        return Err(not_found().into());
    };

    match write {
        Write::Name(name) => task.name = name,
        Write::Start(start) => task.start_date = start,
        Write::End(end) => task.end_date = end,
        Write::Both(start, end) => {
            task.start_date = start;
            task.end_date = end;
        }
        Write::Progress(percent) => task.progress = percent,
    }

    Ok(Json(plan.changed(&id, None)))
}

#[route(POST "/g/api/plans/demo/tasks/{task_id}/move")]
async fn reorder(cx: &Cx, Json(request): Json<MoveRequest>) -> Result<Json<Mutation>> {
    let id = task_id(cx)?;
    let mut plan = plan(cx);
    let order = plan.ordered();

    let Some(at) = order.iter().position(|row| plan.rows[*row].id == id) else {
        return Err(not_found().into());
    };

    let other = match request.action {
        Move::Up => at.checked_sub(1),
        Move::Down => (at + 1 < order.len()).then_some(at + 1),
        // A flat list: there is no outline to move through.
        Move::Indent | Move::Outdent => {
            return Ok(Json(plan.changed(&id, Some("この見本には階層がありません。"))));
        }
    };

    let Some(other) = other else {
        return Ok(Json(plan.changed(&id, Some("これ以上は動かせません。"))));
    };

    // Changing places is trading keys.
    let (a, b) = (order[at], order[other]);
    let key = plan.rows[a].sort_key.clone();
    plan.rows[a].sort_key = std::mem::replace(&mut plan.rows[b].sort_key, key);

    Ok(Json(plan.changed(&id, None)))
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let router = Router::builder()
        .app_context(Plan(Mutex::new(State::new())))
        .discover()
        .build();

    topcoat::start(router).await?;

    Ok(())
}
```

Topcoat の細部がコンパイルで合わなかったときの直し方（**見本のホストの中だけを直す。`fu-gantt-core` と `fu-gantt-web` には手を入れない**）:

- `not_found` が `topcoat::router::error` に無い → `apps/fugantt/src/project.rs` の `path_str` が使っている `ok_or_not_found()` と、その `use` を真似る
- `raw_path_params(cx)` の要素の型が違う → `apps/fugantt/src/project.rs:76` の `path_str` と同じ取り出し方にする
- `Response::builder()…body(Body::from(String))` が通らない → `apps/fugantt/src/static_files.rs` の `serve` と同じ形にする
- `bad_request(…)` を `?` で返せない → `apps/fugantt/src/api.rs` と同じく `.into()` を付ける
- `TaskRow` のフィールドが足りない／余る → `crates/fu-gantt-core/src/domain.rs` の `TaskRow` の定義に合わせる

- [ ] **Step 3: 煙テストを通す**

Run: `cargo test -p minimal-host`
Expected: 4 passed

- [ ] **Step 4: 依存が約束どおりであることを確かめる**

Run: `cargo tree -p minimal-host --edges normal --depth 1`
Expected: `fu-gantt-core`・`fu-gantt-web`・`jiff`・`serde_json`・`tokio`・`topcoat` だけ。`sqlx`・`fugantt` が無い

Run: `cargo tree -p minimal-host --edges normal | grep -c sqlx`
Expected: `0`

- [ ] **Step 5: ブラウザーで動かして確かめる**

Run: `PORT=18620 cargo run -p minimal-host`、ブラウザーで `http://127.0.0.1:18620/` を開く

確かめること（開発者ツールのネットワークタブを開いておく）:

- 3行のグリッドとバーが描かれる
- 名前のセルを書き換えて確定すると残る
- 開始のセルに `10/5` と打つと `2026-10-05` になる
- 行を追加できる（グリッドのツールバーまたはキー操作）
- 行を上下に動かせる
- 通信がすべて `/g/api/plans/demo/` と `/g/grid.*` 宛てで、`/api/projects/` 宛てが1件も無い（`live` は 404 になるが、画面には何も出ない）

どれかが動かない場合、原因が見本のホストの不足（実装していないエンドポイントを叩いている）なら、その操作を上の一覧から外し、`main.rs` 冒頭の「What it leaves out」に足す。原因が `grid.ts` の側（基点を使っていない URL が残っている）なら Task 8 に戻って直す。

- [ ] **Step 6: 全部通して Commit**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

```bash
git add -A
git commit -m "Show the grid running somewhere that is not fugantt

Three rows in memory behind an address of the example's own. It depends
on the two crates and a web server, and on nothing that stores a plan.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 10: 約束を文書にして 1.1.0 にする

**Files:**
- Create: `docs/gantt-api.md`
- Modify: `README.md`・`README.en.md`（開発の節）、`docs/design.md`・`docs/design.en.md`（構成に触れている箇所があれば）、`apps/fugantt/Cargo.toml`（version）、`Cargo.lock`

**Interfaces:**
- Consumes: Task 6 の型、Task 8 の属性、Task 9 の見本

- [ ] **Step 1: `docs/gantt-api.md` を書く**

次の内容で書く。表の「入力」「出力」は `crates/fu-gantt-core/src/wire.rs` の型名。ステータスコードと `x-fugantt-client` の扱いは、`apps/fugantt/src/api.rs` の該当ハンドラーを読んで確かめた値を書く。

````markdown
# ガントのグリッドを組み込む

fugantt のグリッドは、fugantt 以外のアプリにも組み込める。必要なのは、ページに要素を1つ置く
ことと、JSON の API を用意すること。保存・認証・権限は組み込む側が持つ。

動く最小の例が `examples/minimal-host` にある。

## 使うクレート

```toml
fu-gantt-core = { git = "https://github.com/fu-foo/fugantt", tag = "v1.1.0" }
fu-gantt-web  = { git = "https://github.com/fu-foo/fugantt", tag = "v1.1.0" }
```

| クレート | 中身 | 依存 |
|---|---|---|
| `fu-gantt-core` | 計算（`domain`）、並び順のキー（`sortkey`）、セルの文字の読み取り（`text`）、JSON の型（`wire`） | `jiff`、`serde`。`sqlx` フィーチャーで `TaskRow` に `FromRow` |
| `fu-gantt-web` | `GRID_JS`、`GRID_CSS` とそのハッシュ | なし |

2つは同じタグから取る。グリッドの JS と JSON の型は対で変わる。

## ページに置くもの

```html
<link rel="stylesheet" href="（GRID_CSS を返す URL）">
<div id="fugantt-grid"
     data-project="abc"
     data-api="/gantt/api/projects/abc"
     data-filter-count="#my-counter"></div>
<script src="（GRID_JS を返す URL）" defer></script>
```

| 属性 | 必須 | 意味 |
|---|---|---|
| `id="fugantt-grid"` | ○ | グリッドが描く先。1ページに1つ |
| `data-project` | ○ | プロジェクトの識別子。折りたたみ状態をブラウザーに覚えさせるキーにも使う |
| `data-api` | | API の基点。無ければ `/api/projects/{data-project}` |
| `data-filter-count` | | 絞り込み件数を書く要素の CSS セレクター。無ければ `#fugantt-filter-count`。要素が無ければ何も書かない |

要素には高さが要る（グリッドは親の高さいっぱいに広がる）。

配色は `.fg-grid` の `--fg-*` プロパティで変えられる。fugantt 自身がどう結び付けているかは
`apps/fugantt/assets/theme.css` の「the grid island」の節が見本になる。

## API

基点（`data-api`）からの相対パス。入出力は JSON。

| メソッド | パス | 入力 | 出力 | 何をするか |
|---|---|---|---|---|
| GET | `/grid` | — | `GridData` | 計画の全体 |
| POST | `/tasks` | `InsertTask` | `Mutation` | 行を足す |
| POST | `/tasks/{id}` | `CellEdit` | `Mutation` | セルを1つ書く |
| DELETE | `/tasks/{id}` | — | `Mutation` | 行を消す（子も） |
| POST | `/tasks/{id}/move` | `MoveRequest` | `Mutation` | 字下げ・字上げ・上下 |
| POST | `/tasks/{id}/place` | `PlaceRequest` | `Mutation` | 親と位置を指定して置く |
| GET | `/tasks/{id}/patch` | — | `Patch` | 1行と、その上の集計行 |
| GET | `/live` | — | SSE（`change` イベント、データは `LiveChange`） | 他の人の変更の通知 |
| POST | `/filters` | `SaveFilterSet` | `Mutation` | 絞り込み条件に名前を付けて残す |
| POST | `/filters/remove` | `RemoveFilterSet` | `Mutation` | それを消す |
| POST | `/leaves` | `LeaveList` | `Mutation` | 休みの一覧を置き換える |

全部を実装しなくても描ける。`/grid` だけで表示でき、書き込みは使わせたい操作の分だけ足せばよい。
実装していないパスが 404 を返すと、その操作は失敗としてグリッドに出る。`/live` だけは 404 でも
何も出ない（他の人の変更が自動では届かなくなるだけ）。

## 計画を組み立てる

```rust
let grid: GridData = fu_gantt_core::domain::build(project_id, revision, today, rows, settings);
```

- `rows: Vec<TaskRow>` は保存してある行そのまま。`sort_key` の昇順で渡す
- `settings: Settings` は祝日・休み・担当者・ステータス・表示の設定。`Settings::default()` から始められる
- `today` はホストが決める。「遅れ」は読んでいる人の暦の話なので、fugantt はサーバーのローカル時刻の日付を使っている

行の並びは `sort_key`（文字列）で持つ。2つの行の間に入れるキーは
`fu_gantt_core::sortkey::between(前のキー, 後ろのキー)` で作る。

## セルの文字を読む

グリッドは、日付を打たれたままの文字で送る（`8/5`、`0805`、`2026-08-05`）。読むのはホストの仕事で、
読み方をどのホストでも同じにするために `fu_gantt_core::text` を使う。

| `CellEdit.field` | 読む関数 | 保存する形 |
|---|---|---|
| `start`・`end`・`due`・`actual_start`・`actual_end` | `text::date(value, today)` | `YYYY-MM-DD` か空 |
| `schedule`・`actual_schedule` | `/` で分けて、それぞれ `text::date` | 同上を2つ |
| `waits` | `text::waits(value, today)` | 1行に `開始/終了:理由` |
| `targets` | `text::targets(value, today)` | 1行に `日付/パーセント` |
| `color`・`background` | `text::colour(value)` | `#rrggbb` か空 |
| `name`・`status`・`assignee`・`note`・`progress`・`custom` | ホストが決める | — |

読めなかったときは `TextError` が返る。文言はホストが付け、400 と本文（文字列）で返す。
グリッドは本文をそのまま表示し、打った値を元に戻す。

`CellEdit.expect` が付いているときは、取り消し（Ctrl+Z）の書き戻し。いまの値が `expect` と
違っていたら書かずに断る（他の人の変更を黙って消さないため）。

## ホストが守ること

- **`revision` は変更のたびに進める。** グリッドは自分の持っている `revision` より新しい通知だけを取り込む
- **何も変わらなかった書き込みでは `revision` を進めない。** `Mutation.note` に理由を入れて返す
- **`Mutation` は `grid`（全体）か `patch`（変わった行だけ）のどちらかを返す。** 迷ったら `grid` でよい。
  `patch` は、1行の書き込みで計画全体を返さないための最適化
- **`Patch.total` はいまの行数。** グリッドは手元の行数と比べ、合わなければ `/grid` を読み直す
- **`/live` は変更を `change` イベントで流す。** `LiveChange.kind` は、1行の値が変わっただけなら
  `"cell"`、行が増減・移動したなら `"plan"`
- **書き込みのリクエストには `x-fugantt-client` ヘッダーが付く。** その値を `LiveChange.client` に
  入れて返すと、書いた本人のブラウザーが自分の変更の通知を無視できる
- **権限の検査はホストの仕事。** グリッドは、返ってきた `GridData` の内容どおりに描くだけ

## 版

`fu-gantt-core` と `fu-gantt-web` は 0.x の間、JSON の形を変えることがある。変えるときは
両方を同じコミットで変える。タグを固定して使うこと。
````

書いたあと、表の内容を実物と突き合わせる:

Run: `grep -n '#\[route(.*"/api/projects/{project_id}/' apps/fugantt/src/api.rs`
Expected: 上の表の11行（`grid`・`tasks`・`tasks/{task_id}` の POST と DELETE・`move`・`place`・`patch`・`live`・`filters`・`filters/remove`・`leaves`）と、表に載せていない `document` の2行。`document` は取り込み・書き出し用でグリッドは叩かないので載せない

Run: `grep -o '\${this\.api}/[a-z/${}.A-Za-z]*' crates/fu-gantt-web/web/src/grid.ts | sort -u`
Expected: 表のパスと過不足なく対応する。表に無いパスを叩いていたら表に足す

`CellEdit.field` の表の値は `wire.rs` の `CellField` の各バリアント（`snake_case`）と過不足なく対応させる。`GridData` の権限に関する記述など、確かめられなかったことは書かない。

- [ ] **Step 2: README の開発手順を新しい配置に合わせる**

`README.md` と `README.en.md` の「開発」の節（`cargo-topcoat dev` が出てくるあたり、README.md の228行目付近）を、Task 3 Step 7 で確かめた起動の仕方に直す。`web/` のビルド手順があれば `crates/fu-gantt-web/web` に直す。

同じ節の末尾に、構成の説明を足す（英語版は同じ内容を英語で）:

```markdown
### リポジトリの構成

| 場所 | 中身 |
|---|---|
| `apps/fugantt` | fugantt 本体。DB・認証・ページ・API |
| `crates/fu-calendar` | 日本の祝日の計算。依存なし |
| `crates/fu-gantt-core` | ガントの計算と、グリッドとやり取りする JSON の型 |
| `crates/fu-gantt-web` | グリッドの画面（`grid.ts`）とそのビルド結果 |
| `examples/minimal-host` | fugantt 以外からグリッドを動かす最小の例 |

グリッドを別のアプリに組み込む方法は [docs/gantt-api.md](docs/gantt-api.md) にある。
```

Run: `grep -rn "web/\|src/\|migrations/" README.md README.en.md docs/guide.md docs/guide.en.md docs/reference.md docs/reference.en.md docs/design.md docs/design.en.md | grep -v "crates/\|apps/"`
Expected: 古い配置を指しているパスが残っていない。残っていたら新しいパスに直す（`docs/reference.md:219` の `web/test/measure.mjs` → `crates/fu-gantt-web/web/test/measure.mjs` など）

- [ ] **Step 3: 版を 1.1.0 にする**

`apps/fugantt/Cargo.toml`: `version = "1.0.1"` → `version = "1.1.0"`

Run: `cargo build -p fugantt && cargo run -p fugantt -- version`
Expected: `fugantt 1.1.0`

- [ ] **Step 4: 最後に全部通す**

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p fu-calendar --all-features
cargo test -p fu-gantt-core --no-default-features
cargo build --release --locked -p fugantt
(cd crates/fu-gantt-web/web && npm run typecheck && npm run test:unit && npm run build)
git diff --exit-code crates/fu-gantt-web/web/dist
docker build -t fugantt:check .
```

Expected: すべて成功。`golden` 7 は Task 2 で取った記録のまま一致（版の文字列は記録の中で `VERSION` に置き換えてあるので、1.1.0 にしても変わらない）

テストの本数の確認（移した先で減っていないこと）:

| 場所 | 本数 |
|---|---|
| `fugantt` 単体 | 76 |
| `fugantt` `cli` | 4 |
| `fugantt` `golden` | 7 |
| `fu-calendar` | 9（うち移した5） |
| `fu-gantt-core` | 42（うち移した34） |
| `fu-gantt-web` | 3 |
| `minimal-host` | 4 |

元の単体 115本 = 76 + 5 + 34。`static_files.rs` の2本は残したまま中身を1本分 `fu-gantt-web` に写しているので、数は変わらない。

ブラウザーのテスト（Task 7 Step 7 の手順）を最後にもう一度通す。

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "Cut 1.1.0

Nothing changes for whoever runs it. The chart's arithmetic and its grid
are crates now, and docs/gantt-api.md says what a host owes them.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

タグを打ってリリースするかどうかは、このタスクの範囲外（人が決める）。

---

## Self-Review の結果

- **仕様との対応**: 構成 → Task 3・4・5・7 / `fu-calendar` → Task 4 / `fu-gantt-core`（計算・型・文字の読み取り）→ Task 5・6 / `fu-gantt-web` → Task 7 / ホストとの約束 1・2 → Task 8、3 → Task 6・10 / 順番 1〜6 → Task 1+2・3・4・5+6・7+8+9・10 / 確かめ方 → 各タスクの末尾と Task 10 Step 4
- **仕様に無くて足したもの**: Task 1（今日の固定）。仕様の「段1」に書いてある前提を独立したタスクにした
- **型と名前の整合**: `CellField`（`api.rs` では `as Field`）、`LiveChange`（`live.rs` では `as Change`）、`text::date`／`waits`／`targets`／`colour`／`flexible_date`、`fu_gantt_web::{GRID_JS, GRID_CSS, fingerprint, grid_js_hash, grid_css_hash}`、`apiBase(given, projectId)` は、定義したタスクと使うタスクで同じ
- **実物を読まずに書いた箇所**（実装時に確かめる手順をその場に書いてある）: `normalize_width` の本体（Task 6 は切り貼りで移す）、Topcoat のエラー型の名前（Task 6 Step 3）、`cargo-topcoat dev` が見るディレクトリー（Task 3 Step 7）、`TaskView` の JSON のキー名（Task 9 Step 1）
