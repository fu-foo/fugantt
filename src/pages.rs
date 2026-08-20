mod admin;
mod capacity;
mod history;
mod login;
mod projects;
mod settings;
mod stats;
mod users;

use topcoat::{
    Result,
    context::Cx,
    router::{layout, query_params},
    view::{component, view},
};

use crate::{auth::current_user, project, static_files};

/// Whether the memo panel should come back open.
///
/// Saving the memo posts and the page is built again, which unticks the
/// checkbox the panel hangs on — the note is saved and the panel shuts in the
/// same instant, which reads as the save having thrown the panel away.
#[query_params(error = not_found())]
struct MemoOpen {
    memo: Option<String>,
}

/// Wraps every page: document shell, stylesheet, and the drawer.
#[layout("/")]
async fn shell(cx: &Cx, slot: Result) -> Result {
    let user = current_user(cx).await?;

    let project = match &user {
        Some(user) => project::from_url(cx, &user.id).await,
        None => None,
    };

    let memo = match &project {
        Some(project) => project::settings(cx, &project.id)
            .await?
            .remove("memo")
            .unwrap_or_default(),
        None => String::new(),
    };

    let name = crate::app_settings::name(cx).await;

    let memo_open = query_params::<MemoOpen>(cx)
        .ok()
        .and_then(|query| query.memo.clone())
        .is_some();

    let admin = user.as_ref().is_some_and(|user| user.is_admin());

    // The language: the person's own choice, then the installation's, then the
    // browser's — which is to say the operating system's.
    let l = crate::i18n::lang(cx).await;

    // Light, dark, or nothing at all — and nothing at all means the stylesheet
    // asks the machine. One person's answer; the plan's own colours are the
    // project's and do not move.
    let theme = user
        .as_ref()
        .map(|user| user.theme.as_str())
        .filter(|theme| *theme == "light" || *theme == "dark")
        .unwrap_or_default();

    // Linked rather than inlined: a stylesheet in a `<style>` tag has to be
    // escaped past a `</style>` that would otherwise end it early, and the
    // browser caches this one like any other.
    let own_css = user
        .as_ref()
        .is_some_and(|user| !user.custom_css.trim().is_empty());

    view! {
        <!DOCTYPE html>
        <html lang=(l.code()) data-theme=(theme)>
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>(&name)</title>
                topcoat::dev::script()
                <link rel="icon" type="image/svg+xml" href=(static_files::favicon())>
                <link rel="stylesheet" href=(static_files::tailwind_css())>
                <link rel="stylesheet" href=(static_files::grid_css())>
                // After the others, so it can answer them.
                <link rel="stylesheet" href=(static_files::theme_css())>

                if own_css {
                    // Last of all: whoever wrote it gets the final word on their
                    // own screen.
                    <link rel="stylesheet" href="/me/custom.css">
                }
            </head>
            <body class="flex h-screen flex-col overflow-hidden bg-slate-50 text-slate-900">
                if user.is_some() {
                    // A checkbox drives the drawer: no script, and it survives
                    // the grid island taking over the rest of the page.
                    <input type="checkbox" id="fg-drawer" class="peer/nav sr-only">

                    <label
                        for="fg-drawer"
                        aria-hidden="true"
                        class="fixed inset-0 z-30 hidden bg-slate-900/20 peer-checked/nav:block"
                    ></label>

                    // Scrolls on its own. The page behind it does not scroll —
                    // the grid owns that — so a drawer taller than the window
                    // simply lost its last entries, and on a phone the
                    // administrator's section was the part that fell off.
                    <aside
                        class="fixed inset-y-0 left-0 z-40 flex w-72 -translate-x-full flex-col gap-6 overflow-y-auto overscroll-contain border-r border-slate-200 bg-white px-5 py-4 transition-transform duration-150 peer-checked/nav:translate-x-0"
                    >
                        <div class="flex items-center justify-between">
                            <span class="font-semibold tracking-tight">(&name)</span>
                            <label
                                for="fg-drawer"
                                class="cursor-pointer rounded-lg px-2 py-1 text-slate-400 hover:bg-slate-100"
                            >
                                "✕"
                            </label>
                        </div>

                        match &project {
                            Some(project) => project_menu(project: project, l: l),
                            None => "",
                        }

                        <nav class="flex flex-col gap-1 text-sm">
                            <span class="px-3 pb-1 text-xs tracking-wide text-slate-400">(l.t("自分"))</span>
                            drawer_link(href: "/", label: l.t("プロジェクト一覧"))
                            drawer_link(href: "/capacity", label: l.t("全体の空き検索"))
                            drawer_link(href: "/me", label: l.t("自分の設定"))
                        </nav>

                        if admin {
                            <nav class="flex flex-col gap-1 text-sm">
                                // Labelled, so it reads as the administrator's own group.
                                <span class="px-3 pb-1 text-xs tracking-wide text-slate-400">(l.t("管理"))</span>
                                drawer_link(href: "/users", label: l.t("ユーザー"))
                                drawer_link(href: "/admin", label: l.t("全体の設定"))
                            </nav>
                        }

                        if !crate::open_access::enabled() {
                            <form method="POST" action="/logout" class="mt-auto">
                                <button
                                    class="w-full rounded-lg px-3 py-2 text-left text-sm text-slate-500 hover:bg-slate-100"
                                >
                                    (l.t("ログアウト"))
                                </button>
                            </form>
                        }
                    </aside>
                }

                match &project {
                    Some(project) => memo_panel(project: project, memo: &memo, open: memo_open, l: l),
                    None => "",
                }

                // A standing reminder, not a one-off warning at startup that
                // nobody was there to read.
                if crate::open_access::enabled() {
                    <p class="bg-amber-100 px-4 py-1.5 text-center text-xs text-amber-900">
                        (l.t("認証なしで動いています。この URL に届く人は全員が全プロジェクトを読み書きできます。"))
                    </p>
                }

                <header class="border-b border-slate-200 bg-white">
                    <nav class="flex w-full items-center gap-3 px-4 py-2.5">
                        if user.is_some() {
                            <label
                                for="fg-drawer"
                                aria-label=(l.t("メニュー"))
                                class="cursor-pointer rounded-lg px-2.5 py-1.5 text-lg leading-none hover:bg-slate-100"
                            >
                                "☰"
                            </label>
                        }

                        <a href="/" class="font-semibold tracking-tight">(&name)</a>

                        match &project {
                            Some(project) => {
                                <span class="text-slate-300">"/"</span>
                                <span class="truncate text-sm text-slate-600">(&project.name)</span>

                                // The grid binds to this. Living outside the
                                // island keeps it out of the island's renders,
                                // so typing here cannot lose the caret.
                                <span id="fugantt-filter-count" class="ml-4 text-xs text-slate-400"></span>
                            }
                            None => "",
                        }

                        match &project {
                            Some(_) => {
                                <label
                                    for="fg-memo"
                                    class="ml-auto cursor-pointer rounded-lg border border-slate-300 px-3 py-1.5 text-sm hover:bg-slate-50"
                                >
                                    (l.t("メモ"))
                                    if !memo.is_empty() {
                                        <span class="ml-1.5 inline-block size-1.5 rounded-full bg-blue-500 align-middle"></span>
                                    }
                                </label>
                            }
                            None => "",
                        }

                        match &user {
                            Some(user) => {
                                <span
                                    class=(
                                        if project.is_some() { "ml-3 truncate text-sm text-slate-400" }
                                        else { "ml-auto truncate text-sm text-slate-400" }
                                    )
                                >
                                    (user.display())
                                </span>
                            }
                            None => "",
                        }
                    </nav>
                </header>

                // The page itself does not scroll: the grid inside it does, which is
                // what lets its headings stay put.
                <main class="w-full flex-1 min-h-0 overflow-y-auto px-4 py-6">(slot?)</main>
            </body>
        </html>
    }
}

