//! Project access: the membership check every project-scoped handler shares,
//! and the queries that build the grid's payload.

use std::collections::HashMap;

use jiff::Zoned;
use serde::Deserialize;
use sqlx::FromRow;
use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, raw_path_params},
};

use crate::{
    db,
    domain::{self, GridData, TaskRow},
};

#[derive(Debug, Clone, FromRow)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub revision: i64,
    /// The current user's role: `owner`, `editor`, or `viewer`.
    pub role: String,
}

impl Project {
    /// Viewers can read the whole plan and change none of it.
    pub fn can_edit(&self) -> bool {
        self.role != "viewer"
    }

    pub fn is_owner(&self) -> bool {
        self.role == "owner"
    }
}

/// Turns a project name into the id that appears in its URL.
///
/// A UUID in the address bar tells nobody which plan they are looking at, and
/// people paste these links to each other. Japanese characters are kept: the
/// browser shows them decoded, and percent-encoding is only how they travel.
pub fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;

    for c in name.trim().chars() {
        // ASCII punctuation either means something in a URL or means nothing to
        // a reader. Whitespace and hyphens both become the separator, so runs
        // of them collapse instead of piling up at the edges.
        let drop = c.is_control()
            || c.is_whitespace()
            || (c.is_ascii_punctuation() && c != '_')
            || "　、。・「」（）【】：；？！".contains(c);

        if drop {
            pending_dash = !slug.is_empty();
            continue;
        }

        if pending_dash {
            slug.push('-');
            pending_dash = false;
        }

        slug.extend(c.to_lowercase());
    }

    // Long enough to stay readable, short enough to paste into chat.
    slug.chars().take(60).collect()
}

/// A named segment of the current route.
pub fn path_str<'cx>(cx: &'cx Cx, name: &str) -> Result<&'cx str> {
    Ok(raw_path_params(cx)
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
        .ok_or_not_found()?)
}

/// The `{project_id}` segment of the current route.
pub fn id_from_path(cx: &Cx) -> Result<&str> {
    path_str(cx, "project_id")
}

/// The project the current URL is about, if any and if the user may see it.
///
/// The layout draws the drawer for every page, so it has to work out the
/// project itself: path parameters only exist on the route that declared them.
pub async fn from_url(cx: &Cx, user_id: &str) -> Option<Project> {
    let path = topcoat::router::uri(cx).path();
    let id = path.strip_prefix("/projects/")?.split('/').next()?;

    if id.is_empty() {
        return None;
    }

    authorize(cx, user_id, id).await.ok()
}

/// The name and revision of a project whose access has already been settled.
///
/// Deliberately not a `Project`: a caller that already holds a role — an access
/// token carries its own — must not have it quietly replaced by a fuller one.
pub async fn name_and_revision(cx: &Cx, project_id: &str) -> Result<Option<(String, i64)>> {
    Ok(
        sqlx::query_as::<_, (String, i64)>("SELECT name, revision FROM projects WHERE id = ?1")
            .bind(project_id)
            .fetch_optional(db::pool(cx))
            .await?,
    )
}

/// The task's parent and order key, if it is part of `project_id`.
///
/// Scoping the lookup to the project is what stops a member of one project
/// from editing a task id belonging to another.
pub async fn task_in_project(
    cx: &Cx,
    project_id: &str,
    task_id: &str,
) -> Result<(Option<String>, String)> {
    let task = sqlx::query_as::<_, (Option<String>, String)>(
        "SELECT parent_id, sort_key FROM tasks WHERE id = ?1 AND project_id = ?2",
    )
    .bind(task_id)
    .bind(project_id)
    .fetch_optional(db::pool(cx))
    .await?
    .ok_or_not_found()?;

    Ok(task)
}

/// Whether the task has any children, and so takes its schedule from them.
pub async fn has_children(cx: &Cx, task_id: &str) -> Result<bool> {
    let (exists,) =
        sqlx::query_as::<_, (bool,)>("SELECT EXISTS(SELECT 1 FROM tasks WHERE parent_id = ?1)")
            .bind(task_id)
            .fetch_one(db::pool(cx))
            .await?;

    Ok(exists)
}

/// The sort key for a row inserted directly below `after`, among its siblings.
pub async fn sort_key_after(
    cx: &Cx,
    project_id: &str,
    parent_id: Option<&str>,
    sort_key: &str,
) -> Result<String> {
    // `IS` rather than `=` so a NULL parent matches the top level.
    let next = sqlx::query_as::<_, (String,)>(
        "SELECT sort_key
           FROM tasks
          WHERE project_id = ?1 AND parent_id IS ?2 AND sort_key > ?3
          ORDER BY sort_key
          LIMIT 1",
    )
    .bind(project_id)
    .bind(parent_id)
    .bind(sort_key)
    .fetch_optional(db::pool(cx))
    .await?;

    Ok(crate::sortkey::between(
        Some(sort_key),
        next.as_ref().map(|(key,)| key.as_str()),
    ))
}

