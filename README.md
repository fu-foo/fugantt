# fugantt

**予定と実施を並べ、営業日で数えるガントチャート。**
表はキーボードだけで編集できる。Rust（Topcoat + SQLite）製、実行ファイル1本。

[English](README.en.md)

![行を足して打つ、バーを引く、遅れで絞り込む](docs/images/demo.gif)

## できること

- **予定と実施が同じ行にある。** 開始差異・終了差異は保存せず、毎回計算する
- **遅れは入れた予定に対して出る。** 「この日までに何%」を入れた行だけ、届かなければ赤
- **遅れの内訳が出る。** 「12日遅れ」ではなく「作業9日、待ち3日」
- **誰が空いているか分かる。** 月単位で、割当済の日数・空き日数と、その日付
- **取り消せる。** ⌘Z / Ctrl+Z。値も、足した行も、並べ替えも。他の人が先に触ったセルは取り消さない
- **日数の数え方を選べる。** 土日・祝日・休暇・待ちを除く（既定で除く）
- **日本の暦に合っている。** 祝日は計算（振替休日・国民の休日まで）、年度、和暦、土曜は青・日曜は赤
- **キーボードで編集する。** 日本語入力もそのまま通る
- **Excel と JSON に書き出す。** JSON は取り込みもできる
- **既定は日本語。** ブラウザが英語なら英語で出る
- **行数で重くならない。** 見えている行だけを描くので、2000行でも打鍵は15ms

![スケジュール画面](docs/images/schedule.png)

**統計**はプロジェクトごとの数字。タスク数・平均進捗・遅延中の数と、ずれの内訳
（作業の遅れと、理由ごとの待ち）、ステータス別・担当者別の件数。
**タスク変更履歴**は、誰がいつどの値を変えたか。

| | |
| --- | --- |
| 統計 | ![統計](docs/images/stats.png) |
| タスク変更履歴 | ![タスク変更履歴](docs/images/history.png) |

## 入れる