/// Everything that acts on the open project.
#[component]
async fn project_menu(project: &project::Project, l: crate::i18n::Lang) -> Result {
    let base = format!("/projects/{}", project.id);
    let settings = format!("{base}/settings");
    let xlsx = format!("{base}/export.xlsx");
    let export_json = format!("{base}/export.json");
    let import_json = format!("{base}/import.json");
    let history = format!("{base}/history");
    let stats = format!("{base}/stats");
    let capacity = format!("{base}/capacity");

    view! {
        <nav class="flex flex-col gap-1 text-sm">
            <span class="px-3 pb-1 text-xs tracking-wide text-slate-400">(l.t("このプロジェクト"))</span>

            // In the order they are looked at: every day, now and then, rarely.
            drawer_link(href: &base, label: l.t("スケジュール"))
            drawer_link(href: &stats, label: l.t("統計"))
            drawer_link(href: &capacity, label: l.t("空き検索"))
            drawer_link(href: &history, label: l.t("タスク変更履歴"))
            drawer_link(href: &settings, label: l.t("設定"))

            <span class="mt-3 px-3 pb-1 text-xs tracking-wide text-slate-400">(l.t("データの入出力"))</span>

            drawer_link(href: &xlsx, label: l.t("Excel で書き出す"))

            // Two buttons rather than a link and a checkbox: a checkbox that is
            // not ticked sends nothing at all, so the off position would arrive
            // looking exactly like never having been asked. Each button carries
            // its own answer.
            <form method="GET" action=(&export_json) class="flex flex-col items-start gap-1 px-3 py-1.5">
                // Both spelled out. One line under the other, the shorter one
                // read as a caption on the first rather than as the other
                // choice — and a choice nobody sees is not one.
                <button
                    name="settings"
                    value="1"
                    class="text-left text-sm underline-offset-2 hover:underline"
                >
                    (l.t("JSON で書き出す（タスク＋設定）"))
                </button>
                <button
                    name="settings"
                    value="0"
                    class="text-left text-sm underline-offset-2 hover:underline"
                    title=(l.t("設定・名簿・暦を入れずに、タスクだけを書き出します"))
                >
                    (l.t("JSON で書き出す（タスク）"))
                </button>
            </form>

            if project.can_edit() {
                <form
                    method="POST"
                    action=(&import_json)
                    enctype="multipart/form-data"
                    class="mt-2 flex flex-col gap-2 rounded-lg bg-slate-50 p-3"
                >
                    <span class="text-xs text-slate-500">(l.t("JSON を取り込む（全置換）"))</span>
                    <input
                        type="file"
                        name="document"
                        accept=".json,application/json"
                        required=""
                        class="text-xs text-slate-500 file:mr-2 file:rounded-md file:border file:border-slate-300 file:bg-white file:px-2 file:py-1 file:text-xs"
                    >
                    <button
                        class="rounded-md border border-slate-300 bg-white px-3 py-1.5 text-xs hover:bg-slate-100"
                        onclick=(&format!("return confirm('{}')", l.t("取り込むと、いまのタスクはすべて置き換わります。よろしいですか？")))
                    >
                        (l.t("取り込む"))
                    </button>
                </form>
            }

        </nav>
    }
}