/// The project, if `user_id` is a member of it.
///
/// A project the user cannot see answers as not-found rather than forbidden,
/// so the route never confirms that an id exists.
pub async fn authorize(cx: &Cx, user_id: &str, project_id: &str) -> Result<Project> {
    // The project's own list wins; the account's base role is what applies when
    // the project does not name the person. `none` — the setting for somebody
    // who should only see what they were invited to — means no access at all.
    let project = sqlx::query_as::<_, Project>(
        "SELECT projects.id, projects.name, projects.revision,
                COALESCE(project_members.role,
                         CASE users.base_role
                           WHEN 'admin' THEN 'owner'
                           WHEN 'editor' THEN 'editor'
                           WHEN 'viewer' THEN 'viewer'
                         END) AS role
           FROM projects
           JOIN users ON users.id = ?2
           LEFT JOIN project_members
             ON project_members.project_id = projects.id
            AND project_members.user_id = ?2
          WHERE projects.id = ?1
            AND (project_members.role IS NOT NULL OR users.base_role <> 'none')",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(db::pool(cx))
    .await?
    .ok_or_not_found()?;

    Ok(project)
}

/// An unused id for a project named `name`.
///
/// The name itself is refused when taken, so a suffix here only settles the
/// rarer case of two different names slugging to the same thing.
pub async fn available_id(cx: &Cx, name: &str) -> Result<String> {
    let base = slugify(name);
    let base = if base.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        base
    };

    for attempt in 1..100 {
        let candidate = if attempt == 1 {
            base.clone()
        } else {
            format!("{base}-{attempt}")
        };

        let (taken,) =
            sqlx::query_as::<_, (bool,)>("SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1)")
                .bind(&candidate)
                .fetch_one(db::pool(cx))
                .await?;

        if !taken {
            return Ok(candidate);
        }
    }

    Ok(uuid::Uuid::new_v4().to_string())
}

/// Whether a project of this name already exists.
pub async fn name_taken(cx: &Cx, name: &str) -> Result<bool> {
    let (taken,) = sqlx::query_as::<_, (bool,)>(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE name = ?1 COLLATE NOCASE)",
    )
    .bind(name)
    .fetch_one(db::pool(cx))
    .await?;

    Ok(taken)
}

/// Re-reads a project, picking up a revision a write just bumped.
pub async fn reload(cx: &Cx, project_id: &str, role: &str) -> Result<Project> {
    let project = sqlx::query_as::<_, Project>(
        "SELECT id, name, revision, ?2 AS role FROM projects WHERE id = ?1",
    )
    .bind(project_id)
    .bind(role)
    .fetch_optional(db::pool(cx))
    .await?
    .ok_or_not_found()?;

    Ok(project)
}

/// Everything the grid needs to draw the project once.
pub async fn grid_data(cx: &Cx, project: &Project) -> Result<GridData> {
    let rows = sqlx::query_as::<_, TaskRow>(
        "SELECT id, parent_id, sort_key, name, start_date, end_date,
                actual_start, actual_end, progress, tags,
                status, assignee, note, waits
           FROM tasks
          WHERE project_id = ?1
          ORDER BY sort_key",
    )
    .bind(&project.id)
    .fetch_all(db::pool(cx))
    .await?;

    // "Late" is a question about the user's calendar day, so it follows the
    // server's local zone rather than UTC.
    let today = Zoned::now().date();

    let holidays = holidays(cx, &project.id).await?;
    let leaves = leaves(cx, &project.id).await?;
    let assignees = assignees(cx, &project.id).await?;
    let statuses = statuses(cx, &project.id).await?;
    let theme = domain::Theme::from_settings(&settings(cx, &project.id).await?);
    let fields = fields(cx, &project.id).await?;
    let values = field_values(cx, &project.id).await?;
    let stored = settings(cx, &project.id).await?;

    let hidden: Vec<String> = stored
        .get("hidden_columns")
        .map(|value| value.split_whitespace().map(ToOwned::to_owned).collect())
        .unwrap_or_default();

    let mut data = domain::build(
        &project.id,
        project.revision,
        today,
        rows,
        domain::Settings {
            holidays,
            leaves,
            assignees,
            statuses,
            fiscal_year_start: stored
                .get("fiscal_year_start")
                .and_then(|value| value.parse().ok())
                .filter(|month| (1..=12).contains(month))
                .unwrap_or(4),
            japanese_era: stored.get("japanese_era").is_some_and(|value| value == "1"),
            // Shown by default: a quarter is read together with the business
            // year, so hiding it is the deliberate choice.
            quarters: stored.get("quarters").is_none_or(|value| value == "1"),
            eras: crate::app_settings::parse_eras(&crate::app_settings::eras_text(cx).await),
            day_width: stored
                .get("day_width")
                .and_then(|value| value.parse().ok())
                .filter(|width| (8..=48).contains(width))
                .unwrap_or(26),
            theme,
            fields,
            values,
            hidden_columns: hidden,
            counting: counting(&stored),
            column_order: stored
                .get("column_order")
                .map(|value| value.split_whitespace().map(ToOwned::to_owned).collect())
                .unwrap_or_default(),
            column_widths: column_widths(&stored),
            // The task name alone by default: it is the one column that says
            // which row you are looking at.
            frozen_columns: stored
                .get("frozen_columns")
                .and_then(|value| value.parse().ok())
                .filter(|count| *count <= 8)
                .unwrap_or(1),
            can_edit: project.can_edit(),
        },
    );

    // The wording differs per reader, so the language goes on after the table
    // has been built.
    data.language = crate::i18n::lang(cx).await.code().to_owned();

    Ok(data)
}

/// The sort key that appends a row after every existing one in the project.
pub async fn next_sort_key(cx: &Cx, project_id: &str) -> Result<String> {
    let last = sqlx::query_as::<_, (String,)>(
        "SELECT sort_key FROM tasks WHERE project_id = ?1 ORDER BY sort_key DESC LIMIT 1",
    )
    .bind(project_id)
    .fetch_optional(db::pool(cx))
    .await?;

    Ok(crate::sortkey::between(
        last.as_ref().map(|(key,)| key.as_str()),
        None,
    ))
}

/// How a row moves through the outline.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Move {
    /// Become a child of the row above.
    Indent,
    /// Become a sibling of the current parent, just after it.
    Outdent,
    /// Swap with the previous sibling.
    Up,
    /// Swap with the next sibling.
    Down,
}

