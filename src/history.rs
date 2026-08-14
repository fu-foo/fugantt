//! What changed, and who changed it.
//!
//! Written as each mutation happens: the previous value only exists at the
//! moment it is replaced, so there is no way to reconstruct this later. Rows
//! keep the task's name as text, so history about a deleted task still reads
//! as something rather than an id.

use sqlx::FromRow;
use topcoat::{Result, context::Cx};

use crate::db;

#[derive(Debug, Clone, FromRow)]
pub struct Change {
    pub task_name: String,
    pub action: String,
    pub field: String,
    pub before: String,
    pub after: String,
    pub actor: String,
    pub at: i64,
}

/// One change, as it happened.
pub struct Entry<'a> {
    pub project_id: &'a str,
    pub task_id: Option<&'a str>,
    pub task_name: &'a str,
    pub action: &'a str,
    pub field: &'a str,
    pub before: &'a str,
    pub after: &'a str,
    pub actor: &'a str,
}

/// Records one change.
pub async fn record(cx: &Cx, entry: Entry<'_>) -> Result<()> {
    let Entry {
        project_id,
        task_id,
        task_name,
        action,
        field,
        before,
        after,
        actor,
    } = entry;

    sqlx::query(
        "INSERT INTO changes
             (project_id, task_id, task_name, action, field, before, after, actor, at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )
    .bind(project_id)
    .bind(task_id)
    .bind(task_name)
    .bind(action)
    .bind(field)
    .bind(truncate(before))
    .bind(truncate(after))
    .bind(actor)
    .bind(db::now())
    .execute(db::pool(cx))
    .await?;

    Ok(())
}

/// The project's history, newest first.
pub async fn list(cx: &Cx, project_id: &str, limit: i64, skip: i64) -> Result<Vec<Change>> {
    let changes = sqlx::query_as::<_, Change>(
        "SELECT task_name, action, field, before, after, actor, at
           FROM changes
          WHERE project_id = ?1
          ORDER BY id DESC
          LIMIT ?2 OFFSET ?3",
    )
    .bind(project_id)
    .bind(limit)
    .bind(skip)
    .fetch_all(db::pool(cx))
    .await?;

    Ok(changes)
}

/// How many changes this project has, so the page can say where it is.
pub async fn count(cx: &Cx, project_id: &str) -> Result<i64> {
    let (total,) = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM changes WHERE project_id = ?1")
        .bind(project_id)
        .fetch_one(db::pool(cx))
        .await?;

    Ok(total)
}

/// A note's worth of text is history; a pasted document is not.
fn truncate(value: &str) -> String {
    value.chars().take(200).collect()
}

/// The stored name of a task, for history that outlives it.
pub async fn task_name(cx: &Cx, task_id: &str) -> String {
    sqlx::query_as::<_, (String,)>("SELECT name FROM tasks WHERE id = ?1")
        .bind(task_id)
        .fetch_optional(db::pool(cx))
        .await
        .ok()
        .flatten()
        .map(|(name,)| name)
        .unwrap_or_default()
}

/// The value a field holds right now, so a change can say what it replaced.
pub async fn current_value(cx: &Cx, task_id: &str, column: &str) -> String {
    // `column` comes from a closed set in the caller, never from a request.
    let sql = format!("SELECT COALESCE(CAST({column} AS TEXT), '') FROM tasks WHERE id = ?1");

    sqlx::query_as::<_, (String,)>(&sql)
        .bind(task_id)
        .fetch_optional(db::pool(cx))
        .await
        .ok()
        .flatten()
        .map(|(value,)| value)
        .unwrap_or_default()
}
