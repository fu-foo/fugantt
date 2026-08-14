use std::collections::BTreeMap;

use topcoat::{Result, context::Cx, router::page, view::view};

use crate::{auth::require_user, domain::TaskView, project};

/// One status in a breakdown: name, colour, count.
type Tally = (String, String, usize);

/// What the plan says about itself.
///
/// The one number worth the page is the split of the delay: a project that is
/// late because it was waiting on someone else is a different problem from one
/// that is late because the work took longer, and the two need different
/// conversations.
#[page("/projects/{project_id}/stats")]
async fn index(cx: &Cx) -> Result {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let project = project::authorize(cx, &user.id, &project_id).await?;
    let l = crate::i18n::lang(cx).await;
    let data = project::grid_data(cx, &project).await?;

    // Summary rows are sums of their children, so counting them too would count
    // the same work twice.
    let leaves: Vec<&TaskView> = data.tasks.iter().filter(|task| !task.has_children).collect();

    let total = leaves.len();
    let delayed = leaves.iter().filter(|task| task.delayed).count();
    let progress = if total == 0 {
        0
    } else {
        leaves.iter().map(|task| task.progress).sum::<i64>() / total as i64
    };

    let mut by_status: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_assignee: BTreeMap<&str, (usize, i64)> = BTreeMap::new();
    // What each person is carrying. A count and an average cannot say whether
    // those three tasks are nearly done or not started.
    let mut breakdown: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();

    for task in &leaves {
        *by_status.entry(task.status.as_str()).or_default() += 1;

        let assignee = if task.assignee.is_empty() {
            l.t("（未割当）")
        } else {
            task.assignee.as_str()
        };
        let entry = by_assignee.entry(assignee).or_default();
        entry.0 += 1;
        entry.1 += task.progress;

        *breakdown
            .entry(assignee)
            .or_default()
            .entry(task.status.as_str())
            .or_default() += 1;
    }

    // In the order the statuses are configured: that order is how work moves
    // through this particular workplace.
    let order: Vec<&str> = data.statuses.iter().map(|status| status.name.as_str()).collect();
    let colour = |name: &str| {
        data.statuses
            .iter()
            .find(|status| status.name == name)
            .map(|status| status.color.clone())
            .unwrap_or_default()
    };

    let ordered = |counts: &BTreeMap<&str, usize>| -> Vec<Tally> {
        let mut rows: Vec<Tally> = Vec::new();

        for name in &order {
            if let Some(count) = counts.get(name) {
                rows.push(((*name).to_owned(), colour(name), *count));
            }
        }

        // A status no longer configured — an old name — is still shown.
        for (name, count) in counts {
            if !order.contains(name) {
                rows.push(((*name).to_owned(), String::new(), *count));
            }
        }

        rows
    };

    let per_person: Vec<(&str, Vec<Tally>)> = breakdown
        .iter()
        .map(|(name, counts)| (*name, ordered(counts)))
        .collect();

    let late_days = delay(&leaves);

    // Waiting is counted apart from lateness. Variance already excludes it, so
    // the two added together are how far the plan actually slipped.
    let wait_days: i64 = leaves.iter().map(|task| task.wait_days).sum();

    let mut by_reason: BTreeMap<&str, i64> = BTreeMap::new();
    for task in &leaves {
        for wait in &task.waits {
            let reason = if wait.reason.is_empty() {
                l.t("（理由なし）")
            } else {
                wait.reason.as_str()
            };

            *by_reason.entry(reason).or_default() += wait.days;
        }
    }

    let slipped = late_days + wait_days;

    let total_text = total.to_string();
    let progress_text = format!("{progress}%");
    let delayed_text = delayed.to_string();
    let late_text = format!("{late_days}日");
    let wait_text = format!("{wait_days}日");
    let slipped_text = format!("{slipped}日");

    view! {
        <div class="mx-auto w-full max-w-4xl">
            <h1 class="text-2xl font-bold tracking-tight">"統計"</h1>
            <p class="mt-1 text-sm text-slate-500">(&project.name)</p>

            <div class="mt-6 grid gap-4 sm:grid-cols-4">
                tile(label: "タスク", value: &total_text, tone: "")
                tile(label: "平均進捗", value: &progress_text, tone: "")
                tile(
                    label: "遅延中",
                    value: &delayed_text,
                    tone: if delayed > 0 { "late" } else { "" },
                )
                tile(
                    label: "作業の遅れ",
                    value: &late_text,
                    tone: if late_days > 0 { "late" } else { "" },
                )
            </div>

            // --- what the plan slipped by, and why -----------------------------

            <section class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">"ずれの内訳"</h2>
                <p class="mt-1 text-sm text-slate-500">
                    (l.t("差異は待ちを除いて数えているので、作業の遅れと待ちを足したものが実際のずれです。"))
                </p>

                if slipped == 0 {
                    <p class="mt-4 text-sm text-slate-500">"予定からずれているタスクはありません。"</p>
                } else {
                    <ul class="mt-4 flex flex-col gap-2 text-sm">
                        <li class="flex items-center gap-2 font-medium">
                            (l.t("ずれ"))
                            <span class="ml-auto tabular-nums">(&slipped_text)</span>
                        </li>
                        <li class="flex items-center gap-2 pl-4 text-slate-600">
                            <span class="size-3 rounded-sm bg-red-400"></span>
                            (l.t("作業の遅れ"))
                            <span class="ml-auto tabular-nums">(&late_text)</span>
                        </li>
                        <li class="flex items-center gap-2 pl-4 text-slate-600">
                            <span class="size-3 rounded-sm bg-violet-400"></span>
                            (l.t("待ち"))
                            <span class="ml-auto tabular-nums">(&wait_text)</span>
                        </li>

                        for (reason, days) in &by_reason {
                            if *days > 0 {
                                <li class="flex items-center gap-2 pl-10 text-xs text-slate-500">
                                    (reason)
                                    <span class="ml-auto tabular-nums">(&format!("{days}日"))</span>
                                </li>
                            }
                        }
                    </ul>
                }
            </section>

            // --- status -----------------------------------------------------

            <section class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">"ステータス"</h2>

                <ul class="mt-4 flex flex-col gap-2">
                    for (status, count) in &by_status {
                        <li class="flex items-center gap-3 text-sm">
                            <span class="w-20 shrink-0">(status)</span>
                            <span class="h-4 rounded-sm bg-slate-300" style=(("width:", &format!("{}px", count * 220 / total.max(1))))></span>
                            <span class="tabular-nums text-slate-500">(count.to_string())</span>
                        </li>
                    }
                </ul>
            </section>

            // --- assignees --------------------------------------------------

            <section class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">"担当者"</h2>

                <ul class="mt-4 flex flex-col divide-y divide-slate-100">
                    for (name, (count, sum)) in &by_assignee {
                        <li class="flex flex-wrap items-center gap-x-3 gap-y-2 py-2.5 text-sm">
                            <span class="w-28 shrink-0 truncate">(name)</span>
                            <span class="tabular-nums text-slate-500">(count.to_string())"件"</span>

                            // Beside the count, not in a table of its own:
                            // otherwise reading it means matching two lists by eye.
                            <span class="flex flex-wrap items-center gap-1.5">
                                for (status, tint, hits) in
                                    per_person.iter().find(|(who, _)| who == name)
                                        .map(|(_, rows)| rows.clone()).unwrap_or_default()
                                {
                                    <span
                                        class="rounded-full px-2 py-0.5 text-xs"
                                        style=((
                                            "background:",
                                            if tint.is_empty() { "#f1f5f9" } else { &tint },
                                        ))
                                    >
                                        (&status)
                                        <span class="ml-1 tabular-nums font-medium">
                                            (hits.to_string())
                                        </span>
                                    </span>
                                }
                            </span>

                            <span class="ml-auto tabular-nums text-slate-500">
                                (l.t("平均 "))
                                ((sum / *count as i64).to_string())
                                "%"
                            </span>
                        </li>
                    }
                </ul>
            </section>
        </div>
    }
}

