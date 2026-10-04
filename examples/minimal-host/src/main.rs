//! The smallest thing that can show the grid.
//!
//! No database and no accounts: three rows held in memory, and the handful of
//! addresses the grid needs to draw them and change them. It is here to show
//! what a host has to supply, and to prove the grid runs somewhere that is not
//! fugantt — at an address of this program's own choosing.
//!
//!     cargo run -p minimal-host
//!
//! What it leaves out, a real host answers too: placing a row by dragging,
//! deleting, one-row patches, live updates, saved filters and leave. See
//! `docs/gantt-api.md`.

use std::sync::{Mutex, PoisonError};

use fu_gantt_core::{
    domain::{self, GridData, Settings, TaskRow},
    sortkey, text,
    wire::{CellEdit, CellField, InsertTask, Move, MoveRequest, Mutation},
};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Body, Response, Router, RouterBuilderDiscoverExt,
        content::Json,
        error::{bad_request, not_found},
        query_params, raw_path_params, route,
    },
};

/// Where the grid is told to look. Deliberately not `/api/projects/…`.
const API: &str = "/g/api/plans/demo";

struct Plan(Mutex<State>);

struct State {
    revision: i64,
    next_id: u32,
    rows: Vec<TaskRow>,
}

fn row(id: &str, sort_key: &str, name: &str, start: &str, end: &str) -> TaskRow {
    let date = |text: &str| (!text.is_empty()).then(|| text.to_owned());

    TaskRow {
        id: id.to_owned(),
        parent_id: None,
        sort_key: sort_key.to_owned(),
        name: name.to_owned(),
        start_date: date(start),
        end_date: date(end),
        due: None,
        actual_start: None,
        actual_end: None,
        progress: 0,
        tags: String::new(),
        status: String::new(),
        assignee: String::new(),
        note: String::new(),
        waits: String::new(),
        targets: String::new(),
        color: String::new(),
        background: String::new(),
    }
}

impl State {
    fn new() -> Self {
        let first = sortkey::between(None, None);
        let second = sortkey::between(Some(&first), None);
        let third = sortkey::between(Some(&second), None);

        Self {
            revision: 0,
            next_id: 4,
            rows: vec![
                row("t-1", &first, "設計", "2026-10-01", "2026-10-09"),
                row("t-2", &second, "実装", "2026-10-12", "2026-10-30"),
                row("t-3", &third, "テスト", "2026-11-02", "2026-11-13"),
            ],
        }
    }

    /// The plan as one view of it sees it, worked out afresh.
    fn grid(&self, view: View) -> GridData {
        let mut rows: Vec<TaskRow> = self
            .rows
            .iter()
            .filter(|row| !(view.hide_done && row.progress >= 100))
            .cloned()
            .collect();
        rows.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));

        domain::build(
            "demo",
            self.revision,
            jiff::Zoned::now().date(),
            rows,
            // Everything else as it comes; but the grid is read-only until
            // the host says who is looking may write.
            Settings {
                can_edit: true,
                ..Settings::default()
            },
        )
    }

    /// The rows in the order they are drawn.
    fn ordered(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.rows.len()).collect();
        order.sort_by(|a, b| self.rows[*a].sort_key.cmp(&self.rows[*b].sort_key));
        order
    }

    /// What every write answers with: the new plan, and the row it was about.
    fn changed(&mut self, task_id: &str, view: View, note: Option<&'static str>) -> Mutation {
        if note.is_none() {
            self.revision += 1;
        }

        Mutation {
            grid: Some(self.grid(view)),
            patch: None,
            task_id: Some(task_id.to_owned()),
            note,
        }
    }
}

/// Which rows whoever is looking wants to see.
///
/// The grid does not know what this means. It was handed the query by the
/// page and hands it back on every request; reading it is this program's job.
#[derive(Clone, Copy, Default)]
struct View {
    hide_done: bool,
}

#[query_params(error = bad_request("hide_done は 0 か 1 です。"))]
struct ViewQuery {
    hide_done: Option<String>,
}

fn view(cx: &Cx) -> View {
    let hide_done = query_params::<ViewQuery>(cx)
        .ok()
        .and_then(|query| query.hide_done.clone())
        .is_some_and(|value| value == "1");

    View { hide_done }
}

fn plan(cx: &Cx) -> std::sync::MutexGuard<'_, State> {
    app_context::<Plan>(cx)
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

fn task_id(cx: &Cx) -> Result<String> {
    raw_path_params(cx)
        .iter()
        .find(|(key, _)| *key == "task_id")
        .map(|(_, value)| value.to_string())
        .ok_or_else(|| not_found().into())
}

fn file(content_type: &str, body: &'static str) -> Result<Response> {
    Ok(Response::builder()
        .header("Content-Type", content_type)
        .body(Body::from(body))?)
}

#[route(GET "/")]
async fn page(cx: &Cx) -> Result<Response> {
    let query = if view(cx).hide_done {
        r#" data-query="hide_done=1""#
    } else {
        ""
    };

    let html = format!(
        r##"<!doctype html>
<html lang="ja">
<meta charset="utf-8">
<title>minimal-host</title>
<link rel="stylesheet" href="/g/grid.css">
<style>
  body {{ margin: 0; font-family: system-ui, sans-serif; }}
  header {{ padding: 8px 16px; }}
  #fugantt-grid {{ height: calc(100vh - 48px); display: flex; flex-direction: column; }}
</style>
<header>minimal-host <a href="/">すべて</a> <a href="/?hide_done=1">終わったものを隠す</a> <span id="count"></span></header>
<div id="fugantt-grid" data-project="demo" data-api="{API}" data-filter-count="#count" data-row-link="/rows/{{id}}"{query}></div>
<script src="/g/grid.js" defer></script>
</html>"##
    );

    Ok(Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(html))?)
}

