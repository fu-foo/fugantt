//! The JSON surface the grid island talks to.
//!
//! Pages are server-rendered, but the grid owns its own DOM and so needs data
//! rather than markup. Every endpoint here re-checks membership: the page
//! having rendered is not authorization.
//!
//! Mutations answer with the whole recomputed grid. An edit to one cell can
//! move every ancestor's dates and progress, so returning a patch would mean
//! working out the affected set twice — once here and once in the browser.

use futures_core::Stream;
use futures_util::stream;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, IntoResponse, Response,
        content::{
            Form, Json,
            multipart::Multipart,
            sse::{Event, KeepAlive, Sse},
        },
        error::{SeeOther, bad_request, forbidden, see_other},
        headers, query_params, route,
    },
};

use crate::{
    auth::require_user,
    db,
    domain::GridData,
    history,
    interop::{json, xlsx},
    live, project,
};

/// The project, refusing anyone who may not change it.
async fn authorize_edit(cx: &Cx, user_id: &str, project_id: &str) -> Result<project::Project> {
    let project = project::authorize(cx, user_id, project_id).await?;

    // A viewer is a member, so the project exists as far as they are concerned;
    // the honest answer to a write is "not allowed", not "not found".
    if !project.can_edit() {
        return Err(forbidden().into());
    }

    Ok(project)
}

/// Who is asking, when it may not be a browser.
///
/// A `Authorization: Bearer fug_…` token stands for one project and one role,
/// which is the whole point: something automated can be given the plan it needs
/// and nothing else. Without one this falls back to the signed-in person.
async fn actor(cx: &Cx, project_id: &str) -> Result<(project::Project, String)> {
    let bearer = headers(cx)
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty());

    if let Some(token) = bearer {
        let Some(opened) = crate::tokens::resolve(cx, token).await? else {
            return Err(forbidden().into());
        };

        // A token for another project is not a token for this one. One issued
        // for all of them is a token for every project, including the ones made
        // after it.
        if !opened.covers(project_id) {
            return Err(forbidden().into());
        }

        return Ok((
            project::Project {
                id: project_id.to_owned(),
                name: String::new(),
                revision: 0,
                role: opened.role,
            },
            opened.who,
        ));
    }

    // Not a redirect to the sign-in page: this is an API, and something waiting
    // for JSON has no use for a login form with a 200 on it.
    let user = crate::auth::current_user(cx).await?.ok_or_else(forbidden)?;

    Ok((
        project::authorize(cx, &user.id, project_id).await?,
        user.display().to_owned(),
    ))
}

/// The same, for something that intends to write.
async fn actor_edit(cx: &Cx, project_id: &str) -> Result<(project::Project, String)> {
    let (project, who) = actor(cx, project_id).await?;

    if !project.can_edit() {
        return Err(forbidden().into());
    }

    Ok((project, who))
}

/// Every project the caller can reach.
///
/// A token opens one project by name, so nothing could ask "which plans are
/// there". This is the answer, and the starting point for anything that wants
/// to look across them.
#[route(GET "/api/projects")]
async fn list_projects(cx: &Cx) -> Result<Json<Vec<project::Summary>>> {
    let reach = reach(cx).await?;

    Ok(Json(project::summaries(cx, &reach).await?))
}

/// The numbers, for every project at once.
///
/// The same arithmetic the statistics page does, per project rather than per
/// task: which plans are late, by how much, and how much of that was waiting on
/// somebody else. Counting this a project at a time is what a person does when
/// a tool will not do it for them.
#[route(GET "/api/summary")]
async fn summary(cx: &Cx) -> Result<Json<Vec<project::Numbers>>> {
    let reach = reach(cx).await?;
    let mut numbers = Vec::new();

    for summary in project::summaries(cx, &reach).await? {
        let project = project::Project {
            id: summary.id.clone(),
            name: summary.name.clone(),
            revision: summary.revision,
            role: "viewer".to_owned(),
        };

        numbers.push(project::numbers(cx, &project).await?);
    }

    Ok(Json(numbers))
}

/// Which projects this caller may see at all.
///
/// A token for one project sees that one; a token for all of them, or a signed
/// in person, sees what they are allowed to.
async fn reach(cx: &Cx) -> Result<project::Reach> {
    let bearer = headers(cx)
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty());

    if let Some(token) = bearer {
        let Some(opened) = crate::tokens::resolve(cx, token).await? else {
            return Err(forbidden().into());
        };

        return Ok(match opened.project_id {
            Some(id) => project::Reach::One(id),
            None => project::Reach::Everything,
        });
    }

    let user = crate::auth::current_user(cx).await?.ok_or_else(forbidden)?;

    Ok(project::Reach::Person(user.id))
}

#[route(GET "/api/projects/{project_id}/grid")]
async fn grid(cx: &Cx) -> Result<Json<GridData>> {
    let project_id = project::id_from_path(cx)?.to_owned();
    let (project, _) = actor(cx, &project_id).await?;

    // A token carries a role but no name, so the rest is filled in — without
    // touching the role it came with.
    let project = fill_in(cx, project).await?;

    Ok(Json(project::grid_data(cx, &project).await?))
}

/// Notes an import in the history.
///
/// One line rather than one per row: a file replaces the plan, and a hundred
/// rows of "changed" says less than "this arrived, from here". `who` is the
/// person, or which token it was — a change nobody can attribute is a change
/// nobody can ask about.
async fn record_import(cx: &Cx, project_id: &str, who: &str, rows: usize) -> Result<()> {
    history::record(
        cx,
        history::Entry {
            project_id,
            task_id: None,
            // Not a task: the whole plan arrived at once. Left blank, the page
            // reads it as an unnamed task and says so.
            task_name: "プロジェクト全体",
            action: "取り込み",
            field: "",
            before: "",
            after: &format!("{rows} 行"),
            actor: who,
        },
    )
    .await
}

/// Fills in what a token does not carry, leaving the role exactly as it was.
async fn fill_in(cx: &Cx, mut project: project::Project) -> Result<project::Project> {
    if let Some((name, revision)) = project::name_and_revision(cx, &project.id).await? {
        project.name = name;
        project.revision = revision;
    }

    Ok(project)
}

/// Which parts of the project the file carries.
///
/// `settings=0` writes the plan on its own. A program reading the tasks does
/// not want a page of colours and holiday dates in front of them, and a file
/// with no settings section leaves that part of the project alone on the way
/// back in.
#[query_params(error = bad_request("settings は 0 か 1 です。"))]
struct Sections {
    settings: Option<String>,
}

/// Whether the settings and the master lists travel with the tasks.
fn wants_settings(cx: &Cx) -> bool {
    query_params::<Sections>(cx)
        .ok()
        .and_then(|sections| sections.settings.clone())
        .is_none_or(|value| !matches!(value.trim(), "0" | "no" | "false" | "off"))
}

/// The plan as a document: the same file the export button hands out.
///
/// Read it, work out what should change, and post it back. This is the loop a
/// person cannot do by hand, and the reason tokens exist at all.
#[route(GET "/api/projects/{project_id}/document")]
async fn read_document(cx: &Cx) -> Result<Json<serde_json::Value>> {
    let project_id = project::id_from_path(cx)?.to_owned();
    let project = fill_in(cx, actor(cx, &project_id).await?.0).await?;

    let data = project::grid_data(cx, &project).await?;
    let extras = match wants_settings(cx) {
        true => Some(project::export_extras(cx, &project.id).await?),
        false => None,
    };
    let text = crate::interop::json::write(&project.name, &data, extras);

    Ok(Json(serde_json::from_str(&text).unwrap_or_default()))
}

#[route(POST "/api/projects/{project_id}/document")]
async fn write_document(
    cx: &Cx,
    Json(document): Json<serde_json::Value>,
) -> Result<Json<GridData>> {
    let project_id = project::id_from_path(cx)?.to_owned();
    let (project, who) = actor_edit(cx, &project_id).await?;
    let l = crate::i18n::lang(cx).await;

    let document = crate::interop::json::read(&document.to_string())
        .map_err(|message| bad_request(l.t("取り込めませんでした。").to_owned() + &message))?;

    // `updated_by` points at a real account, and a token has no person behind
    // it. Left empty rather than blamed on whoever happens to be signed in;
    // which token it was goes in the history instead.
    let account = match crate::auth::current_user(cx).await? {
        Some(user) => user.id,
        None => String::new(),
    };

    project::import_project(cx, &project.id, &account, &document).await?;
    record_import(cx, &project.id, &who, document.tasks.len()).await?;

    // Tell the open screens. Without this a plan written through the API sits
    // in the database until somebody happens to reload, which is the one thing
    // an automated write cannot ask for.
    bump_and_announce(cx, &project.id, &who).await?;

    let project = fill_in(cx, project).await?;

    Ok(Json(project::grid_data(cx, &project).await?))
}

/// What a mutation gives back: the new state, and the row it concerns.
#[derive(Serialize)]
struct Mutation {
    grid: GridData,
    task_id: Option<String>,
    /// Why a request that succeeded still changed nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Field {
    Name,
    Start,
    End,
    Progress,
    Status,
    Assignee,
    Note,
    /// 実施開始 / 実施終了.
    ActualStart,
    ActualEnd,
    /// Waiting periods, as many as needed: `8/17〜8/21, 9/1〜9/3`.
    Waits,
    /// 予定進捗: the checkpoints the plan names — `8/20 30%, 8/28 100%`.
    Targets,
    /// One of the project's own columns, named by `field_id`.
    Custom,
    /// Both dates at once, as `START/END`. Dragging a bar moves them together,
    /// and sending them apart would put the row through an invalid state.
    Schedule,
    /// 実施開始 / 実施終了 together, for the same reason.
    ActualSchedule,
}

