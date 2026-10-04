# グリッドに条件と行のリンクを渡す Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `fu-gantt-web` のグリッドに `data-query` と `data-row-link` を足し、fugantt 自身は変えずに v1.2.0 にする。

**Architecture:** URL の組み立てと行のリンクの組み立てを `base.ts` の純粋な関数にして単体テストで固定し、`grid.ts` はそれを呼ぶだけにする。見本のホスト（`examples/minimal-host`）に2つの属性を付け、fugantt 以外で効くことを煙テストとブラウザーテストで確かめる。

**Tech Stack:** TypeScript + esbuild / puppeteer-core / Rust 2024 / Topcoat 0.5

**Spec:** `docs/superpowers/specs/2026-10-04-grid-query-and-row-link-design.md`

## Global Constraints

- fugantt のページは2つの属性を付けない。画面・通信・API の形は変えない
- 属性が無いとき、グリッドが叩く URL はこれまでと1文字も変わらない
- `apps/fugantt/tests/golden.rs` の記録は取り直さない
- `fu-gantt-core` の `wire` は変えない
- 版: `fugantt` 1.2.0、`fu-gantt-web` 0.2.0。`fu-gantt-core`・`fu-calendar` は 0.1.0 のまま
- `web/dist` はビルドしてコミットする（CI が `git diff --exit-code` で確かめる）
- コミットメッセージは英語の平叙文、末尾に `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`
- 各タスクの終わりに: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`、`crates/fu-gantt-web/web` で `npm run typecheck && npm run test:unit && npm run build`
- グリッドのブラウザーテストは、変更前と同じ結果（383 通過、「誰が変えたかを知らせる」の1件はもとから失敗）であること

## Review Focus

1. **条件が `?` や `&` で始まっている**（`?status=open`）。`??` や `?&` にならないこと → Task 1
2. **行の id に URL で意味を持つ文字がある**（`a/b`、`a b`、`#1`）。行き先が壊れないこと → Task 1
3. **雛形が `javascript:` で始まる**。リンクを出さないこと → Task 1
4. **リンクを押したとき、セルの選択や編集が始まる／グリッドの横送りが始まる**。普通のリンクとして動くこと → Task 3 のブラウザーテスト
5. **他の人の変更の通知（`live`）で読み直すとき、条件が落ちる**。読み直しの URL にも条件が付いていること → Task 2 Step 3 の確認と Task 3 のブラウザーテスト

---

### Task 1: URL と行き先を組み立てる関数

**Files:**
- Modify: `crates/fu-gantt-web/web/src/base.ts`、`crates/fu-gantt-web/web/test/base.test.mjs`

**Interfaces:**
- Produces:
  - `export function cleanQuery(given: string | undefined): string` — 先頭の `?`・`&` と前後の空白を落とした条件。無ければ `""`
  - `export function withQuery(url: string, query: string): string` — `query` が空なら `url` そのまま、あれば `url?query`
  - `export function rowLink(template: string | undefined, id: string): string | null` — 行き先。雛形が無い、または通せない形なら `null`

- [ ] **Step 1: 失敗するテストを書く**

`crates/fu-gantt-web/web/test/base.test.mjs` の `const { apiBase } = await import(` を
`const { apiBase, cleanQuery, withQuery, rowLink } = await import(` に変え、末尾に足す:

