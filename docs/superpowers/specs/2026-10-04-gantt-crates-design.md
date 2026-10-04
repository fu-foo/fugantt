# ガントをクレートに切り出す — 設計

2026-10-04。fugantt v1.0.1 から、ガントの計算と画面を他のアプリに組み込める形で切り出す。
fugantt は **v1.1.0** として出す。利用者から見える挙動は変えない。

## 目的

Topcoat で Redmine 風のアプリ（課題管理＋Wiki）を作り、そこにガントを組み込みたい。
そのアプリでは **課題とガントのタスクは同じもの** にする。課題に開始日・期日・進捗があり、
ガントは課題を並べて見せて、その場で書き換える画面になる。

だから組み込むのは fugantt 全体ではない。**計算と画面だけ** を持ち出し、保存・認証・権限は
組み込む側が自分で持つ。

グリッドの画面（`grid.ts`）は、このアプリに限らず今後も使い回したい。fugantt 単体も製品として
出し続ける。fugantt は切り出したクレートの最初の利用者になる。

## 決めたこと

- 切り出すのは3クレート: 祝日の計算、ガントの計算、ガントの画面
- 保存（DB・マイグレーション）、認証、権限、プロジェクト管理、API のハンドラーは切り出さない
- リポジトリは fugantt のまま1つ。Cargo ワークスペースにする
- fugantt は v1.1.0。DB・設定・データの置き場所・API・画面は変えない
- 切り出すクレートは fugantt と別に版を持ち、`0.1.0` から始める。`publish = false`
- 他のリポジトリからは git 依存で使う
- 機能の出し分け（休暇や絞り込みを隠すなど）は入れない。必要になったホストが出てから足す

## やらないこと

元の「v2.0.0 ワークスペース分割計画」にあった次の作業は、この設計では不要になる。

- 層3・4（SQLite を開く、認証、セッション、ガード）の共通化
- sqlx の `migrate!()` をやめて自前の実行器にすること、0001〜0034 の畳み直し
- アプリ名のパラメーター化（`FUGANTT_*`、データの置き場所）
- `app_settings.rs`・`i18n`・`api.rs` の分割

新アプリの側でこれらが fugantt と重複してきたら、そのとき実物を2つ見比べて切り出す。

API のハンドラーを保存先のトレイトで抽象化して共有する案も見送る。利用者が fugantt しか
いない段階でトレイトの形を決めると、課題側の都合（コメント、履歴、権限）が入ったときに
作り直しになる。

## 現状（調べた事実）

- `src/domain.rs`（2,287行）は `crate::` への参照がない。DB との接点は `TaskRow` と
  `FilterSet` の `sqlx::FromRow` derive の2か所だけ
- 入口は `domain::build(project_id, revision, today, Vec<TaskRow>, Settings) -> GridData`
- `src/sortkey.rs`（159行）も `crate::` への参照がない
- `src/holidays.rs`（397行）は、`japanese(year)` が jiff だけに依存する計算、
  `keep_filled` が DB に書く処理
- 和暦と年度の計算は Rust 側にない。`domain.rs` は `Era { from, name }` と
  `fiscal_year_start` を設定として画面に渡すだけで、表示の計算は `grid.ts` がしている
- 営業日の数え方（`Calendar`）は休暇と待ちを含むので、祝日の計算ではなくガントの計算に属する
- 画面は `web/src/grid.ts`（6,696行）と `grid.css`（1,841行）。esbuild で `web/dist/grid.js`
  1ファイルに固める。実行時の依存はない
- ホストのページに要るのは `<div id="fugantt-grid" data-project="…">` と
  `<script src="…/grid.js" defer>` だけ。あとは `grid.ts` が API を叩いて描く
- 言語は `grid` の JSON にある `language` で決まる
- `grid.ts` が叩く URL は `/api/projects/{id}/` 配下に固定:
  `grid`・`tasks`・`tasks/{id}`・`tasks/{id}/move`・`tasks/{id}/place`・`tasks/{id}/patch`・
  `live`・`filters`・`filters/remove`・`leaves`
- `grid.ts` はグリッドの外の要素を1つだけ直接探す: `#fugantt-filter-count`
- `src/static_files.rs` が `include_str!("../web/dist/grid.js")` などで `web/` を埋め込む
- Topcoat の `discover()` は `inventory` で集める。ページやルートは今回 `apps/fugantt` から
  動かさないので、ライブラリクレートからの登録の問題は起きない
