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
        "変更履歴" => "History",
        "設定" => "Settings",
        "データの入出力" => "Import and export",
        "Excel で書き出す" => "Export to Excel",
        "JSON で書き出す" => "Export as JSON",
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
        "作業の遅れ" => "Work behind",
        "ずれの内訳" => "Where the slippage went",
        "ずれ" => "Slippage",
        "待ち" => "Waiting",
        "ステータス" => "Status",
        "担当者" => "Assignee",
        "（未割当）" => "(unassigned)",
        "（理由なし）" => "(no reason given)",

        // --- settings -----------------------------------------------------
        "表示" => "Display",
        "列" => "Columns",
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
        "休暇" => "Leave",
        "進捗" => "Progress",
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
        "認証なしで動いています。この URL に届く人は全員が全プロジェクトを読み書きできます。" =>
            "Running without sign-in. Anyone who can reach this URL can read and edit every project.",
        "取り込むと、いまのタスクはすべて置き換わります。よろしいですか？" =>
            "Importing replaces every task in this project. Continue?",
        "引き継ぎや決めごとなど。改行はそのまま出ます" =>
            "Handover notes, decisions, anything. Line breaks are kept.",
        "まだ何も書かれていません。" => "Nothing written yet.",

        // --- signing in ---------------------------------------------------
        "最初のアカウントを作る" => "Create the first account",
        "最初に登録した人が管理者になり、以降のユーザーはその人が作ります。" =>
            "Whoever registers first becomes the administrator, and makes every account after that.",
        "画面にはこの名前が出ます。担当者としても選べます。" =>
            "This is the name shown on screen, and the one you can assign work to.",
        "山田 太郎" => "Alex Doe",

        // --- the project list ---------------------------------------------
        "新しいプロジェクト" => "New project",
        "作成" => "Create",
        "まだプロジェクトがありません。" => "No projects yet.",

        // --- statistics -----------------------------------------------------
        "差異は待ちを除いて数えているので、作業の遅れと待ちを足したものが実際のずれです。" =>
            "Variance is counted with waiting excluded, so the real slippage is the work behind plus the waiting.",
        "平均" => "average",

        // --- settings and administration -----------------------------------
        "アプリの名前（左上に出ます）" => "The name of this app (shown top left)",
        "既定です。自分の設定で選んだ人は、そちらが優先されます。「自動」は、その人のブラウザ（OS）の言語に合わせます。" => "The default. Anyone who picks a language in their own settings keeps that choice. \"Automatic\" follows each person's browser, which follows their operating system.",
        "1行に「開始日 名称」。新しい元号が決まったら、ここに1行足すだけで済みます。読めない行は無視します。" => "One era per line: start date, then name. When a new era is announced, adding a line here is the whole change. Lines that cannot be read are ignored.",
        "新しく決めるときだけ効きます。いま使っているパスワードは、次に変えるまでそのまま使えます。" => "This applies when a password is set. Passwords already in use keep working until they are next changed.",
        "最低文字数" => "Minimum length",
        "何もチェックしなければ指定なし。日本語は記号に数えます" => "Tick nothing to require none. Non-Latin characters count as symbols",
        "使わせない語" => "Words to refuse",
        "1行に1語。これを含むパスワードは断ります（大文字小文字は問いません）。会社名や製品名を足しておくと効きます。空にすれば、この検査はしません。" => "One word per line. A password containing any of them is refused, whatever the case. Adding your company or product name is worth doing. Leave it empty to skip this check.",
        "会社の暦です。すべてのプロジェクトが、まずここを見ます。現場ごとの違いは、それぞれのプロジェクトの設定で足したり外したりできます。" => "The company calendar. Every project starts from this list, and each one can add to it or opt out of a day in its own settings.",
        "日本の祝日をまとめて入れる" => "Add a year of Japanese public holidays",
        "振替休日と国民の休日も計算します。同じ日付が既にあれば残します。" => "Substitute holidays and citizens' holidays are worked out too. A date already on the list is left as it is.",
        "創立記念日" => "Founders' Day",
        "担当者の色" => "Assignee colours",
        "同じ人はどのプロジェクトでも同じ色にします。アカウントの無い名前（協力会社・他部署）も登録できます。" => "One person, one colour, in every project. Names without an account — contractors, other teams — can be listed here too.",
        "協力会社 A" => "Contractor A",
        "背景色" => "Background",
        "まだ登録がありません。プロジェクトで使われている名前は、そのまま色なしで出ます。" => "Nothing here yet. Names already used in a project still show, without a colour.",
        "更新" => "Update",
        " タスク" => " tasks",
        " 行を取り込みました。" => " rows imported.",
        " 行は読めなかったため飛ばしています。" => " rows could not be read and were skipped.",
        "日数の数え方は編集者が設定します。" => "How days are counted is set by an editor.",
        "週の何曜を休みにするかは現場ごと。既定はどれも外しません。" => "Which weekdays are off differs by workplace. By default none of them are.",
        "進捗の入れ方" => "How progress is set",
        "手入力" => "Typed in",
        "連動しても、進捗を決めていないステータスは手入力のままです。進捗を 100% にすると、実施終了が空なら今日の日付が入ります。" => "Even when linked, a status with no progress of its own leaves the number as typed. Setting progress to 100% fills in today as the actual end, if it is empty.",
        "年度の開始月" => "Business year starts in",
        "月" => "月",
        "固定する列" => "Frozen columns",
        "1日の幅" => "Width of a day",
        "四半期の帯を出す" => "Show the quarter band",
        "年を和暦で表示する" => "Show years as Japanese eras",
        "表示・幅（px、空欄で自動）・並び順。タスク名は先頭で固定です。↑↓ を押すと、入力中の幅も一緒に保存されます。" => "Shown or hidden, width in pixels (empty means automatic), and order. The task name stays first. Pressing the arrows saves the width you are typing as well.",
        "自動" => "Automatic",
        "名前・色・その状態が意味する進捗。進捗を空にすると、その状態では手入力のままになります。" => "Name, colour, and the progress that state implies. Leave the progress empty and that state keeps whatever was typed.",
        "レビュー中" => "In review",
        "進捗（任意）" => "Progress (optional)",
        "項目名" => "Field name",
        "製品" => "Product",
        "種類" => "Kind",
        "選択肢を追加" => "Add a choice",
        "文字" => "Text",
        "背景" => "Background",
        "メンバーとタスクに入っている名前が並びます。ここに名前を足せば、アカウントの無い人も選べます。色は全員で共通なので「全体の設定」で決めます——同じ人が案件ごとに違う色だと、いくつも開いたときに読めなくなるためです。" => "The names of members and of anyone written on a task. Add a name here and people without an account can be assigned work too. Colours are shared by everyone and live in the installation settings: one person in two colours is unreadable once you have several projects open.",
        "土日と同じように網かけします。日数の計算は変わりません。日本の祝日は「全体の設定」に入れておくと、どのプロジェクトにも出ます。ここで扱うのは、この現場だけの違いです。" => "Shaded like a weekend; the day count is unaffected. Public holidays belong in the installation settings, where every project picks them up. What you set here is this project's own difference.",
        "日付" => "Date",
        "名称" => "Name",
        "現場の休業日" => "Site closure",
        "このプロジェクトに追加" => "Add to this project",
        "全体" => "Shared",
        "このプロジェクトでは働く" => "Working here",
        "平均 " => "average ",
        "ここで作った人がログインできます。ベース権限は、そのプロジェクトに名前が無いときに使われる既定です。「無効」にすると、招かれたプロジェクトだけが見えます。" => "Anyone made here can sign in. The base role is what applies to a project that does not name them; \"no access\" means they see only the projects they were added to.",
        "ベース権限" => "Base role",
        "新しいパスワード（変えるときだけ）" => "New password (only to change it)",
        "そのまま" => "Unchanged",
        "画面に出る名前です。変えると、担当者に入っている自分の名前も一緒に変わります。" => "The name shown on screen. Changing it also changes your name wherever it is written on a task.",

        // --- API トークン ---------------------------------------------------
        "API トークン" => "API tokens",
        "ブラウザ以外からこのプロジェクトだけを読み書きするための鍵です。書き出した JSON を読ませて、考えさせて、書き戻す——その往復に使います。" =>
            "Keys that let something other than a browser read and write this project alone. For the loop of reading the plan, working out what should change, and writing it back.",
        "いま作ったトークンです。この画面を離れると二度と出ません。" =>
            "The token you just made. It is not shown again once you leave this page.",
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