```js
test("with no query, an address is left exactly as it was", () => {
  assert.equal(cleanQuery(undefined), "");
  assert.equal(cleanQuery(""), "");
  assert.equal(cleanQuery("   "), "");
  assert.equal(withQuery("/api/projects/abc/grid", cleanQuery(undefined)), "/api/projects/abc/grid");
});

test("a host's query rides on every address", () => {
  const query = cleanQuery("status=open&assignee=%E4%BD%90%E8%97%A4");

  assert.equal(
    withQuery("/g/api/grid", query),
    "/g/api/grid?status=open&assignee=%E4%BD%90%E8%97%A4",
  );
  assert.equal(withQuery("/g/api/tasks/t-1/patch", query).split("?").length, 2);
});

test("a query written with its own ? or & does not double them", () => {
  assert.equal(cleanQuery("?status=open"), "status=open");
  assert.equal(cleanQuery("&status=open"), "status=open");
  assert.equal(cleanQuery("  ??&status=open  "), "status=open");
  assert.equal(cleanQuery("?"), "");
  assert.equal(withQuery("/x", cleanQuery("?")), "/x");
});

test("a row's link is the template with the row's id in it", () => {
  assert.equal(rowLink("/issues/by-id/{id}", "t-1"), "/issues/by-id/t-1");
  assert.equal(rowLink("https://example.test/i/{id}/view", "t-1"), "https://example.test/i/t-1/view");
  assert.equal(rowLink("/a/{id}/b/{id}", "t-1"), "/a/t-1/b/t-1");
  // No place for the id: every row goes to the same page, as the host said.
  assert.equal(rowLink("/issues", "t-1"), "/issues");
});

test("an id is encoded on the way into the link", () => {
  assert.equal(rowLink("/i/{id}", "a/b"), "/i/a%2Fb");
  assert.equal(rowLink("/i/{id}", "a b"), "/i/a%20b");
  assert.equal(rowLink("/i/{id}", "#1"), "/i/%231");
});

test("no template, or one that is not an address, gives no link", () => {
  assert.equal(rowLink(undefined, "t-1"), null);
  assert.equal(rowLink("", "t-1"), null);
  assert.equal(rowLink("   ", "t-1"), null);
  assert.equal(rowLink("javascript:alert({id})", "t-1"), null);
  assert.equal(rowLink("JavaScript:alert(1)", "t-1"), null);
  assert.equal(rowLink("data:text/html,{id}", "t-1"), null);
  assert.equal(rowLink("issues/{id}", "t-1"), null);
  assert.equal(rowLink("//evil.test/{id}", "t-1"), null);
});
```

Run: `cd crates/fu-gantt-web/web && npm run test:unit`
Expected: FAIL（`cleanQuery is not a function` など。もとの4本は通る）

- [ ] **Step 2: 実装する**

`crates/fu-gantt-web/web/src/base.ts` の末尾に足す:

```ts
/**
 * The query a host wants on every request, without the punctuation in front.
 *
 * A host that shows the chart over a filtered set of rows says which set with
 * `data-query`. It is the host's own string and is not read here — only tidied,
 * so that `?status=open` and `status=open` mean the same thing.
 */
export function cleanQuery(given: string | undefined): string {
  return (given ?? "").trim().replace(/^[?&]+/, "").trim();
}

/** `url` with the host's query on it, or `url` as it stands when there is none. */
export function withQuery(url: string, query: string): string {
  return query === "" ? url : `${url}?${query}`;
}

/**
 * Where a row leads, when the host has said rows lead somewhere.
 *
 * `{id}` in the template is the row's id. Only an address on this site (`/…`)
 * or a full `http`/`https` one is taken: the template goes into an `href`, and
 * anything else there is either a mistake or an attack.
 */
export function rowLink(template: string | undefined, id: string): string | null {
  const given = (template ?? "").trim();
  const local = given.startsWith("/") && !given.startsWith("//");

  if (!local && !/^https?:\/\//i.test(given)) return null;

  return given.split("{id}").join(encodeURIComponent(id));
}
```

Run: `npm run test:unit`
Expected: 10 passed

- [ ] **Step 3: Commit**

```bash
npm run typecheck
git add crates/fu-gantt-web/web/src/base.ts crates/fu-gantt-web/web/test/base.test.mjs
git commit -m "Work out where a query goes and where a row leads

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: グリッドがそれを使う

**Files:**
- Modify: `crates/fu-gantt-web/web/src/grid.ts`、`crates/fu-gantt-web/web/src/grid.css`、`crates/fu-gantt-web/web/dist/*`（ビルド）

**Interfaces:**
- Consumes: Task 1 の `cleanQuery`・`withQuery`・`rowLink`
- Produces（HTML の属性）: `data-query`、`data-row-link`。件名のセルに `a.fg-row-link`

- [ ] **Step 1: すべての URL を1つの関数に通す**

`import { apiBase } from "./base";` を
`import { apiBase, cleanQuery, rowLink, withQuery } from "./base";` にする。

`Grid` のコンストラクターの引数 `private readonly api: string,` の次に足す:

```ts
    /** The host's query, put on every request. Empty when it gave none. */
    private readonly query: string,
