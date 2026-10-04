//! Live updates between people editing the same project.
//!
//! Every mutation announces the revision it produced. Clients that are behind
//! refetch; the announcement itself carries no task data, so a stale client
//! cannot apply a change out of order.

use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
};

use serde::Serialize;
use tokio::sync::broadcast;
use topcoat::context::{Cx, app_context};

/// What a client learns when someone else changes the project.
#[derive(Debug, Clone, Serialize)]
pub struct Change {
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

/// One broadcast channel per project, created on first use.
#[derive(Default)]
pub struct Hub {
    channels: Mutex<HashMap<String, broadcast::Sender<Change>>>,
}

/// How many changes a slow client can fall behind before it is dropped. It
/// reconnects and refetches, so the only cost of overflowing is one extra GET.
const BACKLOG: usize = 32;

impl Hub {
    fn channel(&self, project_id: &str) -> broadcast::Sender<Change> {
        self.channels
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(project_id.to_owned())
            .or_insert_with(|| broadcast::channel(BACKLOG).0)
            .clone()
    }

    /// Announces a change. Nobody listening is the normal case, not an error.
    pub fn publish(&self, project_id: &str, change: Change) {
        let _ = self.channel(project_id).send(change);
    }

    pub fn subscribe(&self, project_id: &str) -> broadcast::Receiver<Change> {
        self.channel(project_id).subscribe()
    }
}

/// Tells every open screen that the whole installation moved under it.
///
/// A restore replaces every project at once, so there is no one revision to
/// announce. Each project is told its own new one, and every screen refetches
/// — which is the right answer whether or not the plan it was showing is even
/// there any more.
pub async fn announce_everything(cx: &Cx) {
    let projects = sqlx::query_as::<_, (String, i64)>("SELECT id, revision FROM projects")
        .fetch_all(crate::db::pool(cx))
        .await
        .unwrap_or_default();

    for (id, revision) in projects {
        hub(cx).publish(
            &id,
            Change {
                revision,
                task_id: None,
                actor: "バックアップの復元".to_owned(),
                client: None,
                kind: PLAN,
            },
        );
    }
}

pub fn hub(cx: &Cx) -> &Hub {
    app_context(cx)
}