[Releases](https://github.com/fu-foo/fugantt/releases) に4つ。どれも実行ファイル1本で、ほかに要るものは無い。

| | |
| --- | --- |
| `fugantt-macos-arm64` | macOS / Apple Silicon（ネイティブ。Rosetta は要らない） |
| `fugantt-macos-x86_64` | macOS / Intel |
| `fugantt-windows-x86_64` | Windows 64bit（C ランタイムも静的リンク） |
| `fugantt-linux-x86_64` | Linux 64bit（glibc） |

### Windows

**① zip を置くだけ**

1. [Releases](https://github.com/fu-foo/fugantt/releases) から `fugantt-windows-x86_64.zip`
2. 展開して、好きな場所（`C:\fugantt` など）に置く
3. `fugantt.exe` をダブルクリック。画面が開く（Edge のアプリウィンドウ）

- 初回だけ「**WindowsによってPCが保護されました**」と出る。署名を買っていないため。
  「詳細情報」→「実行」で進む
- 黒いコンソールが1つ残る。**それがサーバー本体**なので、閉じると止まる
- データは `%LOCALAPPDATA%\fugantt\fugantt.db`

**② Scoop で入れる**（更新まで任せたいなら）

Scoop は Windows 向けのパッケージ管理。管理者権限もインストーラも使わない。PowerShell で:

```powershell
# Scoop 本体（最初の一度だけ）
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
Invoke-RestMethod -Uri https://get.scoop.sh | Invoke-Expression

# fugantt
scoop bucket add fu-foo https://github.com/fu-foo/scoop-bucket
scoop install fugantt
```

以後は `fugantt` で起動、`scoop update fugantt` で更新、`scoop uninstall fugantt` で削除。

> Scoop は更新のたびにフォルダを入れ替える。**`fugantt.ini` は exe の隣ではなく
> `%LOCALAPPDATA%\fugantt\` に置く**（隣に置くと更新で消える）。データベースは元からそちら。

### macOS / Linux

```sh
brew install fu-foo/tap/fugantt
```

Apple Silicon には arm64 版が入る。

### Docker

```sh
docker run -p 1861:1861 -v fugantt:/data ghcr.io/fu-foo/fugantt
```

イメージは amd64 だけ。Apple Silicon では `--platform linux/amd64` を付ける（エミュレーションで動く）。
ネイティブで動かしたいなら上のバイナリを。Fly.io の設定（`fly.toml`）も入っている。

## 起動する

```sh
fugantt
```

`http://127.0.0.1:1861` で立ち上がり、画面が開く（Windows は Edge のアプリウィンドウ、ほかは既定のブラウザ）。
閉じるか Ctrl+C で止まる。

> **ポートは 1861。** ヘンリー・ガントの生年。

### 最初のアカウント

誰もいないうちは、ログイン画面の横に**「最初のアカウントを作る」**が出る。
名前・ユーザー名・パスワード（既定で8文字以上）を入れて登録すると、その人が**管理者**になる。

この欄は1人目が登録した時点で消える。2人目からは、管理者がユーザーの画面で作って、
初期パスワードを本人に渡す（自己登録もメールも無い）。

管理者のパスワードを忘れたら、サーバーの上で `fugantt --make-admin <ユーザー名>`。

### 最初のプロジェクト

1. 「新しいプロジェクト」に名前を入れて作成する
2. 「最初のタスクを追加」を押してタスク名を打つ。次からは **⌘Enter / Ctrl+Enter** で下に行が増える
3. 予定開始・予定終了を打つ。日付は `5`（当月5日）、`305`（3月5日）のように数字だけでいい
4. 始まったら実施開始を、終わったら実施終了を入れる。チャートの予定の枠の下に実施の線が出る
5. 担当者・ステータス・納期は選ぶか打つ

操作の全体は[使い方](docs/guide.md)に。

### 裏で動かす

```sh
fugantt start     # 裏で起動して、すぐ戻る（画面はいつもどおり開く）
fugantt status    # 動いているか、URL・データ・ログの場所
fugantt log       # ログの最後
fugantt stop      # 止める
```

## 設定する

設定は環境変数か、同じ名前を書いた **`fugantt.ini`**（実行ファイルの隣・カレント・利用者ごとの場所のどれか）。
**環境変数のほうが強い。**

```ini
# fugantt.ini
PORT = 3100
FUGANTT_DB = D:\plans\fugantt.db
```

| 名前 | 既定 | |
| --- | --- | --- |
| `HOST` | `127.0.0.1` | `0.0.0.0` で LAN に公開 |
| `PORT` | `1861` | |
| `FUGANTT_DB` | 下記 | SQLite ファイル。起動時に自動でマイグレーション |
| `FUGANTT_OPEN` | 自動 | 起動時に画面を開くか。`window`（アプリウィンドウ）／`tab`／`0`（開かない） |
| `FUGANTT_ALLOW_HTTP` | — | `1` で平文 HTTP でもログインできる（下記） |
| `FUGANTT_NO_AUTH` | — | 認証なしで動かす（[リファレンス](docs/reference.md#認証なしで動かす)） |

`FUGANTT_OPEN` を書かなければ、127.0.0.1 などで立てたときだけ開く。Windows はアプリウィンドウ、ほかはタブ。

`fugantt --config` でどの値がどこから来ているか、`fugantt --help` で書ける項目が出る。

**DB の場所**は、`FUGANTT_DB` →（カレントに `fugantt.db` があればそれ）→ 利用者ごとの場所、の順。
最後は Windows が `%LOCALAPPDATA%\fugantt`、macOS が `~/Library/Application Support/fugantt`、
Linux が `~/.local/share/fugantt`。起動時に絶対パスを1行出す。

### 社内 LAN に置くとき

`HOST=0.0.0.0` で LAN から届くようになる。ただしそのままでは、LAN の IP からログインできない
（セッションクッキーに `Secure` が付いていて、ブラウザは localhost 以外の `http://` では受け取らない）。

- HTTPS を用意する（Caddy の `tls internal`、Tailscale の証明書など）。こちらを勧める
- 用意できなければ `FUGANTT_ALLOW_HTTP=1`。トークンが平文で流れる。起動時に警告が出る

リバースプロキシを挟むときの注意は[リファレンス](docs/reference.md#守り)に。

## 運用

- **バックアップは全体の設定から。** 「バックアップを作る」で1ファイル落ちてくる。戻すのも同じ画面
  （戻す直前の中身は、データベースの隣に自動で1つ控える）。**アカウントとパスワードもその時点に戻る**
- コマンドでやるなら `VACUUM INTO`。WAL があるのでファイルコピーは不完全になる
  ```sh
  sqlite3 fugantt.db "VACUUM INTO '/backup/fugantt-$(date +%F).db'"
  ```
- **サーバーは1台だけ。** 同じ SQLite を2つのプロセスから書くと壊れる
- **更新は実行ファイルの差し替え。** マイグレーションは自動で走り、戻す手段はない。先にバックアップを

## ドキュメント

| | |
| --- | --- |
| [使い方](docs/guide.md) | 表とチャートの操作、納期・予定・実施、遅れ、空き検索、統計、設定、取り込み |
| [リファレンス](docs/reference.md) | キー操作、設定項目、JSON の形、API、権限と認証、1.0 の約束 |
| [考え方](docs/design.md) | なぜそう動くのか。遅れの測り方、空き検索の数え方など |

## 開発する

```sh
cargo-topcoat dev        # http://127.0.0.1:1861。保存すると作り直す
cargo build --release    # 配布用の実行ファイル1本（静的ファイルも中に入る）
```

> `cargo topcoat dev` ではなく `cargo-topcoat dev`。topcoat-cli 0.5.0 は
> cargo のサブコマンドとして呼ばれると引数を読めない。`cargo run` でも動く。

## 応援する

役に立ったら、開発の支えに。

[![GitHub Sponsors](https://img.shields.io/github/sponsors/fu-foo?style=for-the-badge&logo=github&label=Sponsor)](https://github.com/sponsors/fu-foo)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-Support-ff5e5b?style=for-the-badge&logo=ko-fi)](https://ko-fi.com/fufoo)

## ライセンス

Apache License 2.0

Copyright 2026 Kazunari Fukagawa