```

クラスの中に足す（`listen()` の前）:

```ts
  /** An address under this grid's API, with the host's query on it. */
  private at(path: string): string {
    return withQuery(`${this.api}${path}`, this.query);
  }
```

クラスの中の URL を置き換える。`` `${this.api}/…` `` の形を `` this.at(`/…`) `` にする:

```bash
cd crates/fu-gantt-web/web
python3 - <<'EOF'
import re
p='src/grid.ts'
s=open(p,encoding='utf-8').read()
n=len(re.findall(r'`\$\{this\.api\}(/[^`]*)`',s))
s=re.sub(r'`\$\{this\.api\}(/[^`]*)`',lambda m:'this.at(`'+m.group(1)+'`)',s)
open(p,'w',encoding='utf-8').write(s)
print('replaced',n)
EOF
```

Expected: `replaced 25`

`start()` の中を次にする:

```ts
  const api = apiBase(root.dataset["api"], projectId);
  const query = cleanQuery(root.dataset["query"]);

  try {
    const response = await fetch(withQuery(`${api}/grid`, query), {
      headers: { accept: "application/json" },
    });

    if (!response.ok) throw new Error(`HTTP ${response.status}`);

    new Grid(root, projectId, api, query, (await response.json()) as GridData);
```

Run: `grep -n 'this\.api' src/grid.ts`
Expected: 定義（`private readonly api`）と `at()` の中の1か所だけ

- [ ] **Step 2: 行のリンクを出す**

件名のセルを描くところ（`element("span", "fg-name-text", …)` の `cell.append(text);` の直後）に足す:

```ts
        // A way out to the page the host keeps for this row, when it has one.
        const href = rowLink(this.root.dataset["rowLink"], task.id);
        if (href !== null) {
          const link = element("a", "fg-row-link", "↗") as HTMLAnchorElement;
          link.href = href;
          link.title = t("開く");
          link.setAttribute("aria-label", t("開く"));
          // Not a cell to type in: a press here is only ever a press on a link,
          // so it selects nothing, opens no editor and pulls no pane.
          link.tabIndex = -1;
          for (const kind of ["mousedown", "pointerdown", "dblclick"]) {
            link.addEventListener(kind, (event) => event.stopPropagation());
          }
          cell.append(link);
        }
```

英語の辞書 `EN`（`const EN: Record<string, string> = {`）に1行足す:

```ts
  "開く": "Open",
```

`element()` の戻り値の型が `HTMLElement` で `as HTMLAnchorElement` が通らない場合は、
`document.createElement("a")` で作り、`className` と `textContent` を自分で入れる。

`crates/fu-gantt-web/web/src/grid.css` の `.fg-name-text.is-placeholder` の規則の後ろに足す:

```css
/* The way out to a row's own page. Quiet until the row is pointed at, so a
   table of a hundred rows is not a table of a hundred arrows. */
.fg-row-link {
  margin-left: 6px;
  color: var(--fg-muted);
  text-decoration: none;
  opacity: 0;
}

.fg-row:hover .fg-row-link,
.fg-row-link:focus-visible {
  opacity: 1;
}

.fg-row-link:hover {
  color: var(--fg-text);
}
```

- [ ] **Step 3: ビルドして、属性なしの経路が変わっていないことを確かめる**

```bash
npm run typecheck && npm run test:unit && npm run build
cd ../../..
cargo test --workspace
```

Expected: PASS。`golden` 8 が記録のまま一致

fugantt を相手にグリッドのブラウザーテストを通す（開発サーバーか debug の実行ファイルを起動し、
`FUGANTT_URL` と `FUGANTT_DB` を渡して `node test/grid.test.mjs`）。

Expected: 383 passed, 1 failed（「誰が変えたかを知らせる」。変更前と同じ）

他の人の変更で読み直す経路にも条件が付くことを、コードで確かめる:

Run: `grep -n "this.at(" crates/fu-gantt-web/web/src/grid.ts | grep -c "live\|patch\|grid"`
Expected: 4 以上（`live`、`patch`、`grid` の読み直し2か所）

- [ ] **Step 4: Commit**

```bash
git add crates/fu-gantt-web/web
git commit -m "Let a host hand the grid a query and a link for each row

data-query rides on every request, so a host can show the chart over a
filtered set and be told which set on the way back. data-row-link puts
a way out beside each name. A page that gives neither is unchanged.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: 見本のホストで確かめる

見本に「終わった行を隠す」条件を足す。条件はページの URL（`/?hide_done=1`）で受け取り、
グリッドに `data-query` で渡す。行のリンクは `/rows/{id}` に向ける。

**Files:**
- Modify: `examples/minimal-host/src/main.rs`、`examples/minimal-host/tests/smoke.rs`、`crates/fu-gantt-web/web/test/host.test.mjs`

**Interfaces:**
- Consumes: Task 2 の属性
- Produces（見本の HTTP）:
  - `GET /?hide_done=1` — グリッドに `data-query="hide_done=1"` を付けたページ。クエリーなしなら属性なし
  - `GET /rows/{id}` — その行の名前を出すだけのページ
  - API の全エンドポイントが `?hide_done=1` を受け取り、進捗 100 の行を除いた計画を返す

- [ ] **Step 1: 失敗する煙テストを書く**

`examples/minimal-host/tests/smoke.rs` の末尾に足す:

```rust
/// The page hands the grid whatever view it was asked for, and nothing when
/// it was asked for none.
#[test]
fn the_page_passes_its_view_on_to_the_grid() {
    let host = Host::start();

    let (_, plain) = host.ask("GET", "/", None);
    assert!(!plain.contains("data-query"), "{plain}");
    assert!(plain.contains(r#"data-row-link="/rows/{id}""#), "{plain}");

    let (_, hiding) = host.ask("GET", "/?hide_done=1", None);
    assert!(hiding.contains(r#"data-query="hide_done=1""#), "{hiding}");
}

/// The same query on a read and on a write gives the same view of the plan.
#[test]
fn a_view_is_kept_on_reads_and_on_what_a_write_answers() {
    let host = Host::start();
    let grid = host.json("GET", &format!("{API}/grid?hide_done=1"), None);
    assert_eq!(names(&grid).len(), 3);
    let first = grid["tasks"][0]["id"].as_str().unwrap().to_owned();

    // Finishing a row under that view takes it out of the answer...
    let done = host.json(
        "POST",
        &format!("{API}/tasks/{first}?hide_done=1"),
        Some(r#"{"field":"progress","value":"100"}"#),
    );
    assert_eq!(names(&done["grid"]), ["実装", "テスト"]);

    // ...and out of the next read, while the plain view still has it.
    let hiding = host.json("GET", &format!("{API}/grid?hide_done=1"), None);
    assert_eq!(names(&hiding), ["実装", "テスト"]);
    let all = host.json("GET", &format!("{API}/grid"), None);
    assert_eq!(names(&all).len(), 3);
}

#[test]
fn a_row_has_a_page_of_its_own() {
    let host = Host::start();

    let (status, page) = host.ask("GET", "/rows/t-2", None);
    assert_eq!(status, 200);
    assert!(page.contains("実装"), "{page}");

    assert_eq!(host.ask("GET", "/rows/nope", None).0, 404);
}
```

Run: `cargo test -p minimal-host`
Expected: 新しい3本が FAIL（もとの5本は通る）

- [ ] **Step 2: 見本のホストに足す**

`examples/minimal-host/src/main.rs`:

`use topcoat::{…}` の `router::{…}` に `query_params` を足す（`apps/fugantt/src/api.rs` と同じ場所から）。

`State::grid` を、条件を受け取る形にする:

```rust
    /// The plan as one view of it sees it, worked out afresh.
    fn grid(&self, view: View) -> GridData {
        let mut rows: Vec<TaskRow> = self
            .rows
            .iter()
            .filter(|row| !(view.hide_done && row.progress >= 100))
            .cloned()
            .collect();
        rows.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));
```

（以下 `domain::build(…)` はそのまま。）

`State::changed` に引数 `view: View` を足し、中の `self.grid()` を `self.grid(view)` にする。

`State` の定義の近くに足す:

```rust
/// Which rows whoever is looking wants to see.
///
/// The grid does not know what this means. It was handed the query by the
/// page and hands it back on every request; reading it is this program's job.
#[derive(Clone, Copy, Default)]
struct View {
    hide_done: bool,
}

#[query_params(error = bad_request("hide_done は 0 か 1 です。"))]
struct ViewQuery {
    hide_done: Option<String>,
}

fn view(cx: &Cx) -> View {
    let hide_done = query_params::<ViewQuery>(cx)
        .ok()
        .and_then(|query| query.hide_done.clone())
        .is_some_and(|value| value == "1");

    View { hide_done }
}
```

各ハンドラーで `let view = view(cx);` を取り、`plan.grid()` → `plan.grid(view)`、
`plan.changed(&id, …)` → `plan.changed(&id, view, …)` にする（`grid`・`insert`・`edit`・`reorder`）。
`plan(cx)` のロックを取る前に `view(cx)` を呼ぶこと。

ページ（`page`）を、条件があるときだけ `data-query` を付ける形にする:

```rust
#[route(GET "/")]
async fn page(cx: &Cx) -> Result<Response> {
    let query = if view(cx).hide_done {
        r#" data-query="hide_done=1""#
    } else {
        ""
    };
```

`format!` の中の `<div id="fugantt-grid" …>` の行を次にする:

```
<div id="fugantt-grid" data-project="demo" data-api="{API}" data-filter-count="#count" data-row-link="/rows/{{id}}"{query}></div>
```

（`format!` の中なので `{id}` は `{{id}}` と書く。）

ヘッダーに切り替えのリンクを足す（`<header>` の中）:

```
<header>minimal-host <a href="/">すべて</a> <a href="/?hide_done=1">終わったものを隠す</a> <span id="count"></span></header>
```

行のページを足す:

```rust
/// Where a row's link leads. A real host has the row's own page here.
#[route(GET "/rows/{task_id}")]
async fn row_page(cx: &Cx) -> Result<Response> {
    let id = task_id(cx)?;
    let plan = plan(cx);

    let Some(task) = plan.rows.iter().find(|row| row.id == id) else {
        return Err(not_found().into());
    };

    // The name is the only thing here somebody typed.
    let name = task
        .name
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let html = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>{name}</title>\
         <p><a href=\"/\">← 計画に戻る</a></p><h1>{name}</h1>"
    );

    Ok(Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(html))?)
}
```

`#[query_params]` の書き方がコンパイルで合わなければ、`apps/fugantt/src/api.rs` の `Sections` と
`wants_settings` の書き方（234〜243行目付近）に合わせる。

Run: `cargo test -p minimal-host`
Expected: 8 passed

- [ ] **Step 3: ブラウザーテストに足す**

`crates/fu-gantt-web/web/test/host.test.mjs` の `check("JavaScript エラーが出ていない", …)` の前に足す:

```js
// The link beside a name leads where the host said, and pressing it is only
// pressing a link.
const link = await page.evaluate(() => {
  const anchor = document.querySelector(".fg-pane-left .fg-row.fg-data .fg-row-link");
  return anchor ? anchor.getAttribute("href") : null;
});
check("行のリンクが出る", link === "/rows/t-1", link ?? "なし");

// The same page under a view: every request carries it.
asked.length = 0;
const queries = [];
page.on("request", (request) => {
  const url = new URL(request.url());
  if (url.pathname.includes("/api/")) queries.push(url.search);
});

await page.goto(`${BASE}/?hide_done=1`, { waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid", { timeout: 10000 }).catch(() => {});

// A write under the view, so there is more than a read to judge by.
await page.click(".fg-pane-left .fg-row.fg-data .fg-name-text");
await page.keyboard.press("F2");
await page.keyboard.down("Meta");
await page.keyboard.press("a");
await page.keyboard.up("Meta");
await page.keyboard.type("詳細設計");
await page.keyboard.press("Enter");
await new Promise((resolve) => setTimeout(resolve, 500));

check("条件の下でも書き込める", (await names())[0] === "詳細設計", (await names()).join(","));
check("読みと書きの両方を見た", queries.length >= 2, String(queries.length));
check(
  "すべてのリクエストに条件が付いている",
  queries.every((search) => search === "?hide_done=1"),
  queries.join(" "),
);

await page.click(".fg-pane-left .fg-row.fg-data .fg-row-link");
await page.waitForFunction(() => location.pathname.startsWith("/rows/"), { timeout: 5000 }).catch(() => {});
check(
  "リンクを押すと行のページに移る",
  await page.evaluate(() => location.pathname === "/rows/t-1"),
  await page.evaluate(() => location.pathname),
);
```

リンクは行に重ねるまで透明（`opacity: 0`）だが、`page.click` は要素の中心に移動してから押すので、
その時点で `:hover` が効く。押せない場合は、先に `page.hover(".fg-pane-left .fg-row.fg-data")` を入れる。

```bash
PORT=18620 HOST=127.0.0.1 cargo run -p minimal-host &
cd crates/fu-gantt-web/web && HOST_URL=http://127.0.0.1:18620 node test/host.test.mjs
```

Expected: 11 passed, 0 failed。終わったら見本のホストを止める

- [ ] **Step 4: 全部通して Commit**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

```bash
git add -A
git commit -m "Show the example under a view, with a page for each row

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: 文書と 1.2.0

**Files:**
- Modify: `docs/gantt-api.md`、`crates/fu-gantt-web/src/lib.rs`（冒頭の例）、`crates/fu-gantt-web/Cargo.toml`、`apps/fugantt/Cargo.toml`、`Cargo.lock`

- [ ] **Step 1: `docs/gantt-api.md` を直す**

「ページに置くもの」の HTML の例を次にする:

```html
<link rel="stylesheet" href="（GRID_CSS を返す URL）">
<div id="fugantt-grid"
     data-project="abc"
     data-api="/gantt/api/projects/abc"
     data-query="status=open"
     data-row-link="/issues/by-id/{id}"
     data-filter-count="#my-counter"></div>
<script src="（GRID_JS を返す URL）" defer></script>
```

属性の表の `data-api` の行の次に2行足す:

```markdown
| `data-query` | | グリッドが送るすべてのリクエストに付けるクエリー（`?` は付けずに書く）。グリッドは中身を解釈しない。無ければ何も付けない |
| `data-row-link` | | 行から外のページに飛ぶリンクの雛形。`{id}` が行の id に置き換わる。`/` で始まるか `http(s)://` で始まるものだけ。無ければリンクを出さない |
```

「ホストが守ること」の箇条の末尾に足す:

```markdown
- **`data-query` を使うなら、読みと書きの両方でそれを解釈する。** グリッドは `grid`・`patch`・`live`
  と、すべての書き込みに同じクエリーを付ける。書き込みの応答（`Mutation`）も、その条件で見た
  計画にすること
- **条件付きのとき、`Patch.total` は「その条件で見えている行数」にする。** グリッドは手元の
  行数と比べ、合わなければ `/grid` を読み直す
- **書き込みで行が条件から外れたら、`grid`（全体）を返す。** `patch` で返すと行が残って見える。
  `note` に理由を入れると、グリッドがそれを1行で知らせる
```

冒頭の依存の例のタグを `v1.2.0` にする（2か所）。

`crates/fu-gantt-web/src/lib.rs` の冒頭の HTML の例の `<div …>` を次にする:

```
//! <div id="fugantt-grid" data-project="abc" data-api="/somewhere/abc"
//!      data-query="status=open" data-row-link="/issues/{id}"></div>
```

- [ ] **Step 2: 版を上げる**

- `crates/fu-gantt-web/Cargo.toml`: `version = "0.1.0"` → `"0.2.0"`
- `apps/fugantt/Cargo.toml`: `version = "1.1.0"` → `"1.2.0"`

Run: `cargo build -p fugantt && cargo run -q -p fugantt -- version`
Expected: `fugantt 1.2.0`

- [ ] **Step 3: 最後に全部通す**

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p fu-calendar --all-features
cargo test -p fu-gantt-core --no-default-features
cargo build --release --locked -p fugantt
(cd crates/fu-gantt-web/web && npm run typecheck && npm run test:unit && npm run build)
git diff --exit-code crates/fu-gantt-web/web/dist
```

Expected: すべて成功。`golden` 8 は記録のまま一致。`minimal-host` 8、`test:unit` 10

グリッドのブラウザーテスト（fugantt 相手）と見本のホストのブラウザーテストを最後にもう一度通す。

Expected: 383 passed / 1 failed（もとから）、11 passed / 0 failed

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "Cut 1.2.0

Nothing changes for whoever runs fugantt. The grid takes a query and a
link for each row from a host that offers them.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

push・PR・マージ・タグは、このタスクの範囲外（人が決める）。