/// Free text fields are capped so one paste cannot bloat every grid fetch.
const TEXT_LIMIT: usize = 500;

#[derive(Deserialize)]
struct CellEdit {
    field: Field,
    /// Which project-defined column, when `field` is `custom`.
    field_id: Option<String>,
    /// The raw cell text. An empty string clears a date.
    value: String,
    /// What the sender believes is there now, as it is stored.
    ///
    /// Undo sends this. Putting a value back is only right if the value it is
    /// putting back is still the one that replaced it — otherwise the person
    /// pressing Ctrl+Z would silently throw away somebody else's work, which is
    /// the one thing an undo must never do. Absent means "write it regardless",
    /// which is what an ordinary edit means.
    expect: Option<String>,
}

/// What a cell holds now, in the shape a caller would send it back.
///
/// Two of the fields cover two columns at once, and one lives in another table
/// entirely, so this is not simply the column's text.
async fn current_cell(cx: &Cx, task_id: &str, edit: &CellEdit) -> String {
    let pair = |a: String, b: String| format!("{a}/{b}");

    match edit.field {
        Field::Schedule => pair(
            history::current_value(cx, task_id, "start_date").await,
            history::current_value(cx, task_id, "end_date").await,
        ),
        Field::ActualSchedule => pair(
            history::current_value(cx, task_id, "actual_start").await,
            history::current_value(cx, task_id, "actual_end").await,
        ),
        Field::Custom => match edit.field_id.as_deref() {
            Some(field_id) => project::field_value(cx, task_id, field_id).await,
            None => String::new(),
        },
        _ => {
            let column = column_of(edit.field);
            match column.is_empty() {
                true => String::new(),
                false => history::current_value(cx, task_id, column).await,
            }
        }
    }
}

/// The column one field is stored in. Empty for the fields that are not one.
fn column_of(field: Field) -> &'static str {
    match field {
        Field::Name => "name",
        Field::Start => "start_date",
        Field::End => "end_date",
        Field::ActualStart => "actual_start",
        Field::ActualEnd => "actual_end",
        Field::Progress => "progress",
        Field::Status => "status",
        Field::Assignee => "assignee",
        Field::Note => "note",
        Field::Waits => "waits",
        Field::Targets => "targets",
        Field::Custom => "",
        Field::Schedule => "start_date",
        Field::ActualSchedule => "actual_start",
    }
}

#[route(POST "/api/projects/{project_id}/tasks/{task_id}")]
async fn update_task(cx: &Cx, Json(edit): Json<CellEdit>) -> Result<Json<Mutation>> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let task_id = project::path_str(cx, "task_id")?.to_owned();

    let project = authorize_edit(cx, &user.id, &project_id).await?;
    project::task_in_project(cx, &project_id, &task_id).await?;

    // A parent's schedule is the sum of its children's, so writing to it would
    // be silently discarded on the next read. Refuse instead of pretending.
    let derived = matches!(
        edit.field,
        Field::Start
            | Field::End
            | Field::ActualStart
            | Field::ActualEnd
            | Field::Progress
            | Field::Schedule
            | Field::ActualSchedule
    );
    if derived && project::has_children(cx, &task_id).await? {
        return Err(forbidden().into());
    }

    let value = edit.value.trim();

    // Read before writing: the old value stops existing the moment it is
    // replaced, and history is exactly the record of what it used to be.
    let name = history::task_name(cx, &task_id).await;

    // An undo that arrives after somebody else has touched the same cell is an
    // undo of their work, not of yours. Refused, and said out loud, so the
    // person can look before deciding.
    if let Some(expect) = &edit.expect
        && current_cell(cx, &task_id, &edit).await != *expect
    {
        return Err(bad_request(l.t("他の人が先に変更しています。取り消しませんでした。")).into());
    }

    let column = column_of(edit.field);
    let before = if column.is_empty() {
        String::new()
    } else {
        history::current_value(cx, &task_id, column).await
    };

    // What this edit leaves the row's progress at, when it is about progress at
    // all. The follow-on writes below need it.
    let mut progress_now: Option<i64> = None;

    match edit.field {
        Field::Name => {
            write_cell(cx, &task_id, &user.id, "name = ?1", value.to_owned()).await?;
        }
        Field::Start | Field::End | Field::ActualStart | Field::ActualEnd => {
            let date = parse_date(value, l)?;
            check_order(cx, &task_id, edit.field, date.as_deref()).await?;

            let column = match edit.field {
                Field::Start => "start_date = ?1",
                Field::End => "end_date = ?1",
                Field::ActualStart => "actual_start = ?1",
                _ => "actual_end = ?1",
            };
            write_cell(cx, &task_id, &user.id, column, date).await?;
        }
        Field::Progress => {
            let progress: i64 = normalize_width(value)
                .trim()
                .trim_end_matches('%')
                .parse()
                .map_err(|_| bad_request(l.t("進捗は 0〜100 の数値で入力してください。")))?;

            if !(0..=100).contains(&progress) {
                return Err(bad_request(l.t("進捗は 0〜100 の範囲です。")).into());
            }

            write_cell(cx, &task_id, &user.id, "progress = ?1", progress).await?;
            progress_now = Some(progress);
        }
        Field::Status => {
            let statuses = project::statuses(cx, &project_id).await?;

            if !value.is_empty() && !statuses.iter().any(|status| status.name == value) {
                return Err(bad_request(l.t("ステータスが不正です。")).into());
            }

            write_cell(cx, &task_id, &user.id, "status = ?1", value.to_owned()).await?;
        }
        Field::Assignee => {
            write_cell(cx, &task_id, &user.id, "assignee = ?1", trim(value)).await?;
        }
        Field::Note => {
            write_cell(cx, &task_id, &user.id, "note = ?1", trim(value)).await?;
        }
        Field::Waits => {
            let stored = parse_waits(value, l)?;
            write_cell(cx, &task_id, &user.id, "waits = ?1", stored).await?;
        }
        Field::Targets => {
            let stored = parse_targets(value, l)?;
            write_cell(cx, &task_id, &user.id, "targets = ?1", stored).await?;
        }
        Field::Custom => {
            let field_id = edit
                .field_id
                .as_deref()
                .ok_or_else(|| bad_request(l.t("項目が指定されていません。")))?;

            project::set_field_value(cx, &project_id, &task_id, field_id, &trim(value)).await?;
        }
        Field::Schedule | Field::ActualSchedule => {
            let normalized = normalize_width(value);
            let (start, end) = normalized
                .split_once('/')
                .ok_or_else(|| bad_request(l.t("期間は START/END の形式です。")))?;

            let (start, end) = (parse_date(start, l)?, parse_date(end, l)?);

            if let (Some(start), Some(end)) = (&start, &end)
                && end < start
            {
                return Err(bad_request(l.t("終了日が開始日より前です。")).into());
            }

            let statement = if matches!(edit.field, Field::Schedule) {
                "UPDATE tasks SET start_date = ?1, end_date = ?2, updated_at = ?3, updated_by = ?4
                  WHERE id = ?5"
            } else {
                "UPDATE tasks SET actual_start = ?1, actual_end = ?2, updated_at = ?3, updated_by = ?4
                  WHERE id = ?5"
            };

            sqlx::query(statement)
                .bind(&start)
                .bind(&end)
                .bind(db::now())
                .bind(&user.id)
                .bind(&task_id)
                .execute(db::pool(cx))
                .await?;
        }
    }

    history::record(
        cx,
        history::Entry {
            project_id: &project_id,
            task_id: Some(&task_id),
            task_name: &name,
            action: "変更",
            field: field_label(edit.field),
            before: &before,
            after: value,
            actor: user.display(),
        },
    )
    .await?;

    follow_on(
        cx,
        &project_id,
        &task_id,
        &user,
        &name,
        edit.field,
        value,
        &mut progress_now,
    )
    .await?;

    respond(cx, &project, user.display(), Some(task_id)).await
}

/// The writes an edit implies, which someone would otherwise make by hand.
///
/// Only mechanical facts are filled in — a task marked 完了 is at 100%, and a
/// task at 100% finished on a day. What is already recorded is left alone, and
/// anything filled in here can be typed over afterwards.
#[allow(clippy::too_many_arguments)]
async fn follow_on(
    cx: &Cx,
    project_id: &str,
    task_id: &str,
    user: &crate::auth::User,
    name: &str,
    field: Field,
    value: &str,
    progress_now: &mut Option<i64>,
) -> Result<()> {
    // A parent's progress and dates come from its children, so a value written
    // here would be stored and then ignored on the next read — and the history
    // would show a change the grid never displays.
    let acts = matches!(field, Field::Status) || *progress_now == Some(100);
    if !acts || project::has_children(cx, task_id).await? {
        return Ok(());
    }

    fn note<'a>(
        project_id: &'a str,
        task_id: &'a str,
        name: &'a str,
        actor: &'a str,
        field: &'a str,
        before: &'a str,
        after: &'a str,
    ) -> history::Entry<'a> {
        history::Entry {
            project_id,
            task_id: Some(task_id),
            task_name: name,
            action: "変更",
            field,
            before,
            after,
            actor,
        }
    }

    // 進捗 from ステータス, for the projects that asked for it. Which states
    // imply a number, and which one, is the project's own answer.
    if matches!(field, Field::Status) && progress_from_status(cx, project_id).await? {
        let target = project::statuses(cx, project_id)
            .await?
            .into_iter()
            .find(|status| status.name == value)
            .and_then(|status| status.percent);

        if let Some(target) = target {
            let before = history::current_value(cx, task_id, "progress").await;

            if before != target.to_string() {
                write_cell(cx, task_id, &user.id, "progress = ?1", target).await?;
                let after = target.to_string();
                let entry = note(
                    project_id,
                    task_id,
                    name,
                    user.display(),
                    "進捗",
                    &before,
                    &after,
                );
                history::record(cx, entry).await?;
            }

            *progress_now = Some(target);
        }
    }

    // 100% means the work finished; record the day, unless it is already known.
    if *progress_now == Some(100) {
        let before = history::current_value(cx, task_id, "actual_end").await;

        if before.is_empty() {
            let today = jiff::Zoned::now().date().to_string();

            write_cell(cx, task_id, &user.id, "actual_end = ?1", today.clone()).await?;
            let entry = note(
                project_id,
                task_id,
                name,
                user.display(),
                "実施終了",
                "",
                &today,
            );
            history::record(cx, entry).await?;
        }
    }

    Ok(())
}

