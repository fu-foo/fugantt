use topcoat::{
    Result,
    context::Cx,
    router::{page, query_params},
    view::view,
};

use crate::{auth::require_user, history, project};

/// Rows per page: not what fits on a screen, but what fits a day's changes.
const PER_PAGE: i64 = 100;

#[query_params(error = not_found())]
struct Page {
    page: Option<String>,
}

/// What has happened to this project, newest first.
#[page("/projects/{project_id}/history")]
async fn index(cx: &Cx) -> Result {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let project = project::authorize(cx, &user.id, &project_id).await?;

    let l = crate::i18n::lang(cx).await;
    let total = history::count(cx, &project.id).await?;
    let pages = ((total + PER_PAGE - 1) / PER_PAGE).max(1);

    // Counted from one. It appears in the URL, and a zero would need explaining.
    let current = query_params::<Page>(cx)?
        .page
        .as_deref()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(1)
        .clamp(1, pages);

    let changes = history::list(cx, &project.id, PER_PAGE, (current - 1) * PER_PAGE).await?;

    let link = |page: i64| format!("/projects/{}/history?page={page}", project.id);

    view! {
        <div class="mx-auto w-full max-w-4xl">
            <h1 class="text-2xl font-bold tracking-tight">"タスク変更履歴"</h1>
            <p class="mt-1 text-sm text-slate-500">
                (&project.name)
                if total > 0 {
                    <span class="ml-2 text-slate-400">
                        (&format!("{total} 件中 {}〜{} 件目", (current - 1) * PER_PAGE + 1,
                                  ((current - 1) * PER_PAGE + changes.len() as i64)))
                    </span>
                }
            </p>

            if changes.is_empty() {
                <p class="mt-10 text-center text-sm text-slate-400">"まだ変更はありません。"</p>
            } else {
                <ul class="mt-6 divide-y divide-slate-100 rounded-xl border border-slate-200 bg-white">
                    for change in &changes {
                        <li class="flex flex-wrap items-baseline gap-x-3 gap-y-1 px-5 py-3 text-sm">
                            <span class="w-36 shrink-0 font-mono text-xs tabular-nums text-slate-400">
                                (&l.stamp(change.at))
                            </span>

                            <span class="w-28 shrink-0 truncate text-slate-500">(&change.actor)</span>

                            <span class="rounded-full bg-slate-100 px-2 py-0.5 text-xs text-slate-600">
                                (&change.action)
                            </span>

                            <span class="font-medium">
                                if change.task_name.is_empty() { "（無題）" } else { (&change.task_name) }
                            </span>

                            if !change.field.is_empty() {
                                <span class="text-slate-400">(&change.field)</span>
                            }

                            if change.action == "変更" {
                                <span class="text-slate-500">
                                    if change.before.is_empty() { "（空）" } else { (&change.before) }
                                    " → "
                                </span>
                                <span class="font-medium">
                                    if change.after.is_empty() { "（空）" } else { (&change.after) }
                                </span>
                            }
                        </li>
                    }
                </ul>

                if pages > 1 {
                    <nav class="mt-4 flex items-center justify-between text-sm">
                        if current > 1 {
                            <a
                                href=(&link(current - 1))
                                class="rounded-lg border border-slate-300 bg-white px-3 py-1.5 hover:bg-slate-50"
                            >
                                (l.t("← 新しい"))
                            </a>
                        } else {
                            // Keep the frame at either end: a target that moves
                            // sideways is a target people miss.
                            <span></span>
                        }

                        <span class="text-slate-500">(&format!("{current} / {pages} ページ"))</span>

                        if current < pages {
                            <a
                                href=(&link(current + 1))
                                class="rounded-lg border border-slate-300 bg-white px-3 py-1.5 hover:bg-slate-50"
                            >
                                (l.t("古い →"))
                            </a>
                        } else {
                            <span></span>
                        }
                    </nav>
                }
            }
        </div>
    }
}
