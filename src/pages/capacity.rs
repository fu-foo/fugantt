//! Who has room, and who has none.
//!
//! The schedule answers "when is this due". The question asked over it, every
//! time, is "can 山田 take this" — and that is not on the screen anywhere. It
//! is answered today by scrolling the chart with a finger on the row and
//! counting bars, which is exactly the kind of arithmetic a person should not
//! be doing off a picture.
//!
//! Months, because that is the unit plans are argued in. Days, because a day is
//! either taken or it is not: anything finer needs a number on every task that
//! nobody would keep up to date, and a made-up number in a capacity table is
//! worse than none.

use jiff::civil::Date;
use topcoat::{
    Result,
    context::Cx,
    router::{page, query_params},
    view::view,
};

use crate::{auth::require_user, domain, project};

/// The stretch being asked about, as two months.
#[query_params(error = not_found())]
struct Window {
    from: Option<String>,
    to: Option<String>,
}

#[page("/projects/{project_id}/capacity")]
async fn index(cx: &Cx) -> Result {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let project = project::authorize(cx, &user.id, &project_id).await?;
    let l = crate::i18n::lang(cx).await;
    let data = project::grid_data(cx, &project).await?;

    let window = query_params::<Window>(cx)?;
    let today = data
        .today
        .parse::<Date>()
        .unwrap_or_else(|_| jiff::Zoned::now().date());

    let from_month = month(window.from.as_deref()).unwrap_or_else(|| first_of(today));
    let to_month = month(window.to.as_deref()).unwrap_or(from_month);
    let (from_month, to_month) = if to_month < from_month {
        (to_month, from_month)
    } else {
        (from_month, to_month)
    };

    let from = from_month;
    let to = last_of(to_month);

    let rows = domain::load(&data, from, to);
    let from_value = from.strftime("%Y-%m").to_string();
    let to_value = to.strftime("%Y-%m").to_string();
    let range = format!("{from} 〜 {to}");

    // The widest bar sets the scale, so a team with two-day plans and a team
    // with ninety-day plans both get a chart worth looking at.
    let widest = rows
        .iter()
        .map(|row| row.capacity.unwrap_or(0).max(row.busy))
        .max()
        .unwrap_or(0)
        .max(1);

    view! {
        <div class="mx-auto w-full max-w-4xl">
            <h1 class="text-2xl font-bold tracking-tight">(l.t("余力"))</h1>
            <p class="mt-1 text-sm text-slate-500">(&project.name)</p>

            <form method="GET" class="mt-6 flex flex-wrap items-end gap-3">
                <div class="flex flex-col gap-1">
                    <label for="from" class="text-xs font-medium text-slate-500">(l.t("開始月"))</label>
                    <input
                        id="from"
                        name="from"
                        type="month"
                        value=(&from_value)
                        class="rounded-lg border border-slate-300 px-3 py-2"
                    >
                </div>
                <div class="flex flex-col gap-1">
                    <label for="to" class="text-xs font-medium text-slate-500">(l.t("終了月"))</label>
                    <input
                        id="to"
                        name="to"
                        type="month"
                        value=(&to_value)
                        class="rounded-lg border border-slate-300 px-3 py-2"
                    >
                </div>
                <button class="rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-700">
                    (l.t("この期間で見る"))
                </button>
                <span class="text-xs text-slate-400">(&range)</span>
            </form>

            <section class="mt-6 overflow-hidden rounded-xl border border-slate-200 bg-white">
                <table class="w-full text-sm">
                    <thead class="bg-slate-50 text-xs text-slate-500">
                        <tr>
                            <th class="px-4 py-2 text-left font-medium">(l.t("担当者"))</th>
                            <th class="px-4 py-2 text-right font-medium">(l.t("稼働できる"))</th>
                            <th class="px-4 py-2 text-right font-medium">(l.t("埋まっている"))</th>
                            <th class="px-4 py-2 text-right font-medium">(l.t("空き"))</th>
                            <th class="px-4 py-2 text-right font-medium">(l.t("重なり"))</th>
                            <th class="w-1/3 px-4 py-2 text-left font-medium"></th>
                        </tr>
                    </thead>
                    <tbody>
                        for row in &rows {
                            // A person with nothing in this window is noise on a
                            // page about who is free — unless they are the row
                            // for work nobody is holding, which is the point.
                            if row.tasks > 0 || row.capacity.is_some() {
                                <tr class="border-t border-slate-100">
                                    <td class="px-4 py-2">
                                        if row.assignee.is_empty() {
                                            <span class="text-slate-400">(l.t("（未割当）"))</span>
                                        } else {
                                            (&row.assignee)
                                        }
                                    </td>
                                    <td class="px-4 py-2 text-right tabular-nums text-slate-500">
                                        (&days(row.capacity))
                                    </td>
                                    <td class="px-4 py-2 text-right tabular-nums">(&format!("{}日", row.busy))</td>
                                    <td
                                        class=(
                                            if row.free.is_some_and(|free| free < 0) {
                                                "px-4 py-2 text-right font-semibold tabular-nums text-red-600"
                                            } else {
                                                "px-4 py-2 text-right font-semibold tabular-nums"
                                            }
                                        )
                                    >
                                        (&days(row.free))
                                    </td>
                                    <td
                                        class=(
                                            if row.overlap > 0 {
                                                "px-4 py-2 text-right tabular-nums text-amber-600"
                                            } else {
                                                "px-4 py-2 text-right tabular-nums text-slate-300"
                                            }
                                        )
                                    >
                                        if row.overlap > 0 {
                                            (&format!("{}日", row.overlap))
                                        } else {
                                            "—"
                                        }
                                    </td>
                                    <td class="px-4 py-2">
                                        <div class="h-3 w-full overflow-hidden rounded-full bg-slate-100">
                                            <div
                                                class=(
                                                    if row.free.is_some_and(|free| free < 0) {
                                                        "h-full rounded-full bg-red-500"
                                                    } else {
                                                        "h-full rounded-full bg-blue-500"
                                                    }
                                                )
                                                style=(&format!("width: {}%", (row.busy * 100 / widest).min(100)))
                                            ></div>
                                        </div>

                                        // Which days, not just how many. "9日空いている"
                                        // is the answer to half a question.
                                        if !row.free_spans.is_empty() {
                                            <p class="mt-1 text-xs text-slate-500">
                                                (l.t("空き")) ": " (&spans(&row.free_spans))
                                            </p>
                                        }
                                        if !row.overlap_spans.is_empty() {
                                            <p class="mt-0.5 text-xs text-amber-600">
                                                (l.t("重なり")) ": " (&spans(&row.overlap_spans))
                                            </p>
                                        }
                                    </td>
                                </tr>
                            }
                        }
                    </tbody>
                </table>
            </section>

            <p class="mt-3 text-xs text-slate-400">
                (l.t("同じ日に2つのタスクがあっても、その日は1日と数えます（何重になっているかは「重なり」に出ます）。終わったタスクと集計行は数えません。休暇はその人の稼働から引きます。"))
            </p>
        </div>
    }
}

