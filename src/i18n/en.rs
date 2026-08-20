//! Japanese → English.
//!
//! The key is the original wording. A `match` rather than a map, not because it
//! is faster but because adding a translation then needs no decision about
//! where to put it.
//!
//! Anything missing here comes out in Japanese. Adding a translation can wait
//! until someone sees the Japanese on a screen.
//!
//! Words the users chose — status names, their own fields, assignees, project
//! names — are never in here. Those are data, not the wording of the app.

pub fn of(ja: &str) -> Option<&'static str> {
    Some(match ja {
        // --- actions, everywhere ------------------------------------------
        "保存" => "Save",
        "追加" => "Add",
        "追加・更新" => "Add or update",
        "削除" => "Delete",
        "外す" => "Remove",
        "変更" => "Change",
        "取り込む" => "Import",
        "戻す" => "Undo",
        "キャンセル" => "Cancel",
        "閉じる" => "Close",
        "解除" => "Clear",
        "編集" => "Edit",
        "登録" => "Register",
        "ログアウト" => "Sign out",
        "メニュー" => "Menu",

        // --- navigation ---------------------------------------------------
        "このプロジェクト" => "This project",
        "スケジュール" => "Schedule",
        "統計" => "Statistics",
        "タスク変更履歴" => "Task history",
        "設定" => "Settings",
        "データの入出力" => "Import and export",
        "Excel で書き出す" => "Export to Excel",
        "JSON で書き出す" => "Export as JSON",
        "JSON で書き出す（タスク＋設定）" => "Export as JSON (tasks + settings)",
        "JSON で書き出す（タスク）" => "Export as JSON (tasks only)",
        "設定・名簿・暦を入れずに、タスクだけを書き出します" => {
            "Writes the tasks on their own, without the settings, the lists or the calendar"
        }
        "settings は 0 か 1 です。" => "settings takes 0 or 1.",
        "他の人が先に変更しています。取り消しませんでした。" => {
            "Somebody else changed this first, so nothing was undone."
        }
        "名前を入れてください。" => "Give it a name.",
        "取り消せる操作がありません。" => "Nothing to undo.",

        // --- backups ------------------------------------------------------
        "バックアップ" => "Backups",
        "バックアップを作る" => "Make a backup",
        "バックアップから戻す" => "Restore from a backup",
        "このファイルの内容に戻す" => "Restore this file",
        "いま使っているファイル" => "In use",
        "現在のデータを1つのファイルに出力します。" => {
            "Writes the current data to a single file."
        }
        "現在のデータはすべて、選んだファイルの内容に置き換わります。復元前のデータは自動で控えます。" => {
            "All current data is replaced by what is in the file you choose. What was there before is kept automatically."
        }
        "アカウントとパスワードも復元されます。再ログインが必要になる場合があります。" => {
            "Accounts and passwords are restored as well. You may have to sign in again."
        }
        "いまの中身は、選んだファイルの中身に置き換わります。よろしいですか？" => {
            "Everything here will be replaced by the contents of that file. Continue?"
        }
        "戻しました。直前の中身はここに残してあります：" => {
            "Restored. What was there a moment before is here:"
        }
        "SQLite のファイルではありません。" => "That is not a SQLite file.",
        "fugantt のバックアップではありません。" => "That is not a fugantt backup.",
        "ファイルを開けませんでした。" => "The file could not be opened.",
        "ファイルが選ばれていません。" => "No file was chosen.",
        "やり直せる操作がありません。" => "Nothing to redo.",
        "行の追加・削除・並べ替えは取り消せません。もう一度押すと、その前の変更を取り消します。" => {
            "Adding, deleting and reordering rows cannot be undone. Press again to undo the change before it."
        }
        "JSON を取り込む（全置換）" => "Import JSON (replaces everything)",
        "自分" => "You",
        "プロジェクト一覧" => "Projects",
        "自分の設定" => "Your settings",
        "管理" => "Administration",
        "ユーザー" => "Users",
        "全体の設定" => "Installation settings",
        "メモ" => "Notes",
        "プロジェクトメモ" => "Project notes",

        // --- signing in ---------------------------------------------------
        "ログイン" => "Sign in",
        "ユーザー名" => "Username",
        "パスワード" => "Password",
        "名前" => "Name",
        "はじめての人" => "First time here",
        "パスワードは" => "Password: ",

        // --- page headings ------------------------------------------------
        "まだ変更はありません。" => "Nothing has changed yet.",
        "← 新しい" => "← Newer",
        "古い →" => "Older →",

        // --- statistics ---------------------------------------------------
        "タスク" => "Tasks",
        "平均進捗" => "Average progress",
        "遅延中" => "Late",
        "遅れていない" => "On time",

        // --- capacity -----------------------------------------------------
        "空き検索" => "Availability",
        "全体の空き検索" => "Availability everywhere",
        "件のプロジェクトを合わせて" => " projects counted together",
        "自分が開けるプロジェクトをすべて合わせて数えます。休みの日は会社の暦（土日と全体の設定の祝日）と、その人の休暇です。プロジェクトごとの暦の違いはここでは使いません。" => {
            "Counts every project you can open, together. Days off are the shared calendar — weekends and the holidays in the global settings — plus the person's own leave. A single project's own calendar is not used here."
        }
        "稼働可能日数" => "Available days",
        "経過済" => "Elapsed",
        "割当済" => "Committed",
        "空き日数" => "Free days",
        "重複" => "Overlapping",
        "開始月" => "From",
        "終了月" => "To",
        "この期間で見る" => "Show this range",
        "今日から先を数えます。同じ日に複数のタスクがあっても1日と数え、その重なりは「重複」に出ます。終わったタスクと集計行は数えません。休暇は稼働可能日数から引きます。" => {
            "Counted from today. Two tasks on one day is one day; how many deep is in Overlapping. Finished tasks and summary rows are not counted, and leave comes off the available days."
        }
        "予定進捗に届いていません" => "Not up to the checkpoint it promised",
        "予定終了を過ぎて、実施終了が入っていません" => {
            "Past its planned end, with no actual end"
        }
        "色を消す" => "Clear the colour",
        "背景" => "Background",
        "文字" => "Text",
        "色は #rrggbb の形式で指定してください。" => "A colour is #rrggbb.",
        "作業の遅れ" => "Work behind",
        "ずれの内訳" => "Where the slippage went",
        "ずれ" => "Slippage",
        "待ち" => "Waiting",
        "進捗は0〜100で入力してください。" => "Progress is a number from 0 to 100.",
        "予定進捗は「8/20 30%」のように日付と％で入力してください。" => {
            "Write 予定進捗 as a date and a percentage, like \"8/20 30%\"."
        }
        "ステータス" => "Status",
        "担当者" => "Assignee",
        "（未割当）" => "(unassigned)",
        "（理由なし）" => "(no reason given)",

        // --- settings -----------------------------------------------------
        "表示" => "Display",
        "列" => "Columns",
        "チャートマウスオーバー時の表示" => "What a bar shows on hover",
        "バーにポインタを合わせたときに、日付のほかに出す項目です。表から外した列も選べます。表示することでチャート上で確認することができます。" => {
            "What a bar shows besides its dates when you point at it. Columns you have taken off the table can be named here, and checked on the chart instead."
        }
        "（非表示）" => "(hidden)",
        "日付だけです。" => "Dates only.",
        "独自の項目" => "Your own fields",
        "祝日・休業日" => "Holidays and closures",
        "色" => "Colours",
        "メンバー" => "Members",
        "権限" => "Role",
        "オーナー" => "Owner",
        "編集者" => "Editor",
        "閲覧者" => "Viewer",
        "無効" => "No access",
        "言語" => "Language",
        "外観" => "Appearance",
        "テーマ" => "Theme",
        "自動（OSに合わせる）" => "Automatic (follow the system)",
        "明るい" => "Light",
        "暗い" => "Dark",
        "自分用の CSS" => "Your own CSS",
        "自分の画面にだけ効きます。ほかの人には見えません。" => {
            "Applies to your screen and nobody else's."
        }
        "最後に読み込まれるため、ここでの指定が優先されます。2万文字まで。@import は使えません。" => {
            "Loaded last, so what you write here takes priority. Up to 20,000 characters. @import is not available."
        }
        "日本語" => "Japanese",
        "英語" => "English",
        "自動（ブラウザに合わせる）" => "Automatic (follow the browser)",

        // --- plan and actual ----------------------------------------------
        "予定" => "Planned",
        "実施" => "Actual",
        "完了分" => "Done",
        "集計行" => "Summary row",
        "遅延" => "Late",
        "土曜" => "Saturday",
        "日曜" => "Sunday",
        "祝日" => "Holiday",
        "今日" => "Today",
        "休暇" => "Leave",
        "進捗" => "Progress",
        "実進捗" => "Progress",
        "予定進捗" => "Planned progress",
        "日数" => "Days",
        "予定開始" => "Planned start",
        "予定終了" => "Planned end",
        "実施開始" => "Actual start",
        "実施終了" => "Actual end",
        "開始差異" => "Start variance",
        "終了差異" => "End variance",
        "コメント" => "Note",
        "担当者の休暇/出社" => "Leave and working days",

        // --- banners, confirmations, help text -----------------------------
        "認証なしで動いています。この URL に届く人は全員が全プロジェクトを読み書きできます。" => {
            "Running without sign-in. Anyone who can reach this URL can read and edit every project."
        }
        "取り込むと、いまのタスクはすべて置き換わります。よろしいですか？" => {
            "Importing replaces every task in this project. Continue?"
        }
        "引き継ぎや決めごとなど。改行はそのまま出ます" => {
            "Handover notes, decisions, anything. Line breaks are kept."
        }
        "まだ何も書かれていません。" => "Nothing written yet.",

        // --- signing in ---------------------------------------------------
        "最初のアカウントを作る" => "Create the first account",
        "最初に登録した人が管理者になり、以降のユーザーはその人が作ります。" => {
            "Whoever registers first becomes the administrator, and makes every account after that."
        }
        "画面にはこの名前が出ます。担当者としても選べます。" => {
            "This is the name shown on screen, and the one you can assign work to."
        }
        "山田 太郎" => "Alex Doe",

        // --- the project list ---------------------------------------------
        "新しいプロジェクト" => "New project",
        "作成" => "Create",
        "まだプロジェクトがありません。" => "No projects yet.",

        // --- statistics -----------------------------------------------------
        "差異は待ちを除いて数えているので、作業の遅れと待ちを足したものが実際のずれです。" => {
            "Variance is counted with waiting excluded, so the real slippage is the work behind plus the waiting."
        }
        "平均" => "average",

        // --- settings and administration -----------------------------------
        "アプリの名前（左上に出ます）" => "The name of this app (shown top left)",
        "既定の言語です。自分の設定が優先されます。「自動」はブラウザの言語に合わせます。" => {
            "The default language. A person's own setting takes priority. \"Automatic\" follows their browser."
        }
        "1行に「開始日 名称」。新しい元号が決まったら、ここに設定します。" => {
            "One era per line: start date, then name. Add a line here when a new era is announced."
        }
        "新しく設定するときにだけ適用されます。" => {
            "Applies when a password is set."
        }
        "最低文字数" => "Minimum length",
        "何もチェックしなければ指定なし。日本語は記号に数えます" => {
            "Tick nothing to require none. Non-Latin characters count as symbols"
        }
        "使わせない語" => "Words to refuse",
        "1行に1語。これを含むパスワードは使えません（大文字小文字は区別しません）。空欄なら検査しません。" => {
            "One word per line. A password containing any of them is refused, whatever the case. Leave it empty to skip this check."
        }
        "全プロジェクト共通の暦です。プロジェクトごとの違いは各プロジェクトの設定で調整します。" => {
            "The calendar every project shares. Differences per project are set in that project's own settings."
        }
        "日本の祝日をまとめて入れる" => "Add a year of Japanese public holidays",
        "振替休日と国民の休日も計算します。同じ日付が既にあれば残します。" => {
            "Substitute holidays and citizens' holidays are worked out too. A date already on the list is left as it is."
        }
        "創立記念日" => "Founders' Day",
        "担当者の色" => "Assignee colours",
        "担当者の色は全プロジェクト共通です。アカウントの無い名前も登録できます。" => {
            "Assignee colours are shared by every project. Names without an account can be listed here too."
        }
        "協力会社 A" => "Contractor A",
        "背景色" => "Background",
        "まだ登録がありません。プロジェクトで使われている名前は、そのまま色なしで出ます。" => {
            "Nothing here yet. Names already used in a project still show, without a colour."
        }
        "更新" => "Update",
        " タスク" => " tasks",
        " 行を取り込みました。" => " rows imported.",
        " 行は読めなかったため飛ばしています。" => {
            " rows could not be read and were skipped."
        }
        "日数の数え方は編集者が設定します。" => {
            "How days are counted is set by an editor."
        }
        "進捗の入れ方" => "How progress is set",
        "手入力" => "Typed in",
        "進捗を設定していないステータスは手入力のままです。進捗を 100% にすると、実施終了が空欄なら今日の日付が入ります。" => {
            "A status with no progress of its own leaves the number as typed. Setting progress to 100% fills in today as the actual end, if it is empty."
        }
        "年度の開始月" => "Business year starts in",
        "月" => "月",
        "固定する列" => "Frozen columns",
        "1日の幅" => "Width of a day",
        "四半期の帯を出す" => "Show the quarter band",
        "年を和暦で表示する" => "Show years as Japanese eras",
        "表示・幅（px、空欄で自動）・並び順。タスク名は先頭で固定です。" => {
            "Shown or hidden, width in pixels (empty means automatic), and order. The task name stays first."
        }
        "自動" => "Automatic",
        "進捗を空欄にすると、そのステータスでは手入力のままです。" => {
            "Leave the progress empty and that status keeps whatever was typed."
        }
        "レビュー中" => "In review",
        "進捗（任意）" => "Progress (optional)",
        "項目名" => "Field name",
        "製品" => "Product",
        "種類" => "Kind",
        "選択肢を追加" => "Add a choice",
        "メンバーとタスクに入っている名前が並びます。ここに名前を足せば、アカウントの無い人も選べます。色は全員で共通なので「全体の設定」で決めます。" => {
            "The names of members and of anyone written on a task. Add a name here and people without an account can be assigned work too. Colours are shared by everyone and live in the installation settings."
        }
        "日数の計算は変わりません。" => "The day count is unaffected.",
        "日付" => "Date",
        "名称" => "Name",
        "現場の休業日" => "Site closure",
        "このプロジェクトに追加" => "Add to this project",
        "全体" => "Shared",
        "このプロジェクトでは働く" => "Working here",
        "平均 " => "average ",
        "ベース権限は、プロジェクトに個別の指定が無いときの既定です。「無効」の場合は招待されたプロジェクトのみ表示します。" => {
            "The base role applies to any project that does not name the person; \"no access\" shows only the projects they were added to."
        }
        "ベース権限" => "Base role",
        "新しいパスワード（変えるときだけ）" => "New password (only to change it)",
        "そのまま" => "Unchanged",
        "画面に出る名前です。変えると、担当者に入っている自分の名前も一緒に変わります。" => {
            "The name shown on screen. Changing it also changes your name wherever it is written on a task."
        }

        // --- API トークン ---------------------------------------------------
        "API トークン" => "API tokens",
        "ブラウザ以外からこのプロジェクトだけを読み書きするための API トークンです。" => {
            "An API token that lets something other than a browser read and write this project alone."
        }
        "いま作ったトークンです。この画面を離れると二度と出ません。" => {
            "The token you just made. It is not shown again once you leave this page."
        }
        "用途" => "What for",
        "週次の見直し" => "Weekly review",
        "読むだけ" => "Read only",
        "読み書き" => "Read and write",
        "発行" => "Issue",
        "トークンはオーナーが発行します。" => "Tokens are issued by the owner.",
        "まだありません。" => "None yet.",
        "（名前なし）" => "(unnamed)",
        "最終利用" => "last used",
        "未使用" => "never used",
        "失効" => "Revoke",
        "使い方" => "How to use it",
        "取り込めませんでした。" => "Could not import it. ",

        // --- 既定のステータス -----------------------------------------------
        "既定のステータス" => "Default statuses",
        "新しいプロジェクトの初期値です。既存のプロジェクトには反映されません。" => {
            "The starting point for new projects. Projects that already exist are not affected."
        }
        "手入力のまま" => "left as typed",

        // --- 独自の項目 ------------------------------------------------------
        "名前を保存" => "Save name",
        "種類を保存" => "Save kind",
        "（入力済み）" => "(has values)",
        "フリー" => "Free text",
        "選択" => "Choice",
        "フリー＋選択" => "Free text with choices",
        "数値" => "Number",
        "入力済みの項目は種類を変えられません。内容を空にしてからにしてください。" => {
            "A field with values in it cannot change kind. Empty the column first."
        }

        // --- 全プロジェクトのトークン ---------------------------------------
        "全プロジェクトの API トークン" => "API tokens for every project",
        "すべてのプロジェクトを読める API トークンです。案件をまたいだ集計に使います。1つのプロジェクトだけでよいなら、そのプロジェクトの設定で発行してください。" => {
            "An API token that reads every project, for numbers gathered across them. If one project is enough, issue the token in that project's settings."
        }
        "全案件の遅延を集める" => "Collecting lateness across projects",

        // --- 画面ぜんぶを英語にしたときの残り ---------------------------
        "進捗は 0〜100 の数値で入力してください。" => {
            "Progress is a number from 0 to 100."
        }
        "進捗は 0〜100 の範囲です。" => "Progress runs from 0 to 100.",
        "ステータスが不正です。" => "That is not one of the statuses.",
        "項目が指定されていません。" => "No column was named.",
        "期間は START/END の形式です。" => "A span is written START/END.",
        "終了日が開始日より前です。" => "The end is before the start.",
        "日付は YYYY-MM-DD の形式で入力してください。" => {
            "Dates are written YYYY-MM-DD."
        }
        "担当者を入力してください。" => "Give it an assignee.",
        "終了日は開始日より後にしてください。" => {
            "The end has to come after the start."
        }
        "休暇の日付を確認してください。" => "Check the dates of the leave.",
        "担当者を選んでください。" => "Choose an assignee.",
        "ステータス名を入力してください。" => "Give the status a name.",
        "進捗は 0〜100 で指定してください。" => "Progress runs from 0 to 100.",
        "ステータスは1つ以上必要です。" => "Keep at least one status.",
        "項目名を入力してください。" => "Give the column a name.",
        "項目の種類が不正です。" => "That is not one of the column kinds.",
        "担当者名を入力してください。" => "Give the person a name.",
        "選択肢を入力してください。" => "Give it a choice to offer.",
        "権限が不正です。" => "That is not one of the roles.",
        "そのメールアドレスの利用者が見つかりません。" => {
            "Nobody signs in with that address."
        }
        "最後の管理者は外せません。" => "The last administrator cannot be removed.",
        "ファイルを選んでください。" => "Choose a file.",
        "待ちは「8/17〜8/21」のように範囲で入力してください。" => {
            "A wait is a span, written 8/17–8/21."
        }
        "待ちの日付は「8/17」か「2026-08-17」の形式です。" => {
            "Waiting days are written 8/17 or 2026-08-17."
        }
        "日付は 20260805・8/5・2026-08-05 のように入力してください。" => {
            "Dates take 20260805, 8/5 or 2026-08-05."
        }
        "全員に有効です" => "Applies to everybody",
        "元号" => "Japanese eras",
        "パスワードの決まり" => "Password rule",
        "バイトではなく文字で数えます" => "Counted in characters, not bytes",
        "必ず入れる文字" => "Must contain",
        "いまの決まり: " => "The rule now: ",
        "まだ登録がありません。" => "Nothing here yet.",
        "文字色" => "Text colour",
        "色を外す" => "Clear the colour",
        "最低文字数は数字で入れてください。" => "The minimum length is a number.",
        "最低文字数は4〜128の範囲で決めてください。" => {
            "The minimum length runs from 4 to 128."
        }
        "名前を入力してください。" => "Give it a name.",
        "2020〜2099 年に対応しています。" => "Years from 2020 to 2099.",
        "（無題）" => "(untitled)",
        "（空）" => "(empty)",
        "パスワードは{}" => "Passwords are {}",
        "プロジェクト" => "Projects",
        "プロジェクト名を入力してください。" => "Give the project a name.",
        "その名前のプロジェクトはすでにあります。" => {
            "A project already has that name."
        }
        "閲覧のみの権限です。" => "You can read this one, not change it.",
        "読み込み中…" => "Loading…",
        "ステータスに連動（進捗を決めたステータスがまだありません）" => {
            "From the status (no status names a progress yet)"
        }
        "ステータスに連動（{}）" => "From the status ({})",
        "日数から除く日" => "Days left out of the count",
        "月曜" => "Monday",
        "火曜" => "Tuesday",
        "水曜" => "Wednesday",
        "木曜" => "Thursday",
        "金曜" => "Friday",
        "担当者の休暇" => "Leave",
        "固定しない" => "None pinned",
        "左から{}列" => "{} from the left",
        "狭い" => "Narrow",
        "やや狭い" => "Fairly narrow",
        "標準" => "Standard",
        "広い" => "Wide",
        "{} 種類" => "{} of them",
        "進捗 {}%" => "{}% done",
        "進捗は手入力" => "Progress stays typed",
        "この項目に入力した内容もすべて消えます。よろしいですか？" => {
            "Everything entered in this column goes with it. Delete it?"
        }
        "選択肢がありません。" => "No choices yet.",
        "まだ誰も出てきていません。" => "Nobody is on this plan yet.",
        "色なし" => "No colour",
        "全体の休みだが、このプロジェクトでは動く日" => {
            "A shared holiday this project works through"
        }
        "バーの色" => "Bar colours",
        "予定からずれているタスクはありません。" => "Nothing has slipped.",
        "このユーザーを削除します。よろしいですか？" => "Delete this user?",
        "その名前は別の人が使っています。担当者は名前で見分けるので、重ならない名前にしてください。" => {
            "Somebody else goes by that name. Assignees are told apart by name, so two people cannot share one."
        }
        "そのユーザー名はすでに使われています。" => {
            "Somebody already signs in with that."
        }
        "ユーザー名を入力してください。空白は使えません。" => {
            "Give a username. Spaces are not allowed."
        }
        "管理者は1人以上必要です。" => "Keep at least one administrator.",
        "自分は削除できません。" => "You cannot delete yourself.",
        "ユーザー名（{}）とベース権限は管理者が決めます。" => {
            "Your username ({}) and base role are the administrator's to set."
        }
        "全体の設定に従う" => "Follow the global setting",
        "いまのパスワード" => "Current password",
        "新しいパスワード" => "New password",
        "その名前は別の人が使っています。別の名前にしてください。" => {
            "Somebody else goes by that name. Pick another."
        }
        "いまのパスワードが違います。" => "That is not your current password.",
        // --- 行き止まり ---------------------------------------------------
        "そのページはありません" => "There is no such page",
        "消されたか、住所が違うか、見る権限が無いかのどれかです。" => {
            "It was deleted, the address is wrong, or it is not yours to see."
        }
        "プロジェクト一覧へ" => "Back to the projects",
        // --- 管理操作の記録 -----------------------------------------------
        "最近の管理操作" => "Recent changes to accounts",
        "ユーザーの追加・削除・権限変更を、誰がしたかと一緒に残します。" => {
            "Accounts added, removed and moved, with the name of whoever did it."
        }
        "まだ何もありません。" => "Nothing yet.",
        "権限変更" => "Role",
        "名前変更" => "Renamed",
        "パスワード変更" => "Password",
        "admin" => "administrator",
        "editor" => "editor",
        "viewer" => "viewer",
        "none" => "no default access",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_words_a_first_screen_needs_are_there() {
        for word in ["保存", "スケジュール", "設定", "予定", "実施"] {
            assert!(super::of(word).is_some(), "{word} が訳されていない");
        }
    }

    /// Data does not belong here. Translating a status name would turn the
    /// vocabulary a team agreed on into something else on an English screen,
    /// and two people would read the same plan in different words.
    #[test]
    fn user_data_is_not_translated() {
        for word in ["未着手", "実施中", "完了", "保留"] {
            assert!(super::of(word).is_none(), "{word} は訳してはいけない");
        }
    }
}