/// Moves a row through the outline.
///
/// Returns `None` when the row moved, or the reason it could not. Refusing in
/// silence was the original design and it was wrong: "nothing happened" is
/// indistinguishable from "broken" when the rule — a row becomes a child of its
/// previous *sibling*, not of whatever is drawn above it — is not obvious.
pub async fn move_task(
    cx: &Cx,
    project_id: &str,
    task_id: &str,
    action: Move,
) -> Result<Option<&'static str>> {
    let (parent_id, _) = task_in_project(cx, project_id, task_id).await?;
    let siblings = siblings(cx, project_id, parent_id.as_deref()).await?;

    let index = siblings
        .iter()
        .position(|(id, _)| id == task_id)
        .ok_or_not_found()?;

    let key = |i: usize| siblings.get(i).map(|(_, key)| key.as_str());

    match action {
        Move::Indent => {
            // The new parent is the row above, which cannot be a descendant of
            // this one, so the tree stays a tree.
            let Some((new_parent, _)) = index.checked_sub(1).and_then(|i| siblings.get(i)) else {
                return Ok(Some(
                    "同じ階層で1つ上の行がないため、子タスクにできません。",
                ));
            };

            let last = last_child_key(cx, project_id, new_parent).await?;
            let sort_key = crate::sortkey::between(last.as_deref(), None);

            reparent(cx, task_id, Some(new_parent), &sort_key).await?;
        }
        Move::Outdent => {
            let Some(parent) = parent_id else {
                return Ok(Some("すでに最上位の行です。"));
            };

            let (grandparent, parent_key) = task_in_project(cx, project_id, &parent).await?;
            let sort_key =
                sort_key_after(cx, project_id, grandparent.as_deref(), &parent_key).await?;

            reparent(cx, task_id, grandparent.as_deref(), &sort_key).await?;
        }
        Move::Up => {
            let Some(previous) = key(index.wrapping_sub(1)) else {
                return Ok(Some("すでに同じ階層の先頭です。"));
            };

            // Land between the row two above and the one directly above.
            let sort_key = crate::sortkey::between(key(index.wrapping_sub(2)), Some(previous));

            resort(cx, task_id, &sort_key).await?;
        }
        Move::Down => {
            let Some(next) = key(index + 1) else {
                return Ok(Some("すでに同じ階層の末尾です。"));
            };

            let sort_key = crate::sortkey::between(Some(next), key(index + 2));

            resort(cx, task_id, &sort_key).await?;
        }
    }

    Ok(None)
}

/// Moves a row to an exact place: under `parent`, directly after `after`.
///
/// `after` of `None` means first among those children. This is what dragging
/// needs — the relative moves cannot express "between these two rows".
pub async fn place_task(
    cx: &Cx,
    project_id: &str,
    task_id: &str,
    parent: Option<&str>,
    after: Option<&str>,
) -> Result<Option<&'static str>> {
    if parent == Some(task_id) {
        return Ok(Some("自分自身の中には入れられません。"));
    }

    // A row dropped inside its own subtree would cut that subtree loose from
    // the tree entirely, and it would never be seen again.
    if let Some(parent) = parent
        && is_descendant(cx, project_id, parent, task_id).await?
    {
        return Ok(Some("自分の子タスクの中には入れられません。"));
    }

    let siblings = siblings(cx, project_id, parent).await?;

    let previous = match after {
        Some(after) => {
            let position = siblings.iter().position(|(id, _)| id == after);
            let Some(position) = position else {
                return Ok(Some("移動先が見つかりません。"));
            };
            Some(siblings[position].1.clone())
        }
        None => None,
    };

    // The row after the drop point, skipping the dragged row itself: leaving it
    // in would place the new key on the wrong side of where it already sits.
    let next = siblings
        .iter()
        .find(|(id, key)| {
            id != task_id && previous.as_deref().is_none_or(|previous| key.as_str() > previous)
        })
        .map(|(_, key)| key.clone());

    let sort_key = crate::sortkey::between(previous.as_deref(), next.as_deref());

    reparent(cx, task_id, parent, &sort_key).await?;

    Ok(None)
}

/// Whether `candidate` sits inside `ancestor`'s subtree.
async fn is_descendant(
    cx: &Cx,
    project_id: &str,
    candidate: &str,
    ancestor: &str,
) -> Result<bool> {
    let mut current = candidate.to_owned();

    // The tree is shallow in practice, and walking up is bounded by its depth.
    for _ in 0..64 {
        let (parent, _) = task_in_project(cx, project_id, &current).await?;

        let Some(parent) = parent else { return Ok(false) };
        if parent == ancestor {
            return Ok(true);
        }

        current = parent;
    }

    Ok(false)
}

/// The rows sharing a parent, in order.
pub(crate) async fn siblings(
    cx: &Cx,
    project_id: &str,
    parent_id: Option<&str>,
) -> Result<Vec<(String, String)>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT id, sort_key
           FROM tasks
          WHERE project_id = ?1 AND parent_id IS ?2
          ORDER BY sort_key",
    )
    .bind(project_id)
    .bind(parent_id)
    .fetch_all(db::pool(cx))
    .await?;

    Ok(rows)
}

async fn last_child_key(cx: &Cx, project_id: &str, parent_id: &str) -> Result<Option<String>> {
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT sort_key
           FROM tasks
          WHERE project_id = ?1 AND parent_id = ?2
          ORDER BY sort_key DESC
          LIMIT 1",
    )
    .bind(project_id)
    .bind(parent_id)
    .fetch_optional(db::pool(cx))
    .await?;

    Ok(row.map(|(key,)| key))
}

