# ガントのグリッドを組み込む

fugantt のグリッドは、fugantt 以外のアプリにも組み込める。必要なのは、ページに要素を1つ置く
ことと、JSON の API を用意すること。保存・認証・権限は組み込む側が持つ。

動く最小の例が `examples/minimal-host` にある。

## 使うクレート

```toml
fu-gantt-core = { git = "https://github.com/fu-foo/fugantt", tag = "v1.2.0" }
fu-gantt-web  = { git = "https://github.com/fu-foo/fugantt", tag = "v1.2.0" }
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
     data-query="status=open"
     data-row-link="/issues/by-id/{id}"
     data-filter-count="#my-counter"></div>
<script src="（GRID_JS を返す URL）" defer></script>
```

| 属性 | 必須 | 意味 |
|---|---|---|
| `id="fugantt-grid"` | ○ | グリッドが描く先。1ページに1つ |
| `data-project` | ○ | プロジェクトの識別子。折りたたみ状態をブラウザーに覚えさせるキーにも使う |
| `data-api` | | API の基点。無ければ `/api/projects/{data-project}` |
| `data-query` | | グリッドが送るすべてのリクエストに付けるクエリー（`?` は付けずに書く）。グリッドは中身を解釈しない。無ければ何も付けない |
| `data-row-link` | | 行から外のページに飛ぶリンクの雛形。`{id}` が行の id に置き換わる。`/` で始まるか `http(s)://` で始まるものだけ。無ければリンクを出さない |
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
- **`Settings.can_edit` は既定で `false`。** そのままだとグリッドは閲覧専用になる。書かせてよい相手には
  `true` にして渡す
- `today` はホストが決める。「遅れ」は読んでいる人の暦の話なので、fugantt はサーバーのローカル時刻の日付を使っている

`build` のあとで、ホストが必要に応じて埋めるフィールドが3つある。どれも空のままで描ける。

| フィールド | 意味 | 空のとき |
|---|---|---|
| `language` | `"ja"` か `"en"` | 日本語 |
| `column_order` | 列の並び（列のキーの配列） | グリッドの既定の並び |
| `filter_sets` | 名前を付けて残した絞り込み条件 | 一覧が空 |

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

403 だけは扱いが違う。グリッドは本文を読まず、「集計行の日付と進捗は子タスクから決まります。」と
表示する。fugantt が、子を持つ行の日付や進捗への書き込みを 403 で断っているため。権限が無いことを
伝えたいときは、403 ではなく `can_edit: false` で最初から書かせないこと。

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
- **`data-query` を使うなら、読みと書きの両方でそれを解釈する。** グリッドは `grid`・`patch`・`live`
  と、すべての書き込みに同じクエリーを付ける。書き込みの応答（`Mutation`）も、その条件で見た
  計画にすること
- **条件付きのとき、`Patch.total` は「その条件で見えている行数」にする。** グリッドは手元の
  行数と比べ、合わなければ `/grid` を読み直す
- **書き込みで行が条件から外れたら、`grid`（全体）を返す。** `patch` で返すと行が残って見える。
  `note` に理由を入れると、グリッドがそれを1行で知らせる

## 版

`fu-gantt-core` と `fu-gantt-web` は 0.x の間、JSON の形を変えることがある。変えるときは
両方を同じコミットで変える。タグを固定して使うこと。
