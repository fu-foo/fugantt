//! What the grid and its host say to each other.
//!
//! The grid is one script that talks JSON to about ten addresses. These are
//! the shapes it sends and the shapes it expects back, so a host can be held
//! to them by the compiler rather than by reading the script.
//!
//! `docs/gantt-api.md` says which address takes which.

use serde::{Deserialize, Serialize};

use crate::domain::{GridData, TaskView};

/// Which saved set of filter conditions to throw away.
#[derive(Debug, Deserialize)]
pub struct RemoveFilterSet {
    pub id: String,
}

/// What a mutation gives back: the new state, and the row it concerns.
#[derive(Debug, Serialize)]
pub struct Mutation {
    /// The whole plan. Sent by the writes that can move anything anywhere —
    /// reordering, importing, a change to the calendar or the columns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grid: Option<GridData>,
    /// What one ordinary write changed, which is a handful of rows however
    /// long the plan is. See [`Patch`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Patch>,
    pub task_id: Option<String>,
    /// Why a request that succeeded still changed nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<&'static str>,
}

/// What a write changed, rather than the plan it changed it in.
///
/// Writing one cell used to answer with every row there was, because the
/// server owns the derived numbers and the caller cannot know how far a value
/// carries. It carries exactly as far as the summary rows above it: a date
/// moves its parent's dates, and its parent's parent's, and stops. So that is
/// what comes back — three or four rows on a plan of ten thousand, instead of
/// four megabytes of JSON that the browser then has to read.
#[derive(Debug, Serialize)]
pub struct Patch {
    pub revision: i64,
    /// The row that was written to, and the summary rows above it.
    pub rows: Vec<TaskView>,
    /// Where a new row belongs: the id it comes after, or `None` for the top.
    /// Only a write that adds a row sets this.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    /// A row that changed places, with its subtree following it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moved: Option<Moved>,
    /// Rows that are gone, subtree and all.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<String>,
    /// The chart's window, which a date can push outwards.
    pub range_start: String,
    pub range_end: String,
    /// How many rows the plan has now.
    ///
    /// The browser compares this with what it holds. If they disagree it has
    /// missed something, and it asks for the whole plan rather than drawing
    /// numbers it cannot vouch for.
    pub total: usize,
}

/// Where a row ended up after being moved.
///
/// The browser holds the plan as one flat list, so a subtree is the row plus
/// the run of deeper rows behind it. Told which row it now follows and how
/// deep it now sits, the browser can cut that run out and put it back — no
/// need to be sent the plan to find out what order it is in.
#[derive(Debug, Serialize)]
pub struct Moved {
    pub id: String,
    /// The row it comes after, or `None` for the top of the plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    pub depth: usize,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellField {
    Name,
    Start,
    End,
    /// 納期: the day this was promised for. Not the plan's end — the plan says
    /// which days are booked, this says which day was named to somebody else.
    Due,
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
    /// The row's own colours, `#rrggbb` or empty to take them off.
    Color,
    Background,
    /// One of the project's own columns, named by `field_id`.
    Custom,
    /// Both dates at once, as `START/END`. Dragging a bar moves them together,
    /// and sending them apart would put the row through an invalid state.
    Schedule,
    /// 実施開始 / 実施終了 together, for the same reason.
    ActualSchedule,
}