async fn reparent(
    cx: &Cx,
    task_id: &str,
    parent_id: Option<&str>,
    sort_key: &str,
) -> Result<()> {
    sqlx::query("UPDATE tasks SET parent_id = ?1, sort_key = ?2, updated_at = ?3 WHERE id = ?4")
        .bind(parent_id)
        .bind(sort_key)
        .bind(db::now())
        .bind(task_id)
        .execute(db::pool(cx))
        .await?;

    Ok(())
}

async fn resort(cx: &Cx, task_id: &str, sort_key: &str) -> Result<()> {
    sqlx::query("UPDATE tasks SET sort_key = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(sort_key)
        .bind(db::now())
        .bind(task_id)
        .execute(db::pool(cx))
        .await?;

    Ok(())
}


/// Records a change to the project, so clients can tell they are behind.
pub async fn bump_revision(cx: &Cx, project_id: &str) -> Result<()> {
    sqlx::query("UPDATE projects SET revision = revision + 1, updated_at = ?2 WHERE id = ?1")
        .bind(project_id)
        .bind(db::now())
        .execute(db::pool(cx))
        .await?;

    Ok(())
}

/// The project's holidays, in date order, with the names to show on hover.
///
/// Holidays are the company's calendar, so the list itself is installation-wide
/// and a project holds only its difference: days off this workplace alone takes
/// (`add`), and shared holidays it works through (`skip`).
pub async fn holidays(cx: &Cx, project_id: &str) -> Result<Vec<domain::Holiday>> {
    let global = sqlx::query_as::<_, (String, String)>("SELECT date, name FROM app_holidays")
        .fetch_all(db::pool(cx))
        .await?;

    let own = sqlx::query_as::<_, (String, String, String)>(
        "SELECT date, name, kind FROM project_holidays WHERE project_id = ?1",
    )
    .bind(project_id)
    .fetch_all(db::pool(cx))
    .await?;

    // A map keyed by date: the project's own entry wins over the shared one, and
    // ordering falls out of the key.
    let mut days: std::collections::BTreeMap<String, String> = global.into_iter().collect();

    for (date, name, kind) in own {
        if kind == "skip" {
            days.remove(&date);
        } else {
            days.insert(date, name);
        }
    }

    Ok(days
        .into_iter()
        .map(|(date, name)| domain::Holiday { date, name })
        .collect())
}

/// The shared holidays. This is the list the settings page edits.
pub async fn app_holidays(cx: &Cx) -> Result<Vec<domain::Holiday>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT date, name FROM app_holidays ORDER BY date",
    )
    .fetch_all(db::pool(cx))
    .await?;

    Ok(rows
        .into_iter()
        .map(|(date, name)| domain::Holiday { date, name })
        .collect())
}

/// The installation-wide assignee list: names and colours, nothing else.
pub async fn assignee_master(cx: &Cx) -> Result<Vec<domain::Assignee>> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT name, color, background FROM assignees ORDER BY name",
    )
    .fetch_all(db::pool(cx))
    .await?;

    Ok(rows
        .into_iter()
        .map(|(name, color, background)| domain::Assignee {
            name,
            color,
            background,
        })
        .collect())
}

/// Tells every open screen that it is behind, after an installation-wide change.
///
/// Holidays and colours change how every project looks, so bumping one project's
/// revision is not enough.
pub async fn bump_everything(cx: &Cx) -> Result<()> {
    sqlx::query("UPDATE projects SET revision = revision + 1, updated_at = ?1")
        .bind(db::now())
        .execute(db::pool(cx))
        .await?;

    Ok(())
}

/// Only this project's own difference. The settings page shows it beside the
/// shared days.
pub async fn holiday_diff(cx: &Cx, project_id: &str) -> Result<Vec<(String, String, String)>> {
    Ok(sqlx::query_as::<_, (String, String, String)>(
        "SELECT date, name, kind FROM project_holidays WHERE project_id = ?1 ORDER BY date",
    )
    .bind(project_id)
    .fetch_all(db::pool(cx))
    .await?)
}