/// Whether 進捗 follows ステータス in this project.
async fn progress_from_status(cx: &Cx, project_id: &str) -> Result<bool> {
    Ok(project::settings(cx, project_id)
        .await?
        .get("progress_mode")
        .is_some_and(|mode| mode == "status"))
}

fn field_label(field: Field) -> &'static str {
    match field {
        Field::Name => "タスク名",
        Field::Start => "予定開始",
        Field::End => "予定終了",
        Field::ActualStart => "実施開始",
        Field::ActualEnd => "実施終了",
        Field::Progress => "実進捗",
        Field::Status => "ステータス",
        Field::Assignee => "担当者",
        Field::Note => "コメント",
        Field::Waits => "待ち",
        Field::Targets => "予定進捗",
        Field::Custom => "独自項目",
        Field::Schedule => "期間",
        Field::ActualSchedule => "実施期間",
    }
}

#[derive(Deserialize)]
struct InsertTask {
    /// Insert below this row, keeping it among the same siblings. `None`
    /// appends to the top level.
    after: Option<String>,
}

#[route(POST "/api/projects/{project_id}/tasks")]
async fn insert_task(cx: &Cx, Json(insert): Json<InsertTask>) -> Result<Json<Mutation>> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let project = authorize_edit(cx, &user.id, &project_id).await?;

    let (parent_id, sort_key) = match &insert.after {
        Some(after) => {
            let (parent_id, sort_key) = project::task_in_project(cx, &project_id, after).await?;
            let next =
                project::sort_key_after(cx, &project_id, parent_id.as_deref(), &sort_key).await?;
            (parent_id, next)
        }
        None => (None, project::next_sort_key(cx, &project_id).await?),
    };

    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO tasks (id, project_id, parent_id, sort_key, name, updated_at, updated_by)
         VALUES (?1, ?2, ?3, ?4, '', ?5, ?6)",
    )
    .bind(&id)
    .bind(&project_id)
    .bind(&parent_id)
    .bind(&sort_key)
    .bind(db::now())
    .bind(&user.id)
    .execute(db::pool(cx))
    .await?;

    history::record(
        cx,
        history::Entry {
            project_id: &project_id,
            task_id: Some(&id),
            task_name: "",
            action: "追加",
            field: "",
            before: "",
            after: "",
            actor: user.display(),
        },
    )
    .await?;

    respond(cx, &project, user.display(), Some(id)).await
}

#[derive(Deserialize)]
struct MoveRequest {
    action: project::Move,
}

#[route(POST "/api/projects/{project_id}/tasks/{task_id}/move")]
async fn move_task(cx: &Cx, Json(request): Json<MoveRequest>) -> Result<Json<Mutation>> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let task_id = project::path_str(cx, "task_id")?.to_owned();

    let project = authorize_edit(cx, &user.id, &project_id).await?;

    // A move that runs into the edge of the outline changes nothing, so the
    // revision must not move either — other clients are not out of date. It
    // does owe the user an explanation, though.
    let name = history::task_name(cx, &task_id).await;

    if let Some(note) = project::move_task(cx, &project_id, &task_id, request.action).await? {
        return Ok(Json(Mutation {
            grid: project::grid_data(cx, &project).await?,
            task_id: Some(task_id),
            note: Some(note),
        }));
    }

    history::record(
        cx,
        history::Entry {
            project_id: &project_id,
            task_id: Some(&task_id),
            task_name: &name,
            action: "移動",
            field: "",
            before: "",
            after: "",
            actor: user.display(),
        },
    )
    .await?;

    respond(cx, &project, user.display(), Some(task_id)).await
}

#[derive(Deserialize)]
struct PlaceRequest {
    /// The new parent, or `None` for the top level.
    parent: Option<String>,
    /// The sibling it lands after, or `None` to become the first child.
    after: Option<String>,
}

#[route(POST "/api/projects/{project_id}/tasks/{task_id}/place")]
async fn place_task(cx: &Cx, Json(request): Json<PlaceRequest>) -> Result<Json<Mutation>> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let task_id = project::path_str(cx, "task_id")?.to_owned();

    let project = authorize_edit(cx, &user.id, &project_id).await?;
    project::task_in_project(cx, &project_id, &task_id).await?;

    if let Some(parent) = &request.parent {
        project::task_in_project(cx, &project_id, parent).await?;
    }

    let refused = project::place_task(
        cx,
        &project_id,
        &task_id,
        request.parent.as_deref(),
        request.after.as_deref(),
    )
    .await?;

    if let Some(note) = refused {
        return Ok(Json(Mutation {
            grid: project::grid_data(cx, &project).await?,
            task_id: Some(task_id),
            note: Some(note),
        }));
    }

    respond(cx, &project, user.display(), Some(task_id)).await
}

#[route(DELETE "/api/projects/{project_id}/tasks/{task_id}")]
async fn delete_task(cx: &Cx) -> Result<Json<Mutation>> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let task_id = project::path_str(cx, "task_id")?.to_owned();

    let project = authorize_edit(cx, &user.id, &project_id).await?;
    project::task_in_project(cx, &project_id, &task_id).await?;

    let name = history::task_name(cx, &task_id).await;

    // Children cascade: deleting a summary row takes its subtree with it.
    sqlx::query("DELETE FROM tasks WHERE id = ?1 AND project_id = ?2")
        .bind(&task_id)
        .bind(&project_id)
        .execute(db::pool(cx))
        .await?;

    history::record(
        cx,
        history::Entry {
            project_id: &project_id,
            task_id: None,
            task_name: &name,
            action: "削除",
            field: "",
            before: &name,
            after: "",
            actor: user.display(),
        },
    )
    .await?;

    respond(cx, &project, user.display(), None).await
}

/// Applies one column write, stamping who touched the row.
async fn write_cell<T>(
    cx: &Cx,
    task_id: &str,
    user_id: &str,
    assignment: &str,
    value: T,
) -> Result<()>
where
    T: for<'q> sqlx::Encode<'q, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> + Send + 'static,
{
    // `assignment` is a fixed string chosen from a closed set above, never
    // anything a request supplies.
    let sql =
        format!("UPDATE tasks SET {assignment}, updated_at = ?2, updated_by = ?3 WHERE id = ?4");

    sqlx::query(&sql)
        .bind(value)
        .bind(db::now())
        .bind(user_id)
        .bind(task_id)
        .execute(db::pool(cx))
        .await?;

    Ok(())
}

/// Bumps the revision and hands back the recomputed grid.
/// The browser that sent the request, if it identified itself.
fn client_id(cx: &Cx) -> Option<String> {
    headers(cx)
        .get("x-fugantt-client")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

async fn respond(
    cx: &Cx,
    project: &project::Project,
    actor: &str,
    task_id: Option<String>,
) -> Result<Json<Mutation>> {
    project::bump_revision(cx, &project.id).await?;

    // Re-read so the revision in the payload is the one the write produced.
    let project = project::reload(cx, &project.id, &project.role).await?;

    live::hub(cx).publish(
        &project.id,
        live::Change {
            revision: project.revision,
            task_id: task_id.clone(),
            actor: actor.to_owned(),
            client: client_id(cx),
        },
    );

    Ok(Json(Mutation {
        grid: project::grid_data(cx, &project).await?,
        task_id,
        note: None,
    }))
}

/// Folds full-width digits and separators onto their ASCII forms.
///
/// A Japanese keyboard left in kana or full-width mode turns "2026-09-01" into
/// "２０２６－０９－０１", which is the same date typed the same way and should
/// not be refused for it.
fn normalize_width(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '０'..='９' => char::from_u32(c as u32 - '０' as u32 + '0' as u32).unwrap_or(c),
            'ａ'..='ｚ' => char::from_u32(c as u32 - 'ａ' as u32 + 'a' as u32).unwrap_or(c),
            'Ａ'..='Ｚ' => char::from_u32(c as u32 - 'Ａ' as u32 + 'A' as u32).unwrap_or(c),
            '－' | 'ー' | '−' | '‐' => '-',
            '／' => '/',
            '．' => '.',
            '％' => '%',
            '　' => ' ',
            _ => c,
        })
        .collect()
}

fn trim(value: &str) -> String {
    value
        .chars()
        .take(TEXT_LIMIT)
        .collect::<String>()
        .trim()
        .to_owned()
}

// --- live updates -----------------------------------------------------------