#[derive(Debug, Deserialize)]
pub struct CellEdit {
    pub field: CellField,
    /// Which project-defined column, when `field` is `custom`.
    pub field_id: Option<String>,
    /// The raw cell text. An empty string clears a date.
    pub value: String,
    /// What the sender believes is there now, as it is stored.
    ///
    /// Undo sends this. Putting a value back is only right if the value it is
    /// putting back is still the one that replaced it — otherwise the person
    /// pressing Ctrl+Z would silently throw away somebody else's work, which is
    /// the one thing an undo must never do. Absent means "write it regardless",
    /// which is what an ordinary edit means.
    pub expect: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InsertTask {
    /// Insert below this row, keeping it among the same siblings. `None`
    /// appends to the top level.
    pub after: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MoveRequest {
    pub action: Move,
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

#[derive(Debug, Deserialize)]
pub struct PlaceRequest {
    /// The new parent, or `None` for the top level.
    pub parent: Option<String>,
    /// The sibling it lands after, or `None` to become the first child.
    pub after: Option<String>,
}

/// The columns: which are shown, how wide, and in what order.
///
/// Their own form rather than part of the view one, because the ↑↓ buttons have
/// to live beside the fields they reorder — and a form cannot nest in a form.
/// Pressing ↑ submits the same form, so the widths typed alongside are saved
/// rather than discarded.
/// A named set of conditions, as the island writes it.
#[derive(Debug, Deserialize)]
pub struct SaveFilterSet {
    pub name: String,
    pub conditions: String,
    /// Everybody's, or only mine.
    pub shared: bool,
}

#[derive(Debug, Deserialize)]
pub struct LeaveList {
    pub leaves: Vec<LeaveEntry>,
}

#[derive(Debug, Deserialize)]
pub struct LeaveEntry {
    pub assignee: String,
    pub start: String,
    pub end: String,
    #[serde(default)]
    pub note: String,
    /// `off` for a day away, `on` for a day worked regardless.
    #[serde(default)]
    pub kind: String,
}

/// What a client learns when someone else changes the project.
#[derive(Debug, Clone, Serialize)]
pub struct LiveChange {
    pub revision: i64,
    /// The row that changed, so the sender can recognise its own echo.
    pub task_id: Option<String>,
    /// Who made it, to show in the UI.
    pub actor: String,
    /// The browser that made it.
    ///
    /// A change is published before its response reaches the client that asked
    /// for it, so comparing revisions cannot tell an echo from someone else's
    /// edit. The originator recognises itself here and ignores the event.
    pub client: Option<String>,
    /// What shape of change it was: [`CELL`] or [`PLAN`].
    ///
    /// A watcher can take one row's numbers on trust and ask for just that
    /// row. It cannot take an order it did not see, so anything that moves
    /// rows about sends it back for the whole plan.
    pub kind: &'static str,
}

/// One row's values changed. Its ancestors' numbers followed, and nothing else.
pub const CELL: &str = "cell";

/// Rows arrived, left, or changed places.
pub const PLAN: &str = "plan";

#[cfg(test)]
mod tests {
    use super::*;

    /// The grid writes these by hand, so the names are a promise.
    #[test]
    fn the_grid_s_requests_are_read() {
        let edit: CellEdit =
            serde_json::from_str(r#"{"field":"actual_start","value":"8/5"}"#).unwrap();
        assert!(matches!(edit.field, CellField::ActualStart));
        assert_eq!(edit.value, "8/5");
        assert_eq!(edit.field_id, None);
        assert_eq!(edit.expect, None);

        let edit: CellEdit =
            serde_json::from_str(r#"{"field":"custom","field_id":"f-1","value":"x","expect":"y"}"#)
                .unwrap();
        assert!(matches!(edit.field, CellField::Custom));
        assert_eq!(edit.field_id.as_deref(), Some("f-1"));
        assert_eq!(edit.expect.as_deref(), Some("y"));

        let insert: InsertTask = serde_json::from_str(r#"{"after":null}"#).unwrap();
        assert_eq!(insert.after, None);

        let request: MoveRequest = serde_json::from_str(r#"{"action":"outdent"}"#).unwrap();
        assert!(matches!(request.action, Move::Outdent));

        let place: PlaceRequest = serde_json::from_str(r#"{"parent":"t-1","after":null}"#).unwrap();
        assert_eq!(place.parent.as_deref(), Some("t-1"));

        let leaves: LeaveList =
            serde_json::from_str(r#"{"leaves":[{"assignee":"佐藤","start":"9/7","end":"9/9"}]}"#)
                .unwrap();
        assert_eq!(leaves.leaves[0].note, "");
        assert_eq!(leaves.leaves[0].kind, "");
    }

    /// What is absent is left out rather than sent as null: the grid tells a
    /// patch from a whole plan by which key is there.
    #[test]
    fn an_answer_leaves_out_what_it_does_not_carry() {
        let answer = Mutation {
            grid: None,
            patch: None,
            task_id: Some("t-1".to_owned()),
            note: None,
        };

        assert_eq!(
            serde_json::to_string(&answer).unwrap(),
            r#"{"task_id":"t-1"}"#
        );

        let change = LiveChange {
            revision: 3,
            task_id: None,
            actor: "山田".to_owned(),
            client: None,
            kind: CELL,
        };

        assert_eq!(
            serde_json::to_string(&change).unwrap(),
            r#"{"revision":3,"task_id":null,"actor":"山田","client":null,"kind":"cell"}"#
        );
    }
}