/// Puts a whole document back: settings, master lists, calendar, and plan.
///
/// One transaction — a half-restored project is worse than none — and only the
/// sections the file carries. A file with just tasks still works, and leaves the
/// project's settings where they were.
pub async fn import_project(
    cx: &Cx,
    project_id: &str,
    user_id: &str,
    document: &crate::interop::json::Document,
) -> Result<()> {
    let mut tx = db::pool(cx).begin().await?;

    if let Some(settings) = &document.settings {
        sqlx::query("DELETE FROM project_settings WHERE project_id = ?1")
            .bind(project_id)
            .execute(&mut *tx)
            .await?;

        for (key, value) in settings {
            sqlx::query(
                "INSERT INTO project_settings (project_id, key, value) VALUES (?1, ?2, ?3)",
            )
            .bind(project_id)
            .bind(key)
            .bind(value)
            .execute(&mut *tx)
            .await?;
        }
    }

    if let Some(statuses) = &document.statuses {
        sqlx::query("DELETE FROM project_statuses WHERE project_id = ?1")
            .bind(project_id)
            .execute(&mut *tx)
            .await?;

        for (position, status) in statuses.iter().enumerate() {
            sqlx::query(
                "INSERT INTO project_statuses (project_id, position, name, color, percent)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(project_id)
            .bind(position as i64)
            .bind(&status.name)
            .bind(&status.color)
            .bind(status.percent)
            .execute(&mut *tx)
            .await?;
        }
    }

    if let Some(assignees) = &document.assignees {
        sqlx::query("DELETE FROM project_assignees WHERE project_id = ?1")
            .bind(project_id)
            .execute(&mut *tx)
            .await?;

        for person in assignees {
            sqlx::query("INSERT INTO project_assignees (project_id, name) VALUES (?1, ?2)")
                .bind(project_id)
                .bind(&person.name)
                .execute(&mut *tx)
                .await?;

            // Colour belongs to the shared list. A colour from an imported file
            // is applied only to someone who has none yet, so one import cannot
            // repaint a person other projects already rely on.
            sqlx::query(
                "INSERT INTO assignees (name, color, background) VALUES (?1, ?2, ?3)
                 ON CONFLICT (name) DO UPDATE
                    SET color = CASE WHEN assignees.color = '' THEN excluded.color
                                     ELSE assignees.color END,
                        background = CASE WHEN assignees.background = '' THEN excluded.background
                                          ELSE assignees.background END",
            )
            .bind(&person.name)
            .bind(&person.color)
            .bind(&person.background)
            .execute(&mut *tx)
            .await?;
        }
    }

    if let Some(holidays) = &document.holidays {
        sqlx::query("DELETE FROM project_holidays WHERE project_id = ?1")
            .bind(project_id)
            .execute(&mut *tx)
            .await?;

        for holiday in holidays {
            sqlx::query(
                "INSERT INTO project_holidays (project_id, date, name, kind)
                 VALUES (?1, ?2, ?3, 'add')
                 ON CONFLICT (project_id, date) DO NOTHING",
            )
            .bind(project_id)
            .bind(&holiday.date)
            .bind(&holiday.name)
            .execute(&mut *tx)
            .await?;
        }
    }

    if let Some(leaves) = &document.leaves {
        // Leave is installation-wide, so an import may drop only duplicates of
        // what it carries. Other people's plans are not collateral.
        for leave in leaves {
            sqlx::query(
                "INSERT INTO leaves (id, assignee, start_date, end_date, note, kind, created_at)
                 SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7
                  WHERE NOT EXISTS (
                    SELECT 1 FROM leaves
                     WHERE assignee = ?2 AND start_date = ?3 AND end_date = ?4 AND kind = ?6
                  )",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&leave.assignee)
            .bind(&leave.start)
            .bind(&leave.end)
            .bind(&leave.note)
            .bind(&leave.kind)
            .bind(db::now())
            .execute(&mut *tx)
            .await?;
        }
    }

    // Fields are rebuilt from scratch, so their ids change. Task values arrive
    // keyed by label, which is the only name that means anything in another
    // installation, and are matched up through this.
    let mut by_label: HashMap<String, String> = HashMap::new();

    if let Some(fields) = &document.fields {
        sqlx::query(
            "DELETE FROM project_fields WHERE project_id = ?1",
        )
        .bind(project_id)
        .execute(&mut *tx)
        .await?;

        let mut key = String::new();

        for field in fields {
            let id = uuid::Uuid::new_v4().to_string();
            key = crate::sortkey::between(Some(&key), None);

            sqlx::query(
                "INSERT INTO project_fields (id, project_id, label, kind, sort_key)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(&id)
            .bind(project_id)
            .bind(&field.label)
            .bind(&field.kind)
            .bind(&key)
            .execute(&mut *tx)
            .await?;

            let mut option_key = String::new();
            for option in &field.options {
                option_key = crate::sortkey::between(Some(&option_key), None);

                sqlx::query(
                    "INSERT INTO project_field_options
                         (field_id, value, sort_key, color, background)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT (field_id, value) DO NOTHING",
                )
                .bind(&id)
                .bind(&option.value)
                .bind(&option_key)
                .bind(&option.color)
                .bind(&option.background)
                .execute(&mut *tx)
                .await?;
            }

            by_label.insert(field.label.clone(), id);
        }
    } else {
        // Without a fields section the columns stay as they are, and the values
        // arriving with the tasks are matched to them by label.
        let existing = sqlx::query_as::<_, (String, String)>(
            "SELECT id, label FROM project_fields WHERE project_id = ?1",
        )
        .bind(project_id)
        .fetch_all(&mut *tx)
        .await?;

        by_label.extend(existing.into_iter().map(|(id, label)| (label, id)));
    }

    // Rows the project already has. A file that carries their ids edits them in
    // place; without this every import would hand back a project of new rows,
    // and a second round trip could never say "this one, not that one".
    let existing: std::collections::HashSet<String> =
        sqlx::query_as::<_, (String,)>("SELECT id FROM tasks WHERE project_id = ?1")
            .bind(project_id)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|(id,)| id)
            .collect();

    // The id of the row at each depth, so the next deeper row knows its parent.
    let mut ancestors: Vec<String> = Vec::new();
    let mut sort_key = String::new();
    let mut kept: std::collections::HashSet<String> = std::collections::HashSet::new();

    for task in &document.tasks {
        // An id the project knows names a row to update. Anything else — no id,
        // or one from somewhere else — is a new row.
        let known = existing.contains(&task.id);
        let id = if known {
            task.id.clone()
        } else {
            uuid::Uuid::new_v4().to_string()
        };

        ancestors.truncate(task.depth);
        let parent = ancestors.last().cloned();

        // Siblings at different depths never share a key, so one running
        // sequence is enough to keep the file's order.
        sort_key = crate::sortkey::between(Some(&sort_key), None);

        if known {
            sqlx::query(
                "UPDATE tasks
                    SET parent_id = ?3, sort_key = ?4, name = ?5,
                        start_date = ?6, end_date = ?7, actual_start = ?8, actual_end = ?9,
                        progress = ?10, status = ?11, assignee = ?12, note = ?13, waits = ?14,
                        updated_at = ?15, updated_by = ?16
                  WHERE id = ?1 AND project_id = ?2",
            )
        } else {
            sqlx::query(
                "INSERT INTO tasks (id, project_id, parent_id, sort_key, name,
                                    start_date, end_date, actual_start, actual_end,
                                    progress, status, assignee, note, waits,
                                    updated_at, updated_by)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            )
        }
        .bind(&id)
        .bind(project_id)
        .bind(&parent)
        .bind(&sort_key)
        .bind(&task.name)
        .bind(&task.start)
        .bind(&task.end)
        .bind(&task.actual_start)
        .bind(&task.actual_end)
        .bind(task.progress)
        .bind(&task.status)
        .bind(&task.assignee)
        .bind(&task.note)
        .bind(task.waits.join(" "))
        .bind(db::now())
        // An empty author is an access token rather than a person: the column
        // points at an account, and inventing one would put a name on work
        // nobody did.
        .bind((!user_id.is_empty()).then_some(user_id))
        .execute(&mut *tx)
        .await?;

        kept.insert(id.clone());

        // The values arrive whole, so the old ones go first — otherwise a value
        // the file dropped would survive the import it was dropped in.
        sqlx::query("DELETE FROM task_field_values WHERE task_id = ?1")
            .bind(&id)
            .execute(&mut *tx)
            .await?;

        for (label, value) in &task.fields {
            let Some(field_id) = by_label.get(label) else {
                continue;
            };

            sqlx::query(
                "INSERT INTO task_field_values (task_id, field_id, value)
                 VALUES (?1, ?2, ?3)",
            )
            .bind(&id)
            .bind(field_id)
            .bind(value)
            .execute(&mut *tx)
            .await?;
        }

        ancestors.push(id);
    }

    // Whatever the file did not mention is gone from it on purpose.
    for id in existing.difference(&kept) {
        sqlx::query("DELETE FROM tasks WHERE id = ?1 AND project_id = ?2")
            .bind(id)
            .bind(project_id)
            .execute(&mut *tx)
            .await?;
    }

    sqlx::query("UPDATE projects SET revision = revision + 1, updated_at = ?2 WHERE id = ?1")
        .bind(project_id)
        .bind(db::now())
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}

/// What the JSON export needs that the grid's data does not carry.
pub async fn export_extras(cx: &Cx, project_id: &str) -> Result<crate::interop::json::Extras> {
    let settings = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM project_settings WHERE project_id = ?1 ORDER BY key",
    )
    .bind(project_id)
    .fetch_all(db::pool(cx))
    .await?
    .into_iter()
    .collect();

    // The list belongs to the project, the colours to the installation. The file
    // carries both — pull everything out and you can take everything with you —
    // so it looks the same on an installation that has no shared list yet.
    let assignees = sqlx::query_as::<_, (String, String, String)>(
        "SELECT project_assignees.name,
                COALESCE(assignees.color, ''),
                COALESCE(assignees.background, '')
           FROM project_assignees
           LEFT JOIN assignees ON assignees.name = project_assignees.name
          WHERE project_assignees.project_id = ?1
          ORDER BY project_assignees.name",
    )
    .bind(project_id)
    .fetch_all(db::pool(cx))
    .await?
    .into_iter()
    .map(
        |(name, color, background)| crate::interop::json::Assignee {
            name,
            color,
            background,
        },
    )
    .collect();

    Ok(crate::interop::json::Extras {
        settings,
        assignees,
    })
}