/// The stream of changes other people make to this project.
///
/// Only the revision travels: a client that hears one refetches, so it can
/// never apply a change it is not ready for or paint one out of order.
#[route(GET "/api/projects/{project_id}/live")]
async fn live_changes(cx: &Cx) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    project::authorize(cx, &user.id, &project_id).await?;

    let changes = live::hub(cx).subscribe(&project_id);

    let events = stream::unfold(changes, |mut changes| async move {
        loop {
            match changes.recv().await {
                Ok(change) => {
                    return Some((Event::new().event("change").json_data(&change), changes));
                }
                // Lagged: the client missed some, but a refetch catches it up
                // regardless of how many it missed.
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Ok(Sse::new(events).keep_alive(KeepAlive::new()))
}

// --- settings ---------------------------------------------------------------

#[derive(Deserialize)]
struct HolidayForm {
    date: String,
    name: Option<String>,
}

#[route(POST "/projects/{project_id}/holidays")]
async fn add_holiday(cx: &Cx, Form(form): Form<HolidayForm>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    let date: Date = form
        .date
        .trim()
        .parse()
        .map_err(|_| bad_request(l.t("日付は YYYY-MM-DD の形式で入力してください。")))?;

    // A day this project keeps that the company calendar does not have. The
    // shared list is edited in 全体の設定; here we only ever write the difference.
    sqlx::query(
        "INSERT INTO project_holidays (project_id, date, name, kind) VALUES (?1, ?2, ?3, 'add')
         ON CONFLICT (project_id, date) DO UPDATE SET name = excluded.name, kind = 'add'",
    )
    .bind(&project_id)
    .bind(date.to_string())
    .bind(trim(form.name.as_deref().unwrap_or("")))
    .execute(db::pool(cx))
    .await?;

    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=holidays#holidays"
    )))
}

#[derive(Deserialize)]
struct RemoveHoliday {
    date: String,
}

#[route(POST "/projects/{project_id}/holidays/remove")]
async fn remove_holiday(cx: &Cx, Form(form): Form<RemoveHoliday>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    let date = form.date.trim();

    // "Do not take this day off" means two things depending on the day. One the
    // project added is simply deleted. A shared holiday cannot be deleted from
    // here, so what is recorded instead is that this project works through it.
    let removed = sqlx::query("DELETE FROM project_holidays WHERE project_id = ?1 AND date = ?2")
        .bind(&project_id)
        .bind(date)
        .execute(db::pool(cx))
        .await?
        .rows_affected();

    if removed == 0 {
        sqlx::query(
            "INSERT INTO project_holidays (project_id, date, name, kind)
             VALUES (?1, ?2, '', 'skip')
             ON CONFLICT (project_id, date) DO UPDATE SET kind = 'skip'",
        )
        .bind(&project_id)
        .bind(date)
        .execute(db::pool(cx))
        .await?;
    }

    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=holidays#holidays"
    )))
}

/// Undoes "this project works through it" and takes the shared holiday back.
#[route(POST "/projects/{project_id}/holidays/restore")]
async fn restore_holiday(cx: &Cx, Form(form): Form<RemoveHoliday>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    sqlx::query(
        "DELETE FROM project_holidays WHERE project_id = ?1 AND date = ?2 AND kind = 'skip'",
    )
    .bind(&project_id)
    .bind(form.date.trim())
    .execute(db::pool(cx))
    .await?;

    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=holidays#holidays"
    )))
}

#[derive(Deserialize)]
struct LeaveForm {
    assignee: String,
    start: String,
    end: String,
    note: Option<String>,
}

/// Leave, by assignee.
///
/// Stored against the name in the 担当者 column rather than an account: plans
/// name people who have no login here, and the column is free text anyway.
#[route(POST "/projects/{project_id}/leaves")]
async fn add_leave(cx: &Cx, Form(form): Form<LeaveForm>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    let assignee = trim(&form.assignee);
    if assignee.is_empty() {
        return Err(bad_request(l.t("担当者を入力してください。")).into());
    }

    let day = |value: &str| -> Option<Date> { value.trim().parse().ok() };
    let (Some(start), Some(end)) = (day(&form.start), day(&form.end)) else {
        return Err(bad_request(l.t("日付は YYYY-MM-DD の形式で入力してください。")).into());
    };

    if end < start {
        return Err(bad_request(l.t("終了日は開始日より後にしてください。")).into());
    }

    sqlx::query(
        "INSERT INTO leaves (id, assignee, start_date, end_date, note, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&assignee)
    .bind(start.to_string())
    .bind(end.to_string())
    .bind(trim(form.note.as_deref().unwrap_or("")))
    .bind(db::now())
    .execute(db::pool(cx))
    .await?;

    bump_and_announce(cx, &project_id, user.display()).await?;

    // Leave is ordinary work rather than a setting, so this goes back to the schedule.
    Ok(see_other(&format!("/projects/{project_id}")))
}

#[derive(Deserialize)]
struct LeaveList {
    leaves: Vec<LeaveEntry>,
}

#[derive(Deserialize)]
struct LeaveEntry {
    assignee: String,
    start: String,
    end: String,
    #[serde(default)]
    note: String,
    /// `off` for a day away, `on` for a day worked regardless.
    #[serde(default)]
    kind: String,
}

/// Replaces the whole list, for the dialog on the schedule.
///
/// Leave is ordinary work — somebody says they are off next week — so it lives
/// on the screen the work is on, not in the project's settings. The dialog
/// hands back the list it edited, and this is that list.
#[route(POST "/api/projects/{project_id}/leaves")]
async fn set_leaves(cx: &Cx, Json(form): Json<LeaveList>) -> Result<Json<Mutation>> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let project = authorize_edit(cx, &user.id, &project_id).await?;

    let mut rows = Vec::new();

    for leave in &form.leaves {
        let assignee = trim(&leave.assignee);
        let (Some(start), Some(end)) = (flexible_date(&leave.start), flexible_date(&leave.end))
        else {
            return Err(bad_request(l.t("休暇の日付を確認してください。")).into());
        };

        if assignee.is_empty() {
            return Err(bad_request(l.t("担当者を選んでください。")).into());
        }

        let kind = if leave.kind == "on" { "on" } else { "off" };
        rows.push((
            assignee,
            start.min(end),
            end.max(start),
            trim(&leave.note),
            kind,
        ));
    }

    let mut tx = db::pool(cx).begin().await?;

    // There is one table of leave for the whole installation. What the dialog
    // edited was the part belonging to people on this plan, so that is the only
    // part this save may delete. Nobody else's week is collateral.
    sqlx::query(
        "DELETE FROM leaves WHERE assignee IN (
             SELECT CASE WHEN users.display_name = '' THEN users.email ELSE users.display_name END
               FROM project_members
               JOIN users ON users.id = project_members.user_id
              WHERE project_members.project_id = ?1
             UNION
             SELECT TRIM(assignee) FROM tasks
              WHERE project_id = ?1 AND TRIM(assignee) <> ''
             UNION
             SELECT name FROM project_assignees WHERE project_id = ?1
         )",
    )
    .bind(&project_id)
    .execute(&mut *tx)
    .await?;

    for (assignee, start, end, note, kind) in rows {
        sqlx::query(
            "INSERT INTO leaves
                 (id, assignee, start_date, end_date, note, kind, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&assignee)
        .bind(start.to_string())
        .bind(end.to_string())
        .bind(&note)
        .bind(kind)
        .bind(db::now())
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    respond(cx, &project, user.display(), None).await
}

#[derive(Deserialize)]
struct RemoveLeave {
    id: String,
}

#[route(POST "/projects/{project_id}/leaves/remove")]
async fn remove_leave(cx: &Cx, Form(form): Form<RemoveLeave>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    sqlx::query("DELETE FROM leaves WHERE id = ?1")
        .bind(form.id.trim())
        .execute(db::pool(cx))
        .await?;

    bump_and_announce(cx, &project_id, user.display()).await?;

    // Leave is ordinary work rather than a setting, so this goes back to the schedule.
    Ok(see_other(&format!("/projects/{project_id}")))
}

#[derive(Deserialize)]
struct StatusForm {
    name: String,
    color: String,
    percent: Option<String>,
}

#[route(POST "/projects/{project_id}/statuses")]
async fn add_status(cx: &Cx, Form(form): Form<StatusForm>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    let name = trim(&form.name);
    if name.is_empty() {
        return Err(bad_request(l.t("ステータス名を入力してください。")).into());
    }

    let color = form.color.trim();
    if !crate::domain::is_hex_colour(color) {
        return Err(bad_request(l.t("色は #rrggbb の形式で指定してください。")).into());
    }

    // Blank means "this state says nothing about progress", which is the honest
    // answer for 実施中 and the reason the column stays hand-entered.
    let percent = match form.percent.as_deref().map(str::trim).unwrap_or("") {
        "" => None,
        text => Some(
            normalize_width(text)
                .trim_end_matches('%')
                .parse::<i64>()
                .ok()
                .filter(|percent| (0..=100).contains(percent))
                .ok_or_else(|| bad_request(l.t("進捗は 0〜100 で指定してください。")))?,
        ),
    };

    // The list is only stored once it is touched, so a project that starts from
    // the built-in one keeps every state it was already using.
    let existing = project::statuses(cx, &project_id).await?;
    let mut tx = db::pool(cx).begin().await?;

    for (position, status) in existing.iter().enumerate() {
        sqlx::query(
            "INSERT INTO project_statuses (project_id, position, name, color, percent)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (project_id, name) DO NOTHING",
        )
        .bind(&project_id)
        .bind(position as i64)
        .bind(&status.name)
        .bind(&status.color)
        .bind(status.percent)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "INSERT INTO project_statuses (project_id, position, name, color, percent)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (project_id, name) DO UPDATE
            SET color = excluded.color, percent = excluded.percent",
    )
    .bind(&project_id)
    .bind(existing.len() as i64)
    .bind(&name)
    .bind(color)
    .bind(percent)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=statuses#statuses"
    )))
}

#[derive(Deserialize)]
struct MoveStatus {
    name: String,
    direction: String,
}

