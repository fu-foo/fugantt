# fugantt

**予定と実施を並べ、営業日で数えるガントチャート。**
表はキーボードで編集できる。Rust（Topcoat + SQLite）製、実行ファイル1本。

[English](README.en.md)

![行を足して打つ、バーを引く、遅れで絞り込む](docs/images/demo.gif)

## できること

- **予定と実施を1行で比べられる。** 開始・終了の差異は自動で計算する
- **入れた予定進捗に対して遅れを判定できる。** 「この日までに何%」を入れた行は、届かなければ赤になる（期限超過と納期遅れは、入れなくても赤）
- **遅れの内訳が分かる。** 「12日遅れ」ではなく「作業9日、待ち3日」
- **誰が空いているか調べられる。** 月単位か任意の範囲で、空き日数とその日付が出る
- **変更を取り消せる。** ⌘Z / Ctrl+Z で、値・足した行・並べ替えを戻す（行の削除は戻せない）
- **日数の数え方を決められる。** 除く曜日・祝日・休暇を選ぶ（既定は土日・祝日・休暇）
- **日本の暦で使える。** 祝日（振替休日・国民の休日まで）、年度、和暦。土曜は青、日曜は赤
- **キーボードで編集できる。** 日本語入力もそのまま通る
- **チャートをマウスで直せる。** バーを引けば日付が、端をつまめば期間が、中のつまみで進捗が変わる
- **Excel と JSON に書き出せる。** JSON は取り込みもできる
- **英語でも使える。** 既定は日本語で、ブラウザが英語なら英語で出る
- **行が増えても打鍵は遅くならない。** 見えている行だけを描くので、100行でも1万行でも1打鍵の速さはほぼ変わらない（[測った値](docs/reference.md#10-の約束)）

![スケジュール画面](docs/images/schedule.png)

**統計**はプロジェクトごとの数字。タスク数、平均進捗、遅延中の数。ずれの内訳（作業の遅れと、理由ごとの待ち）。
ステータス別と担当者別の件数。
**タスク変更履歴**は、誰がいつどの値を変えたか。

| | |
| --- | --- |
| 統計 | ![統計](docs/images/stats.png) |
| タスク変更履歴 | ![タスク変更履歴](docs/images/history.png) |

## 入れる

[Releases](https://github.com/fu-foo/fugantt/releases) に4つ。中身は実行ファイル1つで、ほかにインストールするものは無い。

| | |
| --- | --- |
| `fugantt-macos-arm64.tar.gz` | macOS / Apple Silicon（ネイティブ。Rosetta は要らない） |
| `fugantt-macos-x86_64.tar.gz` | macOS / Intel |
| `fugantt-windows-x86_64.zip` | Windows 64bit（C ランタイムも静的リンク） |
| `fugantt-linux-x86_64.tar.gz` | Linux 64bit（glibc） |

### Windows

**① zip を置くだけ**

1. [Releases](https://github.com/fu-foo/fugantt/releases) から `fugantt-windows-x86_64.zip`
2. 展開して、好きな場所（`C:\fugantt` など）に置く
3. `fugantt.exe` をダブルクリック。画面が開く

- 初回だけ「**WindowsによってPCが保護されました**」と出る。署名を買っていないため。
  「詳細情報」→「実行」で進む
- 黒いコンソールが1つ残る。**それがサーバー本体**
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

Mac は brew を勧める。Releases から直接落とした場合、Apple の公証を受けていないので初回は Gatekeeper に止められる。
展開したフォルダで `xattr -d com.apple.quarantine fugantt` を実行すると起動できる。

### Docker

```sh
docker run -p 1861:1861 -v fugantt:/data ghcr.io/fu-foo/fugantt
```

上の `-p 1861:1861` は、ホストの外からも届く形で開く。[最初のアカウント](#最初のアカウント)を作るまでは
`-p 127.0.0.1:1861:1861` で立てる（データはボリュームに残るので、作ったあとで立て直せばいい）。
ほかの PC から開くときは[社内 LAN に置くとき](#社内-lan-に置くとき)も読む。

イメージは amd64 だけ。Apple Silicon では `--platform linux/amd64` を付ける（エミュレーションで動く）。
ネイティブで動かしたいなら上のバイナリを。

## 起動する

```sh
fugantt
```

`http://127.0.0.1:1861` で立ち上がり、画面が開く（Windows は Edge のアプリウィンドウ、ほかは既定のブラウザ）。
起動したターミナル（Windows は黒いコンソール）を閉じるか、Ctrl+C で止まる。ブラウザを閉じても止まらない。

> **ポートは 1861。** ヘンリー・ガントの生年。

### 最初のアカウント

誰もいないうちは、ログイン画面の横に「**最初のアカウントを作る**」が出る。
名前・ユーザー名・パスワード（既定で8文字以上）を入れて登録すると、その人が**管理者**になる。

2人目からは、管理者がユーザーの画面で作って、初期パスワードを本人に渡す（自己登録もメールも無い）。

「最初のアカウントを作る」は、届く人なら誰でも使える。LAN に出す（`HOST=0.0.0.0`）前に、手元で最初のアカウントを作っておく。

管理者が入れなくなったら（パスワードを忘れた、辞めた）、サーバーの上で `fugantt --make-admin <ユーザー名>` を実行して、
ほかのアカウントを管理者にする。サーバーは動かしたままでいい。その人は入り直さなくても、画面を読み込み直せば管理者になっている。
このコマンドはパスワードを戻さない。元の管理者のパスワードは、新しい管理者が入れ直す。
アカウントが管理者の1つしか無いと、管理者にできる相手がいない。アカウントは2つ以上作っておく。

### 最初のプロジェクト

1. 「新しいプロジェクト」に名前を入れて作成する
2. 「最初のタスクを追加」を押してタスク名を打つ。次からは **⌘Enter / Ctrl+Enter** で下に行が増える
3. 予定開始・予定終了を打つ。日付は `5`（当月5日）、`1225`（12月25日）のように数字だけでいい
4. 始まったら実施開始を、終わったら実施終了を入れる。チャートの予定の枠の下に実施の線が出る
5. 担当者・ステータスは選ぶか打つ。**納期**は「この日までに終わればいい」という約束の日で、予定（自分たちの段取り）とは別に入れる。
   [詳しくは使い方](docs/guide.md#納期)

### 裏で動かす

```sh
fugantt start     # 裏で起動して、すぐ戻る（画面はいつもどおり開く）
fugantt status    # 動いているか、版・URL・データ・ログの場所
fugantt log       # ログの最後
fugantt stop      # 止める
fugantt version   # 版
```

## 設定する

設定は環境変数か、同じ名前を書いた **`fugantt.ini`**（実行ファイルの隣・カレントディレクトリ・利用者ごとの場所のどれか）。
利用者ごとの場所は、下の DB の場所と同じ。**両方に書いたら環境変数が優先する。**

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
| `FUGANTT_ALLOW_HTTP` | — | `1` で平文 HTTP でもログインできる（[下記](#社内-lan-に置くとき)） |
| `FUGANTT_NO_AUTH` | — | 認証なしで動かす（[リファレンス](docs/reference.md#認証なしで動かす)） |

`FUGANTT_OPEN` を書かなければ、127.0.0.1 などで立てたときだけ開く。

`fugantt --config` でどの値がどこから来ているか、`fugantt --help` で書ける項目が出る。

**DB の場所**は、`FUGANTT_DB` →（カレントディレクトリに `fugantt.db` があればそれ）→ 利用者ごとの場所、の順。
最後は Windows が `%LOCALAPPDATA%\fugantt`、macOS が `~/Library/Application Support/fugantt`、
Linux が `~/.local/share/fugantt`。起動時に絶対パスを1行出す。

### 社内 LAN に置くとき

`HOST=0.0.0.0` で LAN から届くようになる。ただしそのままでは、LAN の IP からログインできない
（セッションクッキーに `Secure` が付いていて、ブラウザは localhost 以外の `http://` では受け取らない）。

- HTTPS を用意する（Caddy の `tls internal`、Tailscale の証明書など）。こちらを勧める
- 用意できなければ `FUGANTT_ALLOW_HTTP=1`。セッションのトークンが平文で流れる。起動時に警告が出る

リバースプロキシを挟むときの注意は[リファレンス](docs/reference.md#守り)に。

### インターネットに出すとき

想定は、手元の PC と社内 LAN。認証はユーザー名とパスワードだけで、2要素認証は無い。

外から使いたいなら、Tailscale で閉じたまま使うか、前に認証を置く（Cloudflare Access や oauth2-proxy で SSO を挟む）。
`FUGANTT_NO_AUTH` で fugantt 側のログインを外すと、誰が変えたかが残らなくなるので、両方で入る形にしておく。

前に認証を置くと、API トークンでの呼び出しも止まる。その経路は前段で通す（Cloudflare Access ならサービストークン、
oauth2-proxy なら `skip_auth_routes`）。前段は `Host` をブラウザが送ったまま渡す（[リファレンス](docs/reference.md#守り)）。

## 運用

- **バックアップは全体の設定から。** 「バックアップを作る」で1ファイル落ちてくる。戻すのも同じ画面
  （戻す直前の中身は、データベースの隣に自動で1つ控える）。**アカウントとパスワードもその時点に戻る**
- コマンドでやるなら `VACUUM INTO`。最近の変更は `-wal` ファイルにあるので、`fugantt.db` だけを写すと抜けることがある
  ```sh
  sqlite3 fugantt.db "VACUUM INTO '/backup/fugantt-$(date +%F).db'"
  ```
- **サーバーは1台だけ。** ほかの人はブラウザで入る。同じ DB を2つのサーバーで開くと、
  片方での変更がもう片方の画面に届かない（変更の知らせはサーバーの中だけで回している）
- **共有フォルダに DB を置いて、各自の PC から開かない。** `FUGANTT_DB=\\server\share\fugantt.db` のような使い方は、
  上の問題に加えて、ネットワーク越しでは SQLite のロックが当てにならず、ファイルが壊れることがある
- **更新は実行ファイルの差し替え。** マイグレーションは自動で走り、古い版に戻す手段はない。先にバックアップを。
  `fugantt.db` の探し方は変わらないので、すでに置いて使っているならそのまま。
  動いている版は、左のメニューのいちばん下か `fugantt status` に出る
- **0.3 以前から上げるときは、ポートに注意。** 既定は 3000 から 1861 に変わった。ブックマークや
  `docker run -p 3000:3000` を直すか、`PORT = 3000` を書く

## ドキュメント

| | |
| --- | --- |
| [使い方](docs/guide.md) | 表とチャートの操作、納期・予定・実施、遅れ、空き検索、統計、設定、取り込み |
| [リファレンス](docs/reference.md) | キー操作、設定項目、JSON の形、API、権限と認証、1.0 の約束 |
| [考え方](docs/design.md) | なぜそう動くのか。遅れの測り方、空き検索の数え方など |

## 開発する

Rust（最新の stable）と topcoat-cli が要る。

```sh
cargo install topcoat-cli --version 0.5.0
```

```sh
cargo-topcoat dev        # http://127.0.0.1:1861。保存すると作り直す
cargo build --release    # 配布用の実行ファイル1本（静的ファイルも中に入る）
```

> `cargo topcoat dev` ではなく `cargo-topcoat dev`。topcoat-cli 0.5.0 は
> cargo のサブコマンドとして呼ばれると引数を読めない。`cargo run` でも動く。

## 応援する

役に立ったら、開発を支えてもらえると助かる。

[![GitHub Sponsors](https://img.shields.io/github/sponsors/fu-foo?style=for-the-badge&logo=github&label=Sponsor)](https://github.com/sponsors/fu-foo)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-Support-ff5e5b?style=for-the-badge&logo=ko-fi)](https://ko-fi.com/fufoo)

## ライセンス

Apache License 2.0

Copyright 2026 Kazunari Fukagawa