/// Every column this project has, in the order it shows them.
///
/// Built-ins first in their declared order, then the project's own; anything the
/// stored order does not mention keeps its place at the end, so adding a column
/// never depends on the setting being up to date.
pub fn column_order(data: &GridData) -> Vec<String> {
    let all: Vec<String> = crate::api::COLUMN_KEYS
        .iter()
        .map(|key| (*key).to_owned())
        .chain(data.fields.iter().map(|field| field.id.clone()))
        .collect();

    let mut ordered: Vec<String> = data
        .column_order
        .iter()
        .filter(|key| all.contains(key))
        .cloned()
        .collect();

    for key in all {
        if !ordered.contains(&key) {
            ordered.push(key);
        }
    }

    ordered
}

/// Column widths in pixels, stored as `key:px` pairs.
///
/// A width outside what a column can usefully be is dropped rather than
/// refused: the setting is cosmetic, and a bad one must not empty the table.
fn column_widths(stored: &HashMap<String, String>) -> HashMap<String, u32> {
    stored
        .get("column_widths")
        .map(|value| {
            value
                .split_whitespace()
                .filter_map(|pair| {
                    let (key, width) = pair.split_once(':')?;
                    let width: u32 = width.parse().ok()?;

                    (40..=600).contains(&width).then(|| (key.to_owned(), width))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Which days the day count leaves out.
///
/// The old single `workdays_only` switch stands in for the three calendar ones
/// when the individual keys have never been written: a project that ticked it
/// keeps counting the way it did.
fn counting(stored: &HashMap<String, String>) -> domain::Counting {
    let on = |key: &str| stored.get(key).is_some_and(|value| value == "1");
    let legacy = on("workdays_only");

    if stored.contains_key("counting") {
        domain::Counting {
            monday: on("skip_monday"),
            tuesday: on("skip_tuesday"),
            wednesday: on("skip_wednesday"),
            thursday: on("skip_thursday"),
            friday: on("skip_friday"),
            saturday: on("skip_saturday"),
            sunday: on("skip_sunday"),
            holidays: on("skip_holidays"),
            leave: on("skip_leave"),
        }
    } else if stored.contains_key("workdays_only") {
        // An older project that answered the single switch, either way. That
        // answer stands: raising the default must not quietly start skipping
        // days for somebody who said not to.
        domain::Counting {
            saturday: legacy,
            sunday: legacy,
            holidays: legacy,
            leave: true,
            ..domain::Counting::default()
        }
    } else {
        // Never asked. Weekends and holidays are days off.
        domain::Counting::default()
    }
}

/// Checks that a field belongs to the project, so one project cannot write
/// into another's master list by guessing an id.
pub async fn field_in_project(cx: &Cx, project_id: &str, field_id: &str) -> Result<()> {
    sqlx::query_as::<_, (String,)>("SELECT id FROM project_fields WHERE id = ?1 AND project_id = ?2")
        .bind(field_id)
        .bind(project_id)
        .fetch_optional(db::pool(cx))
        .await?
        .ok_or_not_found()?;

    Ok(())
}

/// The states this project uses, in the order they were defined.
///
/// A project that has never touched the list gets the built-in one rather than
/// an empty menu: the column has to mean something on day one.
pub async fn statuses(cx: &Cx, project_id: &str) -> Result<Vec<domain::Status>> {
    let rows = sqlx::query_as::<_, (String, String, Option<i64>)>(
        "SELECT name, color, percent FROM project_statuses
          WHERE project_id = ?1
          ORDER BY position, name",
    )
    .bind(project_id)
    .fetch_all(db::pool(cx))
    .await?;

    if rows.is_empty() {
        return Ok(domain::Status::defaults());
    }

    Ok(rows
        .into_iter()
        .map(|(name, color, percent)| domain::Status {
            name,
            color,
            percent,
        })
        .collect())
}

/// The statuses a new project starts from.
///
/// The installation's list if somebody has set one, and the shipped defaults
/// otherwise. Read once, at creation: see `0023_default_statuses.sql` for why
/// this is a copy rather than a subscription.
pub async fn default_statuses(cx: &Cx) -> Result<Vec<domain::Status>> {
    let rows = sqlx::query_as::<_, (String, String, Option<i64>)>(
        "SELECT name, color, percent FROM app_statuses ORDER BY position, name",
    )
    .fetch_all(db::pool(cx))
    .await?;

    if rows.is_empty() {
        return Ok(domain::Status::defaults());
    }

    Ok(rows
        .into_iter()
        .map(|(name, color, percent)| domain::Status {
            name,
            color,
            percent,
        })
        .collect())
}

/// Who a task can be assigned to.
///
/// The project's members by the name they chose, plus anyone already named on
/// a task — a plan that predates this list must not lose its assignments, and
/// people outside the tool still appear on plans.
pub async fn assignees(cx: &Cx, project_id: &str) -> Result<Vec<domain::Assignee>> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        // Who is on this plan is a fact about the project; what colour they are
        // is a fact about the company. One person in two colours is unreadable
        // the moment you look at several projects at once.
        "SELECT names.name,
                COALESCE(assignees.color, '') AS color,
                COALESCE(assignees.background, '') AS background
           FROM (
             SELECT CASE WHEN users.display_name = '' THEN users.email ELSE users.display_name END
                    AS name
               FROM project_members
               JOIN users ON users.id = project_members.user_id
              WHERE project_members.project_id = ?1
             UNION
             SELECT TRIM(assignee) FROM tasks
              WHERE project_id = ?1 AND TRIM(assignee) <> ''
             UNION
             SELECT name FROM project_assignees WHERE project_id = ?1
           ) AS names
           LEFT JOIN assignees ON assignees.name = names.name
          ORDER BY names.name",
    )
    .bind(project_id)
    .fetch_all(db::pool(cx))
    .await?;

    Ok(rows
        .into_iter()
        .map(|(name, color, background)| domain::Assignee {
            name,
            color,
            background,
        })
        .collect())
}

/// Leave by assignee, oldest first.
///
/// A person is away, not a project, so there is one table for the installation.
/// What comes back is the part of it belonging to people named on this plan:
/// drawing another department's week helps nobody.
pub async fn leaves(cx: &Cx, project_id: &str) -> Result<Vec<domain::Leave>> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT id, assignee, start_date, end_date, note, kind
           FROM leaves
          WHERE assignee IN (
             SELECT CASE WHEN users.display_name = '' THEN users.email ELSE users.display_name END
               FROM project_members
               JOIN users ON users.id = project_members.user_id
              WHERE project_members.project_id = ?1
             UNION
             SELECT TRIM(assignee) FROM tasks
              WHERE project_id = ?1 AND TRIM(assignee) <> ''
             UNION
             SELECT name FROM project_assignees WHERE project_id = ?1
          )
          ORDER BY start_date, assignee",
    )
    .bind(project_id)
    .fetch_all(db::pool(cx))
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, assignee, start, end, note, kind)| domain::Leave {
            id,
            assignee,
            start,
            end,
            note,
            kind,
        })
        .collect())
}