/// Swaps a status with its neighbour. The menu offers them in this order, so
/// the order is worth being able to set.
#[route(POST "/projects/{project_id}/statuses/move")]
async fn move_status(cx: &Cx, Form(form): Form<MoveStatus>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    let mut statuses = project::statuses(cx, &project_id).await?;

    if let Some(at) = statuses.iter().position(|status| status.name == form.name) {
        let to = if form.direction == "up" {
            at.saturating_sub(1)
        } else {
            (at + 1).min(statuses.len() - 1)
        };

        if at != to {
            statuses.swap(at, to);

            // Written in full: the list may still be the built-in one, which
            // has never been stored at all.
            let mut tx = db::pool(cx).begin().await?;

            sqlx::query("DELETE FROM project_statuses WHERE project_id = ?1")
                .bind(&project_id)
                .execute(&mut *tx)
                .await?;

            for (position, status) in statuses.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO project_statuses (project_id, position, name, color, percent)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                )
                .bind(&project_id)
                .bind(position as i64)
                .bind(&status.name)
                .bind(&status.color)
                .bind(status.percent)
                .execute(&mut *tx)
                .await?;
            }

            tx.commit().await?;
            bump_and_announce(cx, &project_id, user.display()).await?;
        }
    }

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=statuses#statuses"
    )))
}

#[derive(Deserialize)]
struct RemoveStatus {
    name: String,
}

#[route(POST "/projects/{project_id}/statuses/remove")]
async fn remove_status(cx: &Cx, Form(form): Form<RemoveStatus>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    // Removing the last one would leave the column with no menu at all.
    let existing = project::statuses(cx, &project_id).await?;
    if existing.len() <= 1 {
        return Err(bad_request(l.t("ステータスは1つ以上必要です。")).into());
    }

    let mut tx = db::pool(cx).begin().await?;

    for (position, status) in existing.iter().enumerate() {
        sqlx::query(
            "INSERT INTO project_statuses (project_id, position, name, color, percent)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (project_id, name) DO NOTHING",
        )
        .bind(&project_id)
        .bind(position as i64)
        .bind(&status.name)
        .bind(&status.color)
        .bind(status.percent)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query("DELETE FROM project_statuses WHERE project_id = ?1 AND name = ?2")
        .bind(&project_id)
        .bind(form.name.trim())
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=statuses#statuses"
    )))
}

/// The view form arrives as a map rather than a struct: a url-encoded body
/// cannot carry a repeated key into a `Vec`, so each checkbox gets its own name
/// (`column_status`) and is present only when it is ticked.
#[route(POST "/projects/{project_id}/view")]
async fn set_view(
    cx: &Cx,
    Form(form): Form<std::collections::HashMap<String, String>>,
) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    // A marker that the four switches below have been written at least once,
    // so the old single `workdays_only` stops standing in for them.
    project::set_setting(cx, &project_id, "counting", "1").await?;

    for key in [
        "skip_monday",
        "skip_tuesday",
        "skip_wednesday",
        "skip_thursday",
        "skip_friday",
        "skip_saturday",
        "skip_sunday",
        "skip_holidays",
        "skip_leave",
    ] {
        let on = if form.contains_key(key) { "1" } else { "" };
        project::set_setting(cx, &project_id, key, on).await?;
    }

    project::set_setting(
        cx,
        &project_id,
        "quarters",
        if form.contains_key("quarters") {
            "1"
        } else {
            "0"
        },
    )
    .await?;

    project::set_setting(
        cx,
        &project_id,
        "japanese_era",
        if form.contains_key("japanese_era") {
            "1"
        } else {
            ""
        },
    )
    .await?;

    // Anything outside 1–12 is ignored. The setting only affects the display,
    // but an impossible value breaks that display.
    if let Some(month) = form.get("fiscal_year_start")
        && (1..=12).contains(&month.parse::<u32>().unwrap_or(0))
    {
        project::set_setting(cx, &project_id, "fiscal_year_start", month).await?;
    }

    // How progress is set. `status` links it to the two statuses that leave no room for
    // an opinion; anything else means it stays hand-entered.
    project::set_setting(
        cx,
        &project_id,
        "progress_mode",
        if form
            .get("progress_mode")
            .is_some_and(|mode| mode == "status")
        {
            "status"
        } else {
            ""
        },
    )
    .await?;

    if let Some(count) = form.get("frozen_columns")
        && count.parse::<usize>().is_ok_and(|count| count <= 8)
    {
        project::set_setting(cx, &project_id, "frozen_columns", count).await?;
    }

    if let Some(width) = form.get("day_width")
        && (8..=48).contains(&width.parse::<u32>().unwrap_or(0))
    {
        project::set_setting(cx, &project_id, "day_width", width).await?;
    }

    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=view#view"
    )))
}

/// The built-in columns a project may turn off.
/// Every built-in column, in the order they are declared.
pub const COLUMN_KEYS: [&str; 15] = [
    "name",
    "start",
    "end",
    "actual_start",
    "actual_end",
    "days",
    "actual_days",
    "start_variance",
    "end_variance",
    "targets",
    "progress",
    "status",
    "assignee",
    "note",
    "waits",
];

pub const OPTIONAL_COLUMNS: [&str; 14] = [
    "start",
    "end",
    "actual_start",
    "actual_end",
    "days",
    "actual_days",
    "start_variance",
    "end_variance",
    "targets",
    "progress",
    "status",
    "assignee",
    "note",
    "waits",
];

#[derive(Deserialize)]
struct MemoForm {
    memo: String,
}

/// Free notes about the project — decisions, links, whatever the plan itself
/// cannot hold.
#[route(POST "/projects/{project_id}/memo")]
async fn set_memo(cx: &Cx, Form(form): Form<MemoForm>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    // Long enough for real notes, bounded so one paste cannot fill the page.
    let memo: String = form.memo.chars().take(20_000).collect();

    project::set_setting(cx, &project_id, "memo", memo.trim()).await?;

    Ok(see_other(&format!("/projects/{project_id}?memo=1")))
}

#[derive(Deserialize)]
struct FieldForm {
    label: String,
    kind: String,
}

#[route(POST "/projects/{project_id}/fields")]
async fn add_field(cx: &Cx, Form(form): Form<FieldForm>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    let label = trim(&form.label);
    if label.is_empty() {
        return Err(bad_request(l.t("項目名を入力してください。")).into());
    }

    // `suggest` is a free text box with the master list offered as candidates —
    // the answer is usually one of a few, but not always.
    if !FIELD_KINDS.contains(&form.kind.as_str()) {
        return Err(bad_request(l.t("項目の種類が不正です。")).into());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let sort_key = project::next_key(
        cx,
        "SELECT sort_key FROM project_fields WHERE project_id = ?1 ORDER BY sort_key DESC LIMIT 1",
        &project_id,
    )
    .await?;

    let mut tx = db::pool(cx).begin().await?;

    sqlx::query(
        "INSERT INTO project_fields (id, project_id, label, kind, sort_key)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&id)
    .bind(&project_id)
    .bind(&label)
    .bind(&form.kind)
    .bind(&sort_key)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=fields#fields"
    )))
}

#[derive(Deserialize)]
struct RemoveField {
    field_id: String,
}

/// The kinds a project's own column can be.
const FIELD_KINDS: [&str; 5] = ["text", "number", "select", "date", "suggest"];

#[derive(Deserialize)]
struct RenameField {
    field_id: String,
    label: String,
}

/// Changes a field's name.
///
/// The values are keyed by the field's id, so a new name is only a new name —
/// nothing entered under the old one moves or is lost. Without this, correcting
/// a typo meant deleting the column, which takes every value with it.
#[route(POST "/projects/{project_id}/fields/rename")]
async fn rename_field(cx: &Cx, Form(form): Form<RenameField>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;
    project::field_in_project(cx, &project_id, form.field_id.trim()).await?;

    let label = trim(&form.label);
    if label.is_empty() {
        return Err(bad_request(l.t("項目名を入力してください。")).into());
    }

    sqlx::query("UPDATE project_fields SET label = ?3 WHERE id = ?1 AND project_id = ?2")
        .bind(form.field_id.trim())
        .bind(&project_id)
        .bind(&label)
        .execute(db::pool(cx))
        .await?;

    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=fields#fields"
    )))
}

#[derive(Deserialize)]
struct FieldKind {
    field_id: String,
    kind: String,
}

/// Changes a field's kind, while the column is still empty.
///
/// A column of dates read as numbers keeps its dates and shows nothing anyone
/// can use, so this is refused the moment a single task has written a value.
#[route(POST "/projects/{project_id}/fields/kind")]
async fn set_field_kind(cx: &Cx, Form(form): Form<FieldKind>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;
    project::field_in_project(cx, &project_id, form.field_id.trim()).await?;

    if !FIELD_KINDS.contains(&form.kind.as_str()) {
        return Err(bad_request(l.t("項目の種類が不正です。")).into());
    }

    let (used,) = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM task_field_values WHERE field_id = ?1 AND TRIM(value) <> ''",
    )
    .bind(form.field_id.trim())
    .fetch_one(db::pool(cx))
    .await?;

    if used > 0 {
        return Err(bad_request(
            l.t("入力済みの項目は種類を変えられません。内容を空にしてからにしてください。"),
        )
        .into());
    }

    sqlx::query("UPDATE project_fields SET kind = ?3 WHERE id = ?1 AND project_id = ?2")
        .bind(form.field_id.trim())
        .bind(&project_id)
        .bind(form.kind.trim())
        .execute(db::pool(cx))
        .await?;

    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=fields#fields"
    )))
}