/// Where a row's link leads. A real host has the row's own page here.
#[route(GET "/rows/{task_id}")]
async fn row_page(cx: &Cx) -> Result<Response> {
    let id = task_id(cx)?;
    let plan = plan(cx);

    let Some(task) = plan.rows.iter().find(|row| row.id == id) else {
        return Err(not_found().into());
    };

    // The name is the only thing here somebody typed.
    let name = task
        .name
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let html = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>{name}</title>\
         <p><a href=\"/\">← 計画に戻る</a></p><h1>{name}</h1>"
    );

    Ok(Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(html))?)
}

#[route(GET "/g/grid.js")]
async fn grid_js(_cx: &Cx) -> Result<Response> {
    file("text/javascript; charset=utf-8", fu_gantt_web::GRID_JS)
}

#[route(GET "/g/grid.css")]
async fn grid_css(_cx: &Cx) -> Result<Response> {
    file("text/css; charset=utf-8", fu_gantt_web::GRID_CSS)
}

#[route(GET "/g/api/plans/demo/grid")]
async fn grid(cx: &Cx) -> Result<Json<GridData>> {
    Ok(Json(plan(cx).grid(view(cx))))
}

#[route(POST "/g/api/plans/demo/tasks")]
async fn insert(cx: &Cx, Json(insert): Json<InsertTask>) -> Result<Json<Mutation>> {
    let view = view(cx);
    let mut plan = plan(cx);
    let order = plan.ordered();

    // After the named row, or at the end.
    let at = insert
        .after
        .as_deref()
        .and_then(|after| order.iter().position(|row| plan.rows[*row].id == after));
    let (before, after) = match at {
        Some(at) => (Some(order[at]), order.get(at + 1).copied()),
        None => (order.last().copied(), None),
    };
    let key = sortkey::between(
        before.map(|row| plan.rows[row].sort_key.as_str()),
        after.map(|row| plan.rows[row].sort_key.as_str()),
    );

    let id = format!("t-{}", plan.next_id);
    plan.next_id += 1;
    plan.rows.push(row(&id, &key, "", "", ""));

    Ok(Json(plan.changed(&id, view, None)))
}

#[route(POST "/g/api/plans/demo/tasks/{task_id}")]
async fn edit(cx: &Cx, Json(edit): Json<CellEdit>) -> Result<Json<Mutation>> {
    let id = task_id(cx)?;
    let today = jiff::Zoned::now().date();
    let view = view(cx);
    let mut plan = plan(cx);

    // Read before anything is written, so a refusal leaves the row as it was.
    let read =
        |value: &str| text::date(value, today).map_err(|_| bad_request("日付として読めません。"));

    enum Write {
        Name(String),
        Start(Option<String>),
        End(Option<String>),
        Both(Option<String>, Option<String>),
        Progress(i64),
    }

    let write = match edit.field {
        CellField::Name => Write::Name(edit.value.trim().to_owned()),
        CellField::Start => Write::Start(read(&edit.value)?),
        CellField::End => Write::End(read(&edit.value)?),
        CellField::Schedule => {
            let (start, end) = edit.value.split_once('/').unwrap_or((&edit.value, ""));
            Write::Both(read(start)?, read(end)?)
        }
        CellField::Progress => Write::Progress(
            edit.value
                .trim()
                .trim_end_matches('%')
                .parse::<i64>()
                .ok()
                .filter(|percent| (0..=100).contains(percent))
                .ok_or_else(|| bad_request("進捗は0〜100で入力してください。"))?,
        ),
        _ => return Err(bad_request("この見本では変えられない列です。").into()),
    };

    let Some(task) = plan.rows.iter_mut().find(|row| row.id == id) else {
        return Err(not_found().into());
    };

    match write {
        Write::Name(name) => task.name = name,
        Write::Start(start) => task.start_date = start,
        Write::End(end) => task.end_date = end,
        Write::Both(start, end) => {
            task.start_date = start;
            task.end_date = end;
        }
        Write::Progress(percent) => task.progress = percent,
    }

    Ok(Json(plan.changed(&id, view, None)))
}

#[route(POST "/g/api/plans/demo/tasks/{task_id}/move")]
async fn reorder(cx: &Cx, Json(request): Json<MoveRequest>) -> Result<Json<Mutation>> {
    let id = task_id(cx)?;
    let view = view(cx);
    let mut plan = plan(cx);
    let order = plan.ordered();

    let Some(at) = order.iter().position(|row| plan.rows[*row].id == id) else {
        return Err(not_found().into());
    };

    let other = match request.action {
        Move::Up => at.checked_sub(1),
        Move::Down => (at + 1 < order.len()).then_some(at + 1),
        // A flat list: there is no outline to move through.
        Move::Indent | Move::Outdent => {
            return Ok(Json(plan.changed(
                &id,
                view,
                Some("この見本には階層がありません。"),
            )));
        }
    };

    let Some(other) = other else {
        return Ok(Json(plan.changed(
            &id,
            view,
            Some("これ以上は動かせません。"),
        )));
    };

    // Changing places is trading keys.
    let (a, b) = (order[at], order[other]);
    let key = plan.rows[a].sort_key.clone();
    plan.rows[a].sort_key = std::mem::replace(&mut plan.rows[b].sort_key, key);

    Ok(Json(plan.changed(&id, view, None)))
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let router = Router::builder()
        .app_context(Plan(Mutex::new(State::new())))
        .discover()
        .build();

    topcoat::start(router).await?;

    Ok(())
}
