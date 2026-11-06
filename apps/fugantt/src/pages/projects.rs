use serde::Deserialize;
use sqlx::FromRow;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, forbidden, see_other},
        page, query_params, route,
    },
    view::view,
};

use crate::{auth::require_user, db, project};

#[derive(FromRow)]
struct ProjectRow {
    id: String,
    name: String,
    task_count: i64,
}

/// The signed-in user's projects.
#[page("/")]
async fn index(cx: &Cx) -> Result {
    let user = require_user(cx).await?;
    let l = crate::i18n::lang(cx).await;

    let projects = sqlx::query_as::<_, ProjectRow>(
        "SELECT projects.id,
                projects.name,
                (SELECT COUNT(*) FROM tasks WHERE tasks.project_id = projects.id) AS task_count
           FROM projects
           LEFT JOIN project_members
             ON project_members.project_id = projects.id
            AND project_members.user_id = ?1
          WHERE projects.owner_id = ?1
             OR project_members.user_id = ?1
             -- ベース権限が「無効」でない人は、名前が無くても一覧に出る。
             OR (SELECT base_role FROM users WHERE id = ?1) <> 'none'
          GROUP BY projects.id
          ORDER BY projects.position, projects.name",
    )
    .bind(&user.id)
    .fetch_all(db::pool(cx))
    .await?;

    view! {
        <div class="mx-auto w-full max-w-4xl">
        <div class="flex items-center justify-between gap-4">
            <h1 class="text-2xl font-bold tracking-tight">(l.t("プロジェクト"))</h1>

            <form method="POST" action="/projects" class="flex gap-2">
                <input
                    name="name"
                    placeholder=(l.t("新しいプロジェクト"))
                    required=""
                    class="rounded-lg border border-slate-300 px-3 py-2"
                >
                <button
                    class="rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                >
                    (l.t("作成"))
                </button>
            </form>
        </div>

        if projects.is_empty() {
            <p class="mt-10 text-center text-slate-500">
                (l.t("まだプロジェクトがありません。"))
            </p>
        } else {
            <ul class="mt-6 divide-y divide-slate-200 rounded-xl border border-slate-200 bg-white">
                for (at, project) in projects.iter().enumerate() {
                    <li class="flex items-center">
                        <a
                            href=(("/projects/", &project.id))
                            class="flex min-w-0 flex-1 items-center justify-between px-5 py-4 hover:bg-slate-50"
                        >
                            <span class="font-medium">(&project.name)</span>
                            <span class="text-sm text-slate-500">
                                (project.task_count)
                                (l.t(" タスク"))
                            </span>
                        </a>

                        if user.is_admin() {
                            <form method="POST" action="/projects/move" class="flex gap-1 pr-3">
                                <input type="hidden" name="id" value=(&project.id)>
                                <button
                                    name="direction"
                                    value="up"
                                    title=(l.t("上へ"))
                                    aria-label=(l.t("上へ"))
                                    disabled=((at == 0).then_some(""))
                                    class="rounded border border-slate-200 px-1.5 text-xs text-slate-500 hover:bg-slate-50 disabled:opacity-30"
                                >
                                    "↑"
                                </button>
                                <button
                                    name="direction"
                                    value="down"
                                    title=(l.t("下へ"))
                                    aria-label=(l.t("下へ"))
                                    disabled=((at + 1 == projects.len()).then_some(""))
                                    class="rounded border border-slate-200 px-1.5 text-xs text-slate-500 hover:bg-slate-50 disabled:opacity-30"
                                >
                                    "↓"
                                </button>
                            </form>
                        }
                    </li>
                }
            </ul>
        }
        </div>
    }
}

#[derive(Deserialize)]
struct NewProject {
    name: String,
}

#[route(POST "/projects")]
async fn create(cx: &Cx, Form(form): Form<NewProject>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let id = project::create(cx, &form.name, &user.id).await?;

    Ok(see_other(&format!("/projects/{id}")))
}

#[derive(Deserialize)]
struct MoveProject {
    id: String,
    direction: String,
}

/// Moves a project one place up or down the list.
///
/// The list is the same for everyone, so who may rearrange it is the person
/// who looks after the installation.
#[route(POST "/projects/move")]
async fn move_project(cx: &Cx, Form(form): Form<MoveProject>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    if !user.is_admin() {
        return Err(forbidden().into());
    }

    project::move_in_list(cx, form.id.trim(), form.direction == "up").await?;

    Ok(see_other("/"))
}

/// What an import left behind, carried back on the redirect.
#[query_params(error = not_found())]
struct ImportOutcome {
    imported: Option<usize>,
    skipped: Option<usize>,
}

/// The project's schedule. Everything inside `#fugantt-grid` belongs to the
/// island, which fetches its own data and owns that subtree from then on.
#[page("/projects/{project_id}")]
async fn show(cx: &Cx) -> Result {
    let user = require_user(cx).await?;
    let l = crate::i18n::lang(cx).await;
    let project_id = project::id_from_path(cx)?.to_owned();
    let project = project::authorize(cx, &user.id, &project_id).await?;

    let result = query_params::<ImportOutcome>(cx)?;

    view! {
        if !project.can_edit() {
            <p class="mb-3 text-xs text-slate-500">(l.t("閲覧のみの権限です。"))</p>
        }

        if let Some(imported) = result.imported {
            <p
                class="mt-4 rounded-lg border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-900"
            >
                (imported)
                (l.t(" 行を取り込みました。"))

                if result.skipped.unwrap_or(0) > 0 {
                    (result.skipped.unwrap_or(0))
                    (l.t(" 行は読めなかったため飛ばしています。"))
                }
            </p>
        }

        <div
            id="fugantt-grid"
            data-project=(&project.id)
            class="flex h-full flex-col overflow-hidden rounded-xl border border-slate-200 bg-white"
        >
            <p class="px-5 py-16 text-center text-slate-400">(l.t("読み込み中…"))</p>
        </div>

        <script src=(crate::static_files::grid_js()) defer=""></script>
    }
}