#[route(POST "/projects/{project_id}/fields/remove")]
async fn remove_field(cx: &Cx, Form(form): Form<RemoveField>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    // Values and options cascade: removing a column removes what was in it.
    sqlx::query("DELETE FROM project_fields WHERE id = ?1 AND project_id = ?2")
        .bind(&form.field_id)
        .bind(&project_id)
        .execute(db::pool(cx))
        .await?;

    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=fields#fields"
    )))
}

/// The columns: which are shown, how wide, and in what order.
///
/// Their own form rather than part of the view one, because the ↑↓ buttons have
/// to live beside the fields they reorder — and a form cannot nest in a form.
/// Pressing ↑ submits the same form, so the widths typed alongside are saved
/// rather than discarded.
#[route(POST "/projects/{project_id}/columns")]
async fn set_columns(
    cx: &Cx,
    Form(form): Form<std::collections::HashMap<String, String>>,
) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let project = authorize_edit(cx, &user.id, &project_id).await?;

    // The name column carries the outline, so it is not optional.
    let hidden: Vec<&str> = OPTIONAL_COLUMNS
        .iter()
        .copied()
        .filter(|key| !form.contains_key(&format!("column_{key}")))
        .collect();

    project::set_setting(cx, &project_id, "hidden_columns", &hidden.join(" ")).await?;

    // Widths arrive one at a time as `width_<column>`. An empty one means the
    // column goes back to sizing itself.
    let widths: Vec<String> = form
        .iter()
        .filter_map(|(key, value)| {
            let key = key.strip_prefix("width_")?;
            let width: u32 = value.trim().parse().ok()?;

            (40..=600)
                .contains(&width)
                .then(|| format!("{key}:{width}"))
        })
        .collect();

    project::set_setting(cx, &project_id, "column_widths", &widths.join(" ")).await?;

    // `move` arrives as `up:<column>`, and only when the button was pressed.
    if let Some((direction, key)) = form.get("move").and_then(|value| value.split_once(':')) {
        let data = project::grid_data(cx, &project).await?;
        let mut order = project::column_order(&data);

        if let Some(at) = order.iter().position(|column| column == key) {
            // The name column carries the outline and the indent; it stays
            // first, and nothing swaps into its place.
            let to = if direction == "up" {
                at.saturating_sub(1)
            } else {
                (at + 1).min(order.len() - 1)
            };

            if at != to && at != 0 && to != 0 {
                order.swap(at, to);
                project::set_setting(cx, &project_id, "column_order", &order.join(" ")).await?;
            }
        }
    }

    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=columns#columns"
    )))
}

#[derive(Deserialize)]
struct AssigneeForm {
    name: String,
}

/// Puts a name on this plan's list, whether or not it belongs to an account.
///
/// The colour is not decided here. One person in a different colour per project
/// is unreadable across several, so colours live in the shared list.
#[route(POST "/projects/{project_id}/assignees")]
async fn set_assignee(cx: &Cx, Form(form): Form<AssigneeForm>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    let name = trim(&form.name);
    if name.is_empty() {
        return Err(bad_request(l.t("担当者名を入力してください。")).into());
    }

    sqlx::query(
        "INSERT INTO project_assignees (project_id, name) VALUES (?1, ?2)
         ON CONFLICT (project_id, name) DO NOTHING",
    )
    .bind(&project_id)
    .bind(&name)
    .execute(db::pool(cx))
    .await?;

    // A name new to the shared list gets a row to hold a colour. The colour
    // itself is chosen in the installation settings.
    sqlx::query("INSERT INTO assignees (name) VALUES (?1) ON CONFLICT (name) DO NOTHING")
        .bind(&name)
        .execute(db::pool(cx))
        .await?;

    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=assignees#assignees"
    )))
}

#[derive(Deserialize)]
struct RemoveAssignee {
    name: String,
}

/// Takes a name off this plan's list. A name that a member or a task still
/// carries stays on the list — it is only the entry here that goes.
#[route(POST "/projects/{project_id}/assignees/remove")]
async fn remove_assignee(cx: &Cx, Form(form): Form<RemoveAssignee>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    sqlx::query("DELETE FROM project_assignees WHERE project_id = ?1 AND name = ?2")
        .bind(&project_id)
        .bind(form.name.trim())
        .execute(db::pool(cx))
        .await?;

    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=assignees#assignees"
    )))
}

#[derive(Deserialize)]
struct OptionForm {
    field_id: String,
    value: String,
    color: Option<String>,
    background: Option<String>,
}

/// Adds an entry to a master list, or recolours one that is already there.
#[route(POST "/projects/{project_id}/fields/options")]
async fn add_option(cx: &Cx, Form(form): Form<OptionForm>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    let value = trim(&form.value);
    if value.is_empty() {
        return Err(bad_request(l.t("選択肢を入力してください。")).into());
    }

    let colour = |value: Option<&str>| -> Result<String> {
        let value = value.unwrap_or("").trim();

        if value.is_empty() {
            return Ok(String::new());
        }
        if !crate::domain::is_hex_colour(value) {
            return Err(bad_request(l.t("色は #rrggbb の形式で指定してください。")).into());
        }

        Ok(value.to_owned())
    };

    let color = colour(form.color.as_deref())?;
    let background = colour(form.background.as_deref())?;

    // Scoped to the project, so one project cannot write into another's list.
    project::field_in_project(cx, &project_id, &form.field_id).await?;

    let key = project::next_key(
        cx,
        "SELECT sort_key FROM project_field_options WHERE field_id = ?1 ORDER BY sort_key DESC LIMIT 1",
        &form.field_id,
    )
    .await?;

    sqlx::query(
        "INSERT INTO project_field_options (field_id, value, sort_key, color, background)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (field_id, value) DO UPDATE
            SET color = excluded.color, background = excluded.background",
    )
    .bind(&form.field_id)
    .bind(&value)
    .bind(&key)
    .bind(&color)
    .bind(&background)
    .execute(db::pool(cx))
    .await?;

    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=fields#fields"
    )))
}

#[derive(Deserialize)]
struct MoveOption {
    field_id: String,
    value: String,
    direction: String,
}

/// Swaps an entry with its neighbour, which is what ↑ and ↓ mean.
#[route(POST "/projects/{project_id}/fields/options/move")]
async fn move_option(cx: &Cx, Form(form): Form<MoveOption>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;
    project::field_in_project(cx, &project_id, &form.field_id).await?;

    let up = form.direction == "up";

    // The neighbour on the side we are moving towards; nothing there means the
    // entry is already at the end, and the button was disabled anyway.
    let statement = if up {
        "SELECT value, sort_key FROM project_field_options
          WHERE field_id = ?1 AND sort_key < (
              SELECT sort_key FROM project_field_options WHERE field_id = ?1 AND value = ?2
          )
          ORDER BY sort_key DESC LIMIT 1"
    } else {
        "SELECT value, sort_key FROM project_field_options
          WHERE field_id = ?1 AND sort_key > (
              SELECT sort_key FROM project_field_options WHERE field_id = ?1 AND value = ?2
          )
          ORDER BY sort_key LIMIT 1"
    };

    let neighbour = sqlx::query_as::<_, (String, String)>(statement)
        .bind(&form.field_id)
        .bind(form.value.trim())
        .fetch_optional(db::pool(cx))
        .await?;

    if let Some((other, other_key)) = neighbour {
        let (mine,) = sqlx::query_as::<_, (String,)>(
            "SELECT sort_key FROM project_field_options WHERE field_id = ?1 AND value = ?2",
        )
        .bind(&form.field_id)
        .bind(form.value.trim())
        .fetch_one(db::pool(cx))
        .await?;

        let mut tx = db::pool(cx).begin().await?;

        for (value, key) in [(form.value.trim(), other_key), (other.as_str(), mine)] {
            sqlx::query(
                "UPDATE project_field_options SET sort_key = ?3
                  WHERE field_id = ?1 AND value = ?2",
            )
            .bind(&form.field_id)
            .bind(value)
            .bind(key)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        bump_and_announce(cx, &project_id, user.display()).await?;
    }

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=fields#fields"
    )))
}

#[derive(Deserialize)]
struct RemoveOption {
    field_id: String,
    value: String,
}

#[route(POST "/projects/{project_id}/fields/options/remove")]
async fn remove_option(cx: &Cx, Form(form): Form<RemoveOption>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;
    project::field_in_project(cx, &project_id, &form.field_id).await?;

    // What is already entered on a task stays: the list is what may be picked
    // from now on, not a claim about what was picked before.
    sqlx::query("DELETE FROM project_field_options WHERE field_id = ?1 AND value = ?2")
        .bind(&form.field_id)
        .bind(form.value.trim())
        .execute(db::pool(cx))
        .await?;

    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=fields#fields"
    )))
}

#[derive(Deserialize)]
struct ColourForm {
    color_bar: String,
    color_done: String,
    color_actual: String,
    color_summary: String,
    color_late: String,
    color_saturday: String,
    color_sunday: String,
    color_holiday: String,
    color_leave: String,
    color_wait: String,
    color_today: String,
}

#[route(POST "/projects/{project_id}/colors")]
async fn set_colors(cx: &Cx, Form(form): Form<ColourForm>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    for (key, value) in [
        ("color_bar", &form.color_bar),
        ("color_done", &form.color_done),
        ("color_actual", &form.color_actual),
        ("color_summary", &form.color_summary),
        ("color_late", &form.color_late),
        ("color_saturday", &form.color_saturday),
        ("color_sunday", &form.color_sunday),
        ("color_holiday", &form.color_holiday),
        ("color_leave", &form.color_leave),
        ("color_wait", &form.color_wait),
        ("color_today", &form.color_today),
    ] {
        let value = value.trim();

        // A colour goes straight into a stylesheet, so anything that is not a
        // hex triple is refused rather than escaped and hoped for.
        if !value.is_empty() && !crate::domain::is_hex_colour(value) {
            return Err(bad_request(l.t("色は #rrggbb の形式で指定してください。")).into());
        }

        project::set_setting(cx, &project_id, key, value).await?;
    }

    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=colours#colours"
    )))
}

