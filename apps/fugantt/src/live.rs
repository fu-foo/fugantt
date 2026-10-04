//! Live updates between people editing the same project.
//!
//! Every mutation announces the revision it produced. Clients that are behind
//! refetch; the announcement itself carries no task data, so a stale client
//! cannot apply a change out of order.

use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
};

use tokio::sync::broadcast;
use topcoat::context::{Cx, app_context};

pub use fu_gantt_core::wire::{CELL, LiveChange as Change, PLAN};

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