/// The project's settings as a map.
pub async fn settings(cx: &Cx, project_id: &str) -> Result<HashMap<String, String>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM project_settings WHERE project_id = ?1",
    )
    .bind(project_id)
    .fetch_all(db::pool(cx))
    .await?;

    Ok(rows.into_iter().collect())
}

/// Stores one setting, or removes it when `value` is empty.
pub async fn set_setting(cx: &Cx, project_id: &str, key: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        sqlx::query("DELETE FROM project_settings WHERE project_id = ?1 AND key = ?2")
            .bind(project_id)
            .bind(key)
            .execute(db::pool(cx))
            .await?;

        return Ok(());
    }

    sqlx::query(
        "INSERT INTO project_settings (project_id, key, value) VALUES (?1, ?2, ?3)
         ON CONFLICT (project_id, key) DO UPDATE SET value = excluded.value",
    )
    .bind(project_id)
    .bind(key)
    .bind(value)
    .execute(db::pool(cx))
    .await?;

    Ok(())
}

/// The project's own columns, in order, with their master lists.
pub async fn fields(cx: &Cx, project_id: &str) -> Result<Vec<domain::Field>> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, label, kind FROM project_fields WHERE project_id = ?1 ORDER BY sort_key",
    )
    .bind(project_id)
    .fetch_all(db::pool(cx))
    .await?;

    let options = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT project_field_options.field_id, project_field_options.value,
                project_field_options.color, project_field_options.background
           FROM project_field_options
           JOIN project_fields ON project_fields.id = project_field_options.field_id
          WHERE project_fields.project_id = ?1
          ORDER BY project_field_options.sort_key",
    )
    .bind(project_id)
    .fetch_all(db::pool(cx))
    .await?;

    // Which fields have anything written in them. Changing a field's kind is
    // safe while it is empty and a way to lose meaning once it is not.
    let used: std::collections::HashSet<String> = sqlx::query_as::<_, (String,)>(
        "SELECT DISTINCT task_field_values.field_id
           FROM task_field_values
           JOIN project_fields ON project_fields.id = task_field_values.field_id
          WHERE project_fields.project_id = ?1 AND TRIM(task_field_values.value) <> ''",
    )
    .bind(project_id)
    .fetch_all(db::pool(cx))
    .await?
    .into_iter()
    .map(|(id,)| id)
    .collect();

    let mut by_field: HashMap<String, Vec<domain::Option_>> = HashMap::new();
    for (field_id, value, color, background) in options {
        by_field
            .entry(field_id)
            .or_default()
            .push(domain::Option_ {
                value,
                color,
                background,
            });
    }

    Ok(rows
        .into_iter()
        .map(|(id, label, kind)| domain::Field {
            options: by_field.remove(&id).unwrap_or_default(),
            in_use: used.contains(&id),
            id,
            label,
            kind,
        })
        .collect())
}