#[derive(Deserialize)]
struct NewToken {
    name: String,
    role: String,
}

/// Issues a token, and shows it once.
///
/// The value goes back in the URL because that is the only way this page can
/// say it out loud without storing it anywhere: the next visit has no copy.
#[route(POST "/projects/{project_id}/tokens")]
async fn create_token(cx: &Cx, Form(form): Form<NewToken>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let project = project::authorize(cx, &user.id, &project_id).await?;
    let l = crate::i18n::lang(cx).await;

    // Handing out a key to the plan is the owner's call.
    if !project.is_owner() {
        return Err(forbidden().into());
    }

    let role = if form.role == "editor" {
        "editor"
    } else {
        "viewer"
    };
    let (token, hash) = crate::tokens::generate();

    sqlx::query(
        "INSERT INTO api_tokens (id, project_id, name, role, token_hash, created_at, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&project_id)
    .bind(trim(&form.name))
    .bind(role)
    .bind(&hash[..])
    .bind(db::now())
    .bind(&user.id)
    .execute(db::pool(cx))
    .await?;

    let _ = l;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=tokens&issued={token}#tokens"
    )))
}

#[derive(Deserialize)]
struct RemoveToken {
    id: String,
}

#[route(POST "/projects/{project_id}/tokens/remove")]
async fn remove_token(cx: &Cx, Form(form): Form<RemoveToken>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let project = project::authorize(cx, &user.id, &project_id).await?;

    if !project.is_owner() {
        return Err(forbidden().into());
    }

    sqlx::query("DELETE FROM api_tokens WHERE id = ?1 AND project_id = ?2")
        .bind(form.id.trim())
        .bind(&project_id)
        .execute(db::pool(cx))
        .await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=tokens#tokens"
    )))
}

#[derive(Deserialize)]
struct MemberForm {
    email: String,
    role: String,
}

/// Adds or re-roles a member. Only an owner may hand out access.
#[route(POST "/projects/{project_id}/members")]
async fn add_member(cx: &Cx, Form(form): Form<MemberForm>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let project = project::authorize(cx, &user.id, &project_id).await?;

    if !project.is_owner() {
        return Err(forbidden().into());
    }

    if !["owner", "editor", "viewer"].contains(&form.role.as_str()) {
        return Err(bad_request(l.t("権限が不正です。")).into());
    }

    let email = form.email.trim().to_lowercase();

    let member = sqlx::query_as::<_, (String,)>("SELECT id FROM users WHERE email = ?1")
        .bind(&email)
        .fetch_optional(db::pool(cx))
        .await?
        .ok_or_else(|| bad_request(l.t("そのメールアドレスの利用者が見つかりません。")))?;

    sqlx::query(
        "INSERT INTO project_members (project_id, user_id, role) VALUES (?1, ?2, ?3)
         ON CONFLICT (project_id, user_id) DO UPDATE SET role = excluded.role",
    )
    .bind(&project_id)
    .bind(&member.0)
    .bind(&form.role)
    .execute(db::pool(cx))
    .await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=members#members"
    )))
}

#[derive(Deserialize)]
struct RemoveMember {
    user_id: String,
}

#[route(POST "/projects/{project_id}/members/remove")]
async fn remove_member(cx: &Cx, Form(form): Form<RemoveMember>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let project = project::authorize(cx, &user.id, &project_id).await?;

    if !project.is_owner() {
        return Err(forbidden().into());
    }

    // Removing the last owner would leave the project unmanageable.
    let (owners,) = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM project_members WHERE project_id = ?1 AND role = 'owner'",
    )
    .bind(&project_id)
    .fetch_one(db::pool(cx))
    .await?;

    if owners <= 1 && form.user_id == user.id {
        return Err(bad_request(l.t("最後の管理者は外せません。")).into());
    }

    sqlx::query("DELETE FROM project_members WHERE project_id = ?1 AND user_id = ?2")
        .bind(&project_id)
        .bind(&form.user_id)
        .execute(db::pool(cx))
        .await?;

    Ok(see_other(&format!(
        "/projects/{project_id}/settings?open=members#members"
    )))
}

/// A settings change moves the chart for everyone, so it counts as a revision.
async fn bump_and_announce(cx: &Cx, project_id: &str, actor: &str) -> Result<()> {
    project::bump_revision(cx, project_id).await?;

    let project = project::reload(cx, project_id, "owner").await?;

    live::hub(cx).publish(
        project_id,
        live::Change {
            revision: project.revision,
            task_id: None,
            actor: actor.to_owned(),
            client: None,
        },
    );

    Ok(())
}

// --- export and import ------------------------------------------------------

/// A response body served as a file download.
struct Download {
    filename: String,
    content_type: &'static str,
    body: Vec<u8>,
}

impl IntoResponse for Download {
    fn into_response(self, _cx: &Cx) -> Result<Response> {
        Ok(Response::builder()
            .header("Content-Type", self.content_type)
            // The name is percent-encoded so a Japanese project title survives
            // the header, which is ASCII-only.
            .header(
                "Content-Disposition",
                format!(
                    "attachment; filename*=UTF-8''{}",
                    percent_encode(&self.filename)
                ),
            )
            .body(Body::from(self.body))?)
    }
}

/// Encodes everything outside the unreserved set, as RFC 5987 requires.
fn percent_encode(text: &str) -> String {
    let mut out = String::new();

    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }

    out
}

#[route(GET "/projects/{project_id}/export.xlsx")]
async fn export_xlsx(cx: &Cx) -> Result<Download> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?;
    let project = project::authorize(cx, &user.id, project_id).await?;
    let data = project::grid_data(cx, &project).await?;

    Ok(Download {
        filename: format!("{}.xlsx", project.name),
        content_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        body: xlsx::write(&project.name, &data, crate::i18n::lang(cx).await)?,
    })
}

#[route(GET "/projects/{project_id}/export.json")]
async fn export_json(cx: &Cx) -> Result<Download> {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?;
    let project = project::authorize(cx, &user.id, project_id).await?;
    let data = project::grid_data(cx, &project).await?;

    let extras = match wants_settings(cx) {
        true => Some(project::export_extras(cx, &project.id).await?),
        false => None,
    };

    Ok(Download {
        filename: format!("{}.json", project.name),
        content_type: "application/json; charset=utf-8",
        body: json::write(&project.name, &data, extras).into_bytes(),
    })
}

/// Replaces the project's tasks with the contents of a JSON document.
#[route(POST "/projects/{project_id}/import.json")]
async fn import_json(cx: &Cx, mut form: Multipart) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    authorize_edit(cx, &user.id, &project_id).await?;

    let mut text = String::new();

    while let Some(field) = form.next_field().await? {
        if field.name() == Some("document") {
            text = String::from_utf8_lossy(&field.bytes().await?).into_owned();
        }
    }

    if text.trim().is_empty() {
        return Err(bad_request(l.t("ファイルを選んでください。")).into());
    }

    let document = json::read(&text).map_err(bad_request)?;
    let count = document.tasks.len();

    project::import_project(cx, &project_id, &user.id, &document).await?;

    record_import(cx, &project_id, user.display(), count).await?;

    bump_and_announce(cx, &project_id, user.display()).await?;

    Ok(see_other(&format!(
        "/projects/{project_id}?imported={count}"
    )))
}

/// Refuses a date that would put a range back to front.
///
/// Dragging a bar writes both ends at once and is checked there; typing writes
/// one, and the other one is already in the row. Without this, a plan can be
/// left ending before it starts, and every day count downstream is nonsense.
async fn check_order(cx: &Cx, task_id: &str, field: Field, date: Option<&str>) -> Result<()> {
    // Clearing a date can never invert a range.
    let Some(date) = date else {
        return Ok(());
    };

    let (column, is_start, message) = match field {
        Field::Start => ("end_date", true, "予定開始は予定終了より後にできません。"),
        Field::End => (
            "start_date",
            false,
            "予定終了は予定開始より前にできません。",
        ),
        Field::ActualStart => ("actual_end", true, "実施開始は実施終了より後にできません。"),
        Field::ActualEnd => (
            "actual_start",
            false,
            "実施終了は実施開始より前にできません。",
        ),
        _ => return Ok(()),
    };

    let other = sqlx::query_as::<_, (Option<String>,)>(&format!(
        "SELECT {column} FROM tasks WHERE id = ?1"
    ))
    .bind(task_id)
    .fetch_optional(db::pool(cx))
    .await?
    .and_then(|(value,)| value);

    let Some(other) = other else {
        return Ok(());
    };

    let inverted = if is_start {
        date > other.as_str()
    } else {
        date < other.as_str()
    };

    if inverted {
        return Err(bad_request(message).into());
    }

    Ok(())
}