/// `8/1〜8/5, 8/20` — the days themselves, short enough to sit under a bar.
///
/// A month can only hold so many stretches, so they are all listed: a cut-off
/// list would hide exactly the gap somebody is looking for.
fn spans(spans: &[crate::domain::Span]) -> String {
    spans
        .iter()
        .map(|span| {
            let from = short(&span.start);
            let to = short(&span.end);

            if from == to {
                from
            } else {
                format!("{from}〜{to}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `2026-08-05` as `8/5`: the year is on the page already.
fn short(iso: &str) -> String {
    let mut parts = iso.split('-');
    let (_, month, day) = (parts.next(), parts.next(), parts.next());

    match (month, day) {
        (Some(month), Some(day)) => {
            format!(
                "{}/{}",
                month.trim_start_matches('0'),
                day.trim_start_matches('0')
            )
        }
        _ => iso.to_owned(),
    }
}

/// `12日`, or a dash where the question does not apply.
fn days(value: Option<i64>) -> String {
    value.map_or_else(|| "—".to_owned(), |days| format!("{days}日"))
}

/// Reads `2026-08` from a month field.
fn month(value: Option<&str>) -> Option<Date> {
    let value = value?.trim();
    let (year, month) = value.split_once('-')?;

    Date::new(year.parse().ok()?, month.parse().ok()?, 1).ok()
}

fn first_of(date: Date) -> Date {
    Date::new(date.year(), date.month(), 1).unwrap_or(date)
}

fn last_of(month: Date) -> Date {
    month.last_of_month()
}