- テスト: Rust の単体テスト 115本、`tests/cli.rs`、`web/test/`

## 構成

```
fugantt/
├─ Cargo.toml              [workspace]
├─ crates/
│  ├─ fu-calendar/         日本の祝日の計算
│  ├─ fu-gantt-core/       ガントの計算、並び順のキー、API の JSON 型
│  └─ fu-gantt-web/        web/（grid.ts, CSS, テスト）と dist の埋め込み
└─ apps/
   └─ fugantt/             それ以外すべて（DB, 認証, ページ, api.rs, migrations/）
```

依存は下向きのみ。`apps/fugantt` → 3クレート。3クレートは互いに依存しない
（`domain.rs` は祝日の計算を呼ばず、祝日は設定として受け取るだけ）。

### fu-calendar

`holidays::japanese(year)` と、その下請け（第n月曜、春分・秋分、振替休日、国民の休日）。

- 依存なし。日付は自前の `Day`（年・月・日の数値、`YYYY-MM-DD` で表示できる）で返す。
  曜日と翌日の計算も自前で持つ
- `jiff` フィーチャーで `jiff::civil::Date` へ、`chrono` フィーチャーで `chrono::NaiveDate` へ
  変換できる。FuAshiAto は chrono、fugantt は jiff なので両方要る
- `keep_filled` は DB に書くので `apps/fugantt` に残す

中身は祝日だけで、和暦と年度は入らない（Rust 側に計算がないため）。名前を広めにしてあるのは、
新アプリで和暦や年度の計算が Rust 側に要るようになったときの置き場にするため。

### fu-gantt-core

`domain.rs` と `sortkey.rs` をそのまま移す。加えて、API の入出力の型と、セルに打たれた文字の
読み取りを置く。

- 依存: `jiff`、`serde`
- `sqlx` フィーチャーを付けたときだけ `TaskRow` と `FilterSet` に `FromRow` を付ける。
  fugantt はこのフィーチャーを使う。sqlx を使わないホストは自分で `TaskRow` を組み立てる
- Topcoat に依存しない。axum など他のフレームワークからも使える

API の型は、いま `api.rs` の中で非公開になっている入力の型のうち、`grid.ts` が送るものを
公開の型として移す: `CellEdit`、`InsertTask`、`MoveRequest`、`PlaceRequest`、
`SaveFilterSet`、`LeaveList`／`LeaveEntry`。出力は `GridData` と、`patch`・`live` の応答。
設定画面のフォーム（`StatusForm` など）は `grid.ts` が送らないので移さない。

セルの文字の読み取りも移す。グリッドは日付を `8/5` や `0805` のような打ったままの文字で送り、
読むのはサーバーの仕事になっている。いまは `api.rs` の末尾（`flexible_date`・`parse_date`・
`parse_waits`・`parse_targets`・`parse_colour`）にあり、エラー文の翻訳と「今日」の取得が
混ざっている。これを「今日」を引数で受け取り、エラーを列挙型で返す純粋な関数にして移す。
`api.rs` には、同じ名前・同じエラー文の薄い包みを残す。これが無いと、ホストはグリッドが送る
文字を正しく受け取れない。

### fu-gantt-web

`web/` をディレクトリごと移す。Rust 側は埋め込みだけを持つ。

- Rust の依存なし
- 公開するもの: `GRID_JS`、`GRID_CSS` の定数と、内容から作るハッシュ（キャッシュ破棄用）
- `theme.css`・`favicon.svg`・`page.js` は fugantt のページのものなので `apps/fugantt` に移す
- `grid.css` は自分の既定値（明るい配色）だけで描ける。`theme.css` は `.fg-grid` の `--fg-*`
  変数をページの色に結び直しているだけなので、これがホストの配色の入口になる。
  fugantt の `theme.css` をその見本として文書に書く
- 配信するルートは持たない。ホストが自分のやり方で返す。fugantt では `static_files.rs` が
  これまでどおり `/static/{hash}/{name}` で返す
- `web/dist/` はこれまでどおりリポジトリに入れる。git 依存で使う側に Node を要求しないため

## ホストとの約束

いま暗黙になっている約束を3点だけ明示する。

### 1. API の基点を渡せる

`data-api` 属性で基点を受け取る。無ければ現行の値を使う。

```html
<div id="fugantt-grid" data-project="abc" data-api="/gantt/api/projects/abc"></div>
```