/// Reads a cell of 予定進捗 into the stored form: `YYYY-MM-DD/PERCENT` a line.
///
/// Written the way a person would say it — `8/20 30%, 8/28 100%` — in either
/// width, with the percent sign optional. Each line is one promise: by this
/// date, this much. Nothing is read into the gap between two of them.
fn parse_targets(value: &str, l: crate::i18n::Lang) -> Result<String> {
    let text = normalize_width(value);
    let mut stored: Vec<(Date, i64)> = Vec::new();

    for part in text.split(['\n', ',', '、']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let (date, percent) = part.rsplit_once([' ', '/', '\t']).ok_or_else(|| {
            bad_request(l.t("予定進捗は「8/20 30%」のように日付と％で入力してください。"))
        })?;

        let date = wait_date(date.trim(), l)?;
        let percent: i64 = percent
            .trim()
            .trim_end_matches(['%', '％'])
            .trim()
            .parse()
            .map_err(|_| {
                bad_request(l.t("予定進捗は「8/20 30%」のように日付と％で入力してください。"))
            })?;

        if !(0..=100).contains(&percent) {
            return Err(bad_request(l.t("進捗は0〜100で入力してください。")).into());
        }

        // The same date twice is one promise revised, not two.
        stored.retain(|(had, _)| *had != date);
        stored.push((date, percent));
    }

    stored.sort_by_key(|(date, _)| *date);

    Ok(stored
        .iter()
        .map(|(date, percent)| format!("{date}/{percent}"))
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Reads a cell of waiting periods into the stored form.
///
/// People write ranges every which way — `8/17〜8/21`, `2026-08-17 - 2026-08-21`
/// — and in whichever width the IME was in. A range with no end (`9/1〜`) is one
/// that has not finished: the days keep counting until it does. Anything after
/// the range is the reason, which is a note that happens to be worth counting.
fn parse_waits(value: &str, l: crate::i18n::Lang) -> Result<String> {
    let text = normalize_width(value);
    let mut stored: Vec<String> = Vec::new();

    for part in text.split(['\n', ',', '、']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let (range, reason) = split_reason(part);
        let range = range.replace(' ', "");

        let (from, to) = range
            .split_once(['〜', '~', '～'])
            .or_else(|| range.split_once(" - "))
            .or_else(|| split_dash_range(&range))
            .ok_or_else(|| {
                bad_request(l.t("待ちは「8/17〜8/21」のように範囲で入力してください。"))
            })?;

        let from = wait_date(from, l)?;
        let to = to.trim();

        // No end written means it is still waiting.
        if to.is_empty() {
            stored.push(with_reason(&format!("{from}/"), &reason));
            continue;
        }

        let to = wait_date(to, l)?;
        let (from, to) = if to < from { (to, from) } else { (from, to) };

        stored.push(with_reason(&format!("{from}/{to}"), &reason));
    }

    Ok(stored.join("\n"))
}

/// Splits `8/17〜8/21 他部署` into its range and its reason.
///
/// Token by token: the range is however many words at the front are made only
/// of date characters, and everything after them is the reason. That reads
/// `8/17 〜 8/21 他部署` and `8/17〜8/21 3社` the same way a person does.
fn split_reason(part: &str) -> (String, String) {
    let is_date_ish = |text: &str| {
        !text.is_empty()
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || "-/.〜~～年月日".contains(c))
    };

    let tokens: Vec<&str> = part.split_whitespace().collect();
    let range = tokens
        .iter()
        .take_while(|token| is_date_ish(token))
        .count()
        .max(1);

    (tokens[..range].join(" "), tokens[range..].join(" "))
}

fn with_reason(range: &str, reason: &str) -> String {
    if reason.is_empty() {
        range.to_owned()
    } else {
        // Newline separated above, so a colon is free to mark the reason.
        format!("{range}:{}", reason.replace([':', '\n'], " ").trim())
    }
}

/// `2026-08-17 - 2026-08-21`, where the separator is the same character the
/// dates themselves use. Splitting on the dash surrounded by spaces is the only
/// reading that cannot be confused with the date's own dashes.
fn split_dash_range(part: &str) -> Option<(&str, &str)> {
    part.split_once(" - ")
        .or_else(|| part.split_once('/').filter(|(from, _)| from.contains('-')))
}

/// A day inside a waiting range, read the same way as any other date cell.
fn wait_date(text: &str, l: crate::i18n::Lang) -> Result<Date> {
    flexible_date(text)
        .ok_or_else(|| bad_request(l.t("待ちの日付は「8/17」か「2026-08-17」の形式です。")).into())
}

/// An empty cell clears the date; anything else must be a real calendar day.
fn parse_date(value: &str, l: crate::i18n::Lang) -> Result<Option<String>> {
    let value = normalize_width(value.trim());
    let value = value.trim_end_matches('%').trim();

    if value.is_empty() {
        return Ok(None);
    }

    let date = flexible_date(value).ok_or_else(|| {
        bad_request(l.t("日付は 20260805・8/5・2026-08-05 のように入力してください。"))
    })?;

    Ok(Some(date.to_string()))
}

/// A date, however somebody typed it.
///
/// Nobody reaches for the hyphens: on a numeric keypad `20260805` and `0805`
/// are the fast ways to say a day, and `8/5` is how it gets written by hand.
/// All of them mean a day, so all of them are accepted.
pub fn flexible_date(value: &str) -> Option<Date> {
    // 年, 月 and 日 read as separators; a trailing 日 is only punctuation.
    let value = normalize_width(value)
        .trim()
        .replace(['/', '.', '年', '月'], "-")
        .replace('日', "");
    let value = value.trim_end_matches('-').to_owned();
    let year = jiff::Zoned::now().date().year();

    // Bare digits: eight of them are a whole date, four are a day this year.
    if value.chars().all(|c| c.is_ascii_digit()) {
        return match value.len() {
            8 => format!("{}-{}-{}", &value[..4], &value[4..6], &value[6..])
                .parse()
                .ok(),
            4 => format!("{year}-{}-{}", &value[..2], &value[2..])
                .parse()
                .ok(),
            _ => None,
        };
    }

    let parts: Vec<&str> = value.split('-').filter(|part| !part.is_empty()).collect();

    let text = match parts.as_slice() {
        [month, day] => format!("{year}-{month:0>2}-{day:0>2}"),
        [year, month, day] => format!("{year:0>4}-{month:0>2}-{day:0>2}"),
        _ => return None,
    };

    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 予定進捗 is a date and a percentage, however it is written.
    #[test]
    fn a_checkpoint_reads_its_date_and_its_percentage() {
        let year = jiff::Zoned::now().date().year();
        let ja = crate::i18n::Lang::Ja;

        assert_eq!(
            parse_targets("2026-08-20 30%", ja).unwrap(),
            "2026-08-20/30"
        );
        // The percent sign is optional, and either width will do.
        assert_eq!(
            parse_targets("8/20　30％", ja).unwrap(),
            format!("{year}-08-20/30")
        );
        // Several at once, kept in date order whatever order they arrive in.
        assert_eq!(
            parse_targets("2026-08-28 100%, 2026-08-20 30%", ja).unwrap(),
            "2026-08-20/30\n2026-08-28/100"
        );
        // The same date twice is one promise revised, not two.
        assert_eq!(
            parse_targets("2026-08-20 30%\n2026-08-20 50%", ja).unwrap(),
            "2026-08-20/50"
        );

        assert!(parse_targets("2026-08-20", ja).is_err());
        assert!(parse_targets("2026-08-20 いつか", ja).is_err());
        assert!(parse_targets("2026-08-20 120%", ja).is_err());
    }

    /// A wait is from when, to when, and why. No end means it is still waiting.
    #[test]
    fn a_wait_reads_its_range_and_its_reason() {
        let year = jiff::Zoned::now().date().year();

        assert_eq!(
            parse_waits("2026-08-17〜2026-08-21", crate::i18n::Lang::Ja).unwrap(),
            "2026-08-17/2026-08-21"
        );
        assert_eq!(
            parse_waits("2026-08-17〜2026-08-21 他部署", crate::i18n::Lang::Ja).unwrap(),
            "2026-08-17/2026-08-21:他部署"
        );
        // No end: still open.
        assert_eq!(
            parse_waits("2026-09-01〜 顧客", crate::i18n::Lang::Ja).unwrap(),
            "2026-09-01/:顧客"
        );
        // Everything after the dates is the reason, spaces and all.
        assert_eq!(
            parse_waits("8/17〜8/21 他部署 承認待ち", crate::i18n::Lang::Ja).unwrap(),
            format!("{year}-08-17/{year}-08-21:他部署 承認待ち")
        );
        // Several at once.
        assert_eq!(
            parse_waits("8/17〜8/21, 9/1〜", crate::i18n::Lang::Ja).unwrap(),
            format!("{year}-08-17/{year}-08-21\n{year}-09-01/")
        );
    }

    #[test]
    fn a_wait_that_is_not_a_range_is_refused() {
        assert!(parse_waits("きのう", crate::i18n::Lang::Ja).is_err());
    }

    /// Nobody types the hyphens. Every one of these means the same day.
    #[test]
    fn a_date_can_be_typed_as_numbers() {
        let year = jiff::Zoned::now().date().year();

        assert_eq!(
            flexible_date("20260805").map(|date| date.to_string()),
            Some("2026-08-05".to_owned())
        );
        assert_eq!(
            flexible_date("2026/8/5").map(|date| date.to_string()),
            Some("2026-08-05".to_owned())
        );
        assert_eq!(
            flexible_date("2026年8月5日").map(|date| date.to_string()),
            Some("2026-08-05".to_owned())
        );
        // No year written means this year.
        assert_eq!(
            flexible_date("8/5").map(|date| date.to_string()),
            Some(format!("{year}-08-05"))
        );
        assert_eq!(
            flexible_date("0805").map(|date| date.to_string()),
            Some(format!("{year}-08-05"))
        );
        // Full-width digits work too.
        assert_eq!(
            flexible_date("２０２６－０８－０５").map(|date| date.to_string()),
            Some("2026-08-05".to_owned())
        );
    }

    #[test]
    fn nonsense_is_not_a_date() {
        assert!(flexible_date("きのう").is_none());
        assert!(flexible_date("2026-13-99").is_none());
        assert!(flexible_date("").is_none());
    }
}