#[component]
async fn drawer_link(href: &str, label: &str) -> Result {
    view! {
        <a href=(href) class="rounded-lg px-3 py-2 hover:bg-slate-100">(label)</a>
    }
}

/// The memo, in a panel that slides in from the right.
///
/// It belongs beside the schedule rather than under it: notes are read while
/// looking at the plan, not after scrolling past it.
#[component]
async fn memo_panel(
    project: &project::Project,
    memo: &str,
    open: bool,
    l: crate::i18n::Lang,
) -> Result {
    let action = format!("/projects/{}/memo", project.id);

    view! {
        <input
            type="checkbox"
            id="fg-memo"
            checked=(open.then_some("checked"))
            class="peer/memo sr-only"
        >

        <label
            for="fg-memo"
            aria-hidden="true"
            class="fixed inset-0 z-30 hidden bg-slate-900/20 peer-checked/memo:block"
        ></label>

        <aside
            class="fixed inset-y-0 right-0 z-40 flex w-[26rem] max-w-full translate-x-full flex-col border-l border-slate-200 bg-white transition-transform duration-150 peer-checked/memo:translate-x-0"
        >
            <div class="flex items-center justify-between border-b border-slate-200 px-5 py-3">
                <span class="font-semibold tracking-tight">(l.t("プロジェクトメモ"))</span>
                <label
                    for="fg-memo"
                    class="cursor-pointer rounded-lg px-2 py-1 text-slate-400 hover:bg-slate-100"
                >
                    "✕"
                </label>
            </div>

            <div class="flex-1 overflow-y-auto px-5 py-4">
                if !memo.is_empty() {
                    // Plain text, shown as written: the memo is a note to the
                    // next person, not a document. Escaped like anything else,
                    // and `pre-wrap` keeps the line breaks that were typed.
                    <p class="whitespace-pre-wrap text-sm text-slate-700">(memo)</p>
                }

                if project.can_edit() {
                    <details class="mt-4" open=(memo.is_empty().then_some("open"))>
                        <summary class="cursor-pointer text-sm text-slate-500">(l.t("編集"))</summary>

                        <form method="POST" action=(&action) class="mt-3 flex flex-col gap-3">
                            <textarea
                                name="memo"
                                rows="14"
                                placeholder=(l.t("引き継ぎや決めごとなど。改行はそのまま出ます"))
                                class="w-full rounded-lg border border-slate-300 px-3 py-2 font-mono text-xs"
                            >(memo)</textarea>
                            <button
                                class="w-fit rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-500"
                            >
                                (l.t("保存"))
                            </button>
                        </form>
                    </details>
                }

                if memo.is_empty() && !project.can_edit() {
                    <p class="text-sm text-slate-400">(l.t("まだ何も書かれていません。"))</p>
                }
            </div>
        </aside>
    }
}