`data-api` が無いときの基点は `/api/projects/{data-project}`。fugantt のページは変更なしで動く。
`grid.ts` の中の URL の組み立ては1か所の関数に寄せる。

### 2. グリッドの外の id に頼らない

`#fugantt-filter-count` を直接探すのをやめ、`data-filter-count` 属性で要素のセレクターを
受け取る。無ければ現行の id を使う。

### 3. API の形を型と文書で固定する

- 入出力は `fu-gantt-core` の型が正
- `docs/gantt-api.md` に、エンドポイントごとのメソッド・入力・出力・失敗時の応答を書く
- ホストが守ること（`revision` を変更のたびに進める、`live` で他の人の変更を知らせる、など）も
  同じ文書に書く

## 順番

各段で挙動を変えない。段ごとにコミットを分け、CI が通ることを条件にする。

### 1. 挙動を固定するテストを先に作る

- データ入りの DB を固定データとして `apps/fugantt/tests/fixtures/` に置く
  （この段ではまだ `tests/fixtures/`）
- 固定 DB から次を取って記録し、毎回突き合わせる
  - `GET /api/projects/{id}/grid` の JSON
  - JSON 書き出し（全体・タスクのみ）
  - `/api/summary`
  - 主要ページの HTML
- 「今日」は `Zoned::now()` から来ていて、遅れの判定も祝日の自動投入もそれに従う。
  記録と突き合わせるため、デバッグビルドに限り `FUGANTT_TODAY` で今日を固定できるようにする。
  リリースビルドではこの分岐はコンパイルされない
- 固定 DB は作り物のデータで作る。リポジトリは公開なので、業務の実データは入れない。
  作る手順（スクリプトと SQL）も一緒に置き、作り直せるようにする

### 2. 器だけワークスペースにする

- `src/`・`migrations/`・`tests/`・`build.rs`・`Topcoat.toml` を `apps/fugantt/` に移す
- `include_str!` の相対パス、`ci.yml`・`release.yml`・`Dockerfile` のパスを直す
- ロジックには触らない

### 3. fu-calendar を切り出す

- `japanese(year)` とその下請け、単体テストを移す
- `keep_filled` は `apps/fugantt` に残し、`fu-calendar` を呼ぶ

### 4. fu-gantt-core を切り出す

- `domain.rs`・`sortkey.rs` と単体テストを移す
- `FromRow` を `sqlx` フィーチャーの下に入れる
- `api.rs` の入出力の型のうち `grid.ts` とやり取りするものを公開の型として移す
- セルの文字の読み取りを純粋な関数にして移す

### 5. fu-gantt-web を切り出す

- `web/` を移し、`static_files.rs` の `include_str!` を `fu-gantt-web` の定数に置き換える
- `data-api` と `data-filter-count` を入れる。既定値は現行どおり
- `examples/` に最小のホストを1つ置く。fugantt とは別の基点（例: `/g/api/…`）で、メモリー上の
  タスクを相手にグリッドが動くもの。`fu-gantt-core` と `fu-gantt-web` だけに依存する

### 6. 契約を文書にして v1.1.0 を出す

- `docs/gantt-api.md` を書く
- README の開発手順を新しい配置に合わせる。利用者向けの変更点は「なし」
  （CHANGELOG のファイルは無く、リリースノートは GitHub が自動で作る）

## 確かめ方

- 段1の突き合わせテストが、段2〜5のあとも一致する
- 既存の Rust 単体テスト 115本と `tests/cli.rs` が通る（移した先で数が減っていないこと）
- `web/test` のブラウザーテストが通る
- 段5のあと、`examples/` のホストでグリッドが描け、行の追加・移動・日付の変更ができる
- 成果物（リリースの実行ファイル、Docker イメージ）がこれまでどおりビルドできる

## 失敗したときの扱い

- 突き合わせテストが一致しなくなったら、その段のコミットを直すか取り消す。差分を
  「許容」して記録を更新しない
- `data-api` を入れたことで fugantt の画面が変わったら、既定値の組み立てが現行と違っている。
  属性なしの経路を現行と同じ文字列にする

## 新アプリ側の作業（この設計の範囲外）

文脈として書いておく。

- 課題の表は `TaskRow` の項目（親子、並び順、予定の開始と終了、納期、実績、進捗、担当、状態、
  待ち、予定進捗、色）を含む上位集合にする
- 課題の表から `TaskRow` を作って `build()` に渡し、`docs/gantt-api.md` の API を実装する
- fugantt の `api.rs` と `project.rs` が実装の見本になる
