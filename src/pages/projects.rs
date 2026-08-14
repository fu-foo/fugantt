use serde::Deserialize;
use sqlx::FromRow;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, bad_request, see_other},
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
          ORDER BY projects.updated_at DESC",
    )
    .bind(&user.id)
    .fetch_all(db::pool(cx))
    .await?;

    view! {
        <div class="mx-auto w-full max-w-4xl">
        <div class="flex items-center justify-between gap-4">
            <h1 class="text-2xl font-bold tracking-tight">"プロジェクト"</h1>

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
                for project in &projects {
                    <li>
                        <a
                            href=(("/projects/", &project.id))
                            class="flex items-center justify-between px-5 py-4 hover:bg-slate-50"
                        >
                            <span class="font-medium">(&project.name)</span>
                            <span class="text-sm text-slate-500">
                                (project.task_count)
                                (l.t(" タスク"))
                            </span>
                        </a>
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

    let name = form.name.trim();
    if name.is_empty() {
        return Err(bad_request("プロジェクト名を入力してください。").into());
    }

    // The name is the URL, so two projects cannot share one. Refusing here is
    // clearer than quietly handing out "リリース計画-2".
    if project::name_taken(cx, name).await? {
        return Err(bad_request("その名前のプロジェクトはすでにあります。").into());
    }

    let id = project::available_id(cx, name).await?;
    let now = db::now();
    let mut tx = db::pool(cx).begin().await?;

    sqlx::query(
        "INSERT INTO projects (id, name, owner_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
    )
    .bind(&id)
    .bind(name)
    .bind(&user.id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // The owner is a member too, so membership alone answers "may I see this?".
    sqlx::query("INSERT INTO project_members (project_id, user_id, role) VALUES (?1, ?2, 'owner')")
        .bind(&id)
        .bind(&user.id)
        .execute(&mut *tx)
        .await?;

    // The statuses come across as a copy. From here the project owns them, and
    // a later change to the installation's list leaves this plan alone.
    for (position, status) in project::default_statuses(cx).await?.iter().enumerate() {
        sqlx::query(
            "INSERT INTO project_statuses (project_id, position, name, color, percent)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(&id)
        .bind(position as i64)
        .bind(&status.name)
        .bind(&status.color)
        .bind(status.percent)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(see_other(&format!("/projects/{id}")))
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
            <p class="mb-3 text-xs text-slate-500">"閲覧のみの権限です。"</p>
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
            <p class="px-5 py-16 text-center text-slate-400">"読み込み中…"</p>
        </div>

        <script src=(crate::static_files::grid_js()) defer=""></script>
    }
}