/// Every custom value in the project, grouped by task.
pub async fn field_values(
    cx: &Cx,
    project_id: &str,
) -> Result<HashMap<String, HashMap<String, String>>> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT task_field_values.task_id, task_field_values.field_id, task_field_values.value
           FROM task_field_values
           JOIN tasks ON tasks.id = task_field_values.task_id
          WHERE tasks.project_id = ?1",
    )
    .bind(project_id)
    .fetch_all(db::pool(cx))
    .await?;

    let mut values: HashMap<String, HashMap<String, String>> = HashMap::new();
    for (task_id, field_id, value) in rows {
        values.entry(task_id).or_default().insert(field_id, value);
    }

    Ok(values)
}

/// Stores one custom value, removing the row when the value is cleared.
pub async fn set_field_value(
    cx: &Cx,
    project_id: &str,
    task_id: &str,
    field_id: &str,
    value: &str,
) -> Result<()> {
    // Scoped to the project so a field id from elsewhere cannot be written.
    let (belongs,) = sqlx::query_as::<_, (bool,)>(
        "SELECT EXISTS(SELECT 1 FROM project_fields WHERE id = ?1 AND project_id = ?2)",
    )
    .bind(field_id)
    .bind(project_id)
    .fetch_one(db::pool(cx))
    .await?;

    if !belongs {
        return Err(topcoat::router::error::not_found().into());
    }

    if value.is_empty() {
        sqlx::query("DELETE FROM task_field_values WHERE task_id = ?1 AND field_id = ?2")
            .bind(task_id)
            .bind(field_id)
            .execute(db::pool(cx))
            .await?;

        return Ok(());
    }

    sqlx::query(
        "INSERT INTO task_field_values (task_id, field_id, value) VALUES (?1, ?2, ?3)
         ON CONFLICT (task_id, field_id) DO UPDATE SET value = excluded.value",
    )
    .bind(task_id)
    .bind(field_id)
    .bind(value)
    .execute(db::pool(cx))
    .await?;

    Ok(())
}

/// The order key that appends a column, or an option, to the end of its list.
pub async fn next_key(cx: &Cx, sql: &str, scope: &str) -> Result<String> {
    let last = sqlx::query_as::<_, (String,)>(sql)
        .bind(scope)
        .fetch_optional(db::pool(cx))
        .await?;

    Ok(crate::sortkey::between(
        last.as_ref().map(|(key,)| key.as_str()),
        None,
    ))
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn spaces_become_separators() {
        assert_eq!(slugify("Release Plan 2026"), "release-plan-2026");
        assert_eq!(slugify("  余白  だらけ  "), "余白-だらけ");
    }

    /// Japanese stays: the address bar shows it decoded, and a readable link is
    /// the whole point.
    #[test]
    fn japanese_survives() {
        assert_eq!(slugify("リリース計画"), "リリース計画");
        assert_eq!(slugify("2026年度 上期"), "2026年度-上期");
    }

    /// Anything that means something to a URL is taken out rather than escaped.
    #[test]
    fn url_punctuation_is_dropped() {
        assert_eq!(slugify("a/b?c#d"), "a-b-c-d");
        assert_eq!(slugify("【重要】設計・実装"), "重要-設計-実装");
    }

    #[test]
    fn separators_never_double_up_or_dangle() {
        assert_eq!(slugify("a   ///   b"), "a-b");
        assert_eq!(slugify("--- 端 ---"), "端");
    }

    #[test]
    fn a_name_with_nothing_usable_slugs_to_nothing() {
        assert_eq!(slugify("///"), "");
        assert_eq!(slugify("   "), "");
    }
}
