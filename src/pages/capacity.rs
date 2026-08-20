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
    view::{component, view},
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

    let rows = domain::load(&data, from, to, today);
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
            <h1 class="text-2xl font-bold tracking-tight">(l.t("空き検索"))</h1>
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

            table(rows: &rows, widest: widest, l: l)

            <p class="mt-3 text-xs text-slate-400">
                (l.t("今日から先を数えます。同じ日に複数のタスクがあっても1日と数え、その重なりは「重複」に出ます。終わったタスクと集計行は数えません。休暇は稼働可能日数から引きます。"))
            </p>
        </div>
    }
}

/// The same question, asked of every plan at once.
///
/// Per project, somebody on three of them looks three times as free as they
/// are: each page can only see its own bars. This one counts them together,
/// which is the number anybody actually wants before handing out work.
///
/// The calendar here is the company's — shared holidays, weekends, and the
/// person's own leave. A single project's shading is that project's business
/// and cannot speak for the others.
#[page("/capacity")]
async fn everywhere(cx: &Cx) -> Result {
    let user = require_user(cx).await?;
    let l = crate::i18n::lang(cx).await;

    let (rows, projects) = project::tasks_everywhere(cx, &user.id).await?;
    let today = jiff::Zoned::now().date();

    let data = domain::build(
        "",
        0,
        today,
        rows,
        domain::Settings {
            holidays: project::app_holidays(cx).await?,
            leaves: project::all_leaves(cx).await?,
            assignees: project::assignees_everywhere(cx, &user.id).await?,
            ..domain::Settings::default()
        },
    );

    let window = query_params::<Window>(cx)?;
    let from_month = month(window.from.as_deref()).unwrap_or_else(|| first_of(today));
    let to_month = month(window.to.as_deref()).unwrap_or(from_month);
    let (from_month, to_month) = if to_month < from_month {
        (to_month, from_month)
    } else {
        (from_month, to_month)
    };

    let from = from_month;
    let to = last_of(to_month);

    let rows = domain::load(&data, from, to, today);
    let from_value = from.strftime("%Y-%m").to_string();
    let to_value = to.strftime("%Y-%m").to_string();
    let range = format!("{from} 〜 {to}");
    let counted = format!("{projects}{}", l.t("件のプロジェクトを合わせて"));

    let widest = rows
        .iter()
        .map(|row| row.capacity.unwrap_or(0).max(row.busy))
        .max()
        .unwrap_or(0)
        .max(1);

    view! {
        <div class="mx-auto w-full max-w-4xl">
            <h1 class="text-2xl font-bold tracking-tight">(l.t("全体の空き検索"))</h1>
            <p class="mt-1 text-sm text-slate-500">(&counted)</p>

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

            table(rows: &rows, widest: widest, l: l)

            <p class="mt-3 text-xs text-slate-400">
                (l.t("自分が開けるプロジェクトをすべて合わせて数えます。休みの日は会社の暦（土日と全体の設定の祝日）と、その人の休暇です。プロジェクトごとの暦の違いはここでは使いません。"))
            </p>
        </div>
    }
}

/// The table itself, shared by the project's own page and the one that reads
/// across them all.
///
/// One arithmetic and one set of columns: the same question answered in two
/// shapes is how a number ends up being argued with.
#[component]
async fn table(rows: &[domain::Load], widest: i64, l: crate::i18n::Lang) -> Result {
    view! {
        <section class="mt-6 overflow-hidden rounded-xl border border-slate-200 bg-white">
            <table class="w-full text-sm">
                <thead class="bg-slate-50 text-xs text-slate-500">
                    <tr>
                        <th class="px-4 py-2 text-left font-medium">(l.t("担当者"))</th>
                        <th class="px-4 py-2 text-right font-medium">(l.t("稼働可能日数"))</th>
                        <th class="px-4 py-2 text-right font-medium">(l.t("経過済"))</th>
                        <th class="px-4 py-2 text-right font-medium">(l.t("割当済"))</th>
                        <th class="px-4 py-2 text-right font-medium">(l.t("空き日数"))</th>
                        <th class="px-4 py-2 text-right font-medium">(l.t("重複"))</th>
                        <th class="w-1/3 px-4 py-2 text-left font-medium"></th>
                    </tr>
                </thead>
                <tbody>
                    for row in rows {
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
                                    (&days(row.capacity, l))
                                </td>
                                <td class="px-4 py-2 text-right tabular-nums text-slate-400">
                                    (&days(row.gone, l))
                                </td>
                                <td class="px-4 py-2 text-right tabular-nums">(&count(row.busy, l))</td>
                                <td
                                    class=(
                                        if row.free.is_some_and(|free| free < 0) {
                                            "px-4 py-2 text-right font-semibold tabular-nums text-red-600"
                                        } else {
                                            "px-4 py-2 text-right font-semibold tabular-nums"
                                        }
                                    )
                                >
                                    (&days(row.free, l))
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
                                        (&count(row.overlap, l))
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
                                            (l.t("空き日数")) ": " (&spans(&row.free_spans, l))
                                        </p>
                                    }
                                    if !row.overlap_spans.is_empty() {
                                        <p class="mt-0.5 text-xs text-amber-600">
                                            (l.t("重複")) ": " (&spans(&row.overlap_spans, l))
                                        </p>
                                    }
                                </td>
                            </tr>
                        }
                    }
                </tbody>
            </table>
        </section>
    }
}

/// `8/1〜8/5, 8/20` — the days themselves, short enough to sit under a bar.
///
/// A month can only hold so many stretches, so they are all listed: a cut-off
/// list would hide exactly the gap somebody is looking for.
fn spans(spans: &[crate::domain::Span], l: crate::i18n::Lang) -> String {
    spans
        .iter()
        .map(|span| {
            let from = l.short_date(&span.start);
            let to = l.short_date(&span.end);

            if from == to {
                from
            } else {
                format!("{from}{}{to}", l.to_())
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `12日` or `12d`, and a dash where the question does not apply.
fn days(value: Option<i64>, l: crate::i18n::Lang) -> String {
    value.map_or_else(|| "—".to_owned(), |days| count(days, l))
}

/// A number of days, with the unit the reader counts in.
fn count(days: i64, l: crate::i18n::Lang) -> String {
    match l {
        crate::i18n::Lang::Ja => format!("{days}日"),
        crate::i18n::Lang::En => format!("{days}d"),
    }
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