/// The days a plan ran past itself.
///
/// A finished task knows exactly how late it was. One that is still running is
/// late by however long it has already overrun, which is the number people
/// actually want on a Monday morning. Both come from the domain, counted in the
/// days this project counts.
fn delay(tasks: &[&TaskView]) -> i64 {
    tasks
        .iter()
        .map(|task| match task.end_variance {
            Some(variance) => variance.max(0),
            None => task.overdue.max(0),
        })
        .sum()
}

#[topcoat::view::component]
async fn tile(label: &str, value: &str, tone: &str) -> Result {
    view! {
        <div class="rounded-xl border border-slate-200 bg-white p-4">
            <p class="text-xs text-slate-500">(label)</p>
            <p
                class=(
                    if tone == "late" {
                        "mt-1 text-2xl font-bold tabular-nums text-red-600"
                    } else {
                        "mt-1 text-2xl font-bold tabular-nums"
                    }
                )
            >
                (value)
            </p>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(end_variance: Option<i64>, overdue: i64) -> TaskView {
        TaskView {
            end: Some("2026-08-14".to_owned()),
            end_variance,
            overdue,
            ..TaskView::default()
        }
    }

    #[test]
    fn finished_late_counts_its_variance() {
        let rows = [task(Some(4), 0)];
        let leaves: Vec<&TaskView> = rows.iter().collect();

        assert_eq!(delay(&leaves), 4);
    }

    /// The interesting case: a task that has not finished is late by what it
    /// has already overrun, not by nothing at all.
    #[test]
    fn unfinished_counts_its_overrun_so_far() {
        let rows = [task(None, 6)];
        let leaves: Vec<&TaskView> = rows.iter().collect();

        assert_eq!(delay(&leaves), 6);
    }

    #[test]
    fn on_time_work_counts_for_nothing() {
        let rows = [task(Some(-2), 0), task(None, 0)];
        let leaves: Vec<&TaskView> = rows.iter().collect();

        assert_eq!(delay(&leaves), 0);
    }
}
