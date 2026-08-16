//! Task rollup and schedule arithmetic.
//!
//! This is the single place the derived numbers are computed. The grid in the
//! browser renders what it is given and never recomputes any of it, so parent
//! totals and delay flags cannot drift between the two languages.

use std::collections::HashMap;

use jiff::civil::Date;
use serde::Serialize;
use sqlx::FromRow;

/// A task exactly as it is stored.
#[derive(Debug, Clone, FromRow)]
pub struct TaskRow {
    pub id: String,
    pub parent_id: Option<String>,
    pub sort_key: String,
    pub name: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub actual_start: Option<String>,
    pub actual_end: Option<String>,
    pub progress: i64,
    pub tags: String,
    pub status: String,
    pub assignee: String,
    pub note: String,
    /// Waiting periods: `YYYY-MM-DD/YYYY-MM-DD` per line.
    pub waits: String,
    /// 予定進捗: `YYYY-MM-DD/PERCENT` per line.
    pub targets: String,
    /// The colours this row was given, as `#rrggbb`, or empty for neither.
    pub color: String,
    pub background: String,
}

/// A task as the grid receives it: depth-first order, derived values resolved.
///
/// `Default` is for tests, which usually care about two fields out of thirty.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TaskView {
    pub id: String,
    pub depth: usize,
    pub name: String,
    /// The plan.
    pub start: Option<String>,
    pub end: Option<String>,
    /// What actually happened, when it is known.
    pub actual_start: Option<String>,
    pub actual_end: Option<String>,
    pub progress: i64,
    /// Inclusive day count of the plan, `None` when it has no dates yet.
    pub days: Option<i64>,
    pub actual_days: Option<i64>,
    /// 実施開始 − 予定開始, in days. Positive means it started late.
    pub start_variance: Option<i64>,
    /// 実施終了 − 予定終了, in days. Positive means it finished late.
    pub end_variance: Option<i64>,
    /// How far past its planned end an unfinished task already is, today.
    pub overdue: i64,
    /// Waiting periods. Days inside them are not counted against the task.
    pub waits: Vec<Span>,
    /// How many countable days the waits took out.
    pub wait_days: i64,
    pub status: String,
    pub assignee: String,
    pub note: String,
    /// The waiting spell, when there is one. Kept plain: the state is the core
    /// and everything about it is optional.
    /// Days spent waiting, for the share of a delay that was not work.
    /// 予定進捗: the checkpoints this plan names, in date order.
    pub targets: Vec<Target>,
    /// What the plan says should be done by now: the percent of the last
    /// checkpoint whose date has passed. `None` when the plan names none, or
    /// none has come round yet — and then nothing is judged.
    pub expected: Option<i64>,
    /// Behind a checkpoint the plan itself names. A row that names none is
    /// never behind — nothing was promised, so nothing was missed. Running past
    /// the planned end is a separate fact, and stays in `overdue`.
    pub delayed: bool,
    /// The colours this row was given. Empty means the row looks like a row.
    pub color: String,
    pub background: String,
    /// Parents are read-only in the grid: their dates and progress are sums.
    pub has_children: bool,
    pub tags: Vec<String>,
    /// Values for the project's own fields, keyed by field id.
    pub values: HashMap<String, String>,
}

/// The whole payload the project page hands to the grid.
#[derive(Debug, Clone, Serialize)]
pub struct GridData {
    pub project_id: String,
    pub revision: i64,
    /// The language the island draws in. The server decides; the island only
    /// draws what it is handed.
    pub language: String,
    pub today: String,
    /// The date window the chart draws, padded out from the task dates.
    pub range_start: String,
    pub range_end: String,
    /// Days the chart shades, with the names to show on hover.
    pub holidays: Vec<Holiday>,
    /// Days each assignee is away.
    pub leaves: Vec<Leave>,
    /// The names the 担当者 column offers.
    pub assignees: Vec<Assignee>,
    /// The states the ステータス column offers.
    pub statuses: Vec<Status>,
    /// The colours the chart draws bars in.
    pub theme: Theme,
    /// The project's own columns, after the built-in ones.
    pub fields: Vec<Field>,
    /// Built-in columns the project chose to hide.
    pub hidden_columns: Vec<String>,
    /// The order the columns are shown in. Anything missing keeps its place at
    /// the end, so a new column never disappears because the list is old.
    pub column_order: Vec<String>,
    /// Columns a bar repeats in its tooltip. May name a hidden column: taking
    /// 製品 off the table and still being able to ask a bar about it is most of
    /// why this exists.
    pub tooltip_columns: Vec<String>,
    /// Saved filters: the team's, then this person's own.
    pub filter_sets: Vec<FilterSet>,
    /// Widths in pixels, by column key. Absent means the column sizes itself.
    pub column_widths: HashMap<String, u32>,
    /// How many columns stay put when the table scrolls sideways.
    pub frozen_columns: usize,
    /// Which days the count leaves out.
    pub counting: Counting,
    /// The month a business year starts in — 4 in most of Japan.
    pub fiscal_year_start: u32,
    /// Whether years are written as 令和8年 rather than 2026年.
    pub japanese_era: bool,
    /// Whether the chart carries the 年度・四半期 band above the months.
    pub quarters: bool,
    /// The era table, newest first.
    pub eras: Vec<Era>,
    /// Width of one day column, in pixels.
    pub day_width: u32,
    /// Whether the viewer may change anything, so the grid can go read-only.
    pub can_edit: bool,
    pub tasks: Vec<TaskView>,
}

impl GridData {
    /// An empty grid, for tests to build on.
    #[cfg(test)]
    pub fn empty(project_id: &str, revision: i64) -> Self {
        Self {
            project_id: project_id.to_owned(),
            revision,
            language: "ja".to_owned(),
            today: "2026-08-12".to_owned(),
            range_start: "2026-08-01".to_owned(),
            range_end: "2026-08-31".to_owned(),
            holidays: Vec::new(),
            leaves: Vec::new(),
            assignees: Vec::new(),
            statuses: Status::defaults(),
            theme: Theme::default(),
            fields: Vec::new(),
            hidden_columns: Vec::new(),
            column_order: Vec::new(),
            tooltip_columns: Vec::new(),
            filter_sets: Vec::new(),
            column_widths: HashMap::new(),
            frozen_columns: 1,
            counting: Counting::default(),
            fiscal_year_start: 4,
            japanese_era: false,
            quarters: true,
            eras: Vec::new(),
            day_width: 26,
            can_edit: true,
            tasks: Vec::new(),
        }
    }
}

/// Everything about the project that is not a task.
#[derive(Debug, Clone)]
pub struct Settings {
    pub holidays: Vec<Holiday>,
    pub leaves: Vec<Leave>,
    pub assignees: Vec<Assignee>,
    pub statuses: Vec<Status>,
    pub fiscal_year_start: u32,
    pub japanese_era: bool,
    pub quarters: bool,
    pub eras: Vec<Era>,
    pub day_width: u32,
    pub theme: Theme,
    pub fields: Vec<Field>,
    pub values: HashMap<String, HashMap<String, String>>,
    pub hidden_columns: Vec<String>,
    pub column_order: Vec<String>,
    pub tooltip_columns: Vec<String>,
    pub column_widths: HashMap<String, u32>,
    pub frozen_columns: usize,
    pub counting: Counting,
    pub can_edit: bool,
}

/// Which days do not count towards a task's length.
///
/// Four switches rather than one: a site that works Saturdays still closes on
/// holidays, and a plan drawn in calendar days may still want a person's leave
/// left out of their own tasks.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Counting {
    pub monday: bool,
    pub tuesday: bool,
    pub wednesday: bool,
    pub thursday: bool,
    pub friday: bool,
    pub saturday: bool,
    pub sunday: bool,
    pub holidays: bool,
    pub leave: bool,
}

impl Counting {
    fn skips(&self, weekday: jiff::civil::Weekday) -> bool {
        use jiff::civil::Weekday::*;

        match weekday {
            Monday => self.monday,
            Tuesday => self.tuesday,
            Wednesday => self.wednesday,
            Thursday => self.thursday,
            Friday => self.friday,
            Saturday => self.saturday,
            Sunday => self.sunday,
        }
    }

    /// Whether any day of the week is left out, which is what makes the count
    /// "営業日" rather than plain days.
    fn skips_a_weekday(&self) -> bool {
        self.monday
            || self.tuesday
            || self.wednesday
            || self.thursday
            || self.friday
            || self.saturday
            || self.sunday
    }
}

impl Counting {
    /// Counts every day, including the ones most workplaces are shut on.
    ///
    /// The starting point for a project that wants calendar days, and what the
    /// arithmetic tests build on.
    #[cfg(test)]
    pub fn none() -> Self {
        Self {
            monday: false,
            tuesday: false,
            wednesday: false,
            thursday: false,
            friday: false,
            saturday: false,
            sunday: false,
            holidays: false,
            leave: false,
        }
    }
}

impl Default for Counting {
    fn default() -> Self {
        // Weekends and public holidays are days off nearly everywhere this is
        // used, so counting them would be the surprising answer. Monday to
        // Friday start off: a workplace that closes on Wednesdays knows it and
        // can say so. Leave is per person and entered on purpose, so it counts
        // from the start too.
        Self {
            monday: false,
            tuesday: false,
            wednesday: false,
            thursday: false,
            friday: false,
            saturday: true,
            sunday: true,
            holidays: true,
            leave: true,
        }
    }
}

/// One checkpoint of 予定進捗: by this date, this much should be done.
///
/// Nothing is claimed between two checkpoints. Joining them with a line would
/// be inventing a plan again, only at a finer grain.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Target {
    pub date: String,
    pub percent: i64,
    /// Whether its date has come round.
    pub due: bool,
    /// Whether it came round and the work was not there.
    pub missed: bool,
}

/// A stretch of days one person is away.
///
/// Vacation is per person, so it cannot live in the project's holiday list: two
/// people on the same plan rarely take the same week off.
#[derive(Debug, Clone, Serialize)]
pub struct Leave {
    pub id: String,
    pub assignee: String,
    pub start: String,
    pub end: String,
    pub note: String,
    /// `off` for 休み, `on` for 出社 — a day that counts even though the
    /// calendar says otherwise.
    pub kind: String,
}

/// A stretch of days, as the grid receives it.
#[derive(Debug, Clone, Serialize)]
pub struct Span {
    pub start: String,
    /// Where it stops. For a wait that has not ended, this is today — the days
    /// keep adding up — and `open` says so.
    pub end: String,
    /// Why the work stopped, when somebody said. Free text: it is a note that
    /// happens to be worth counting, not a closed set.
    pub reason: String,
    /// Whether the range is still running.
    pub open: bool,
    /// How many countable days this wait took out of the task, so the stats can
    /// say what the project was waiting on rather than only how long.
    pub days: i64,
}

/// Someone a task can be assigned to, and how their name is drawn.
#[derive(Debug, Clone, Serialize)]
pub struct Assignee {
    pub name: String,
    pub color: String,
    pub background: String,
}

/// One of the states a task can be in, as this project names it.
#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub name: String,
    pub color: String,
    /// The progress this state implies, for projects that link the two.
    pub percent: Option<i64>,
}

impl Status {
    /// The states a project starts with — a plain reading of how work moves,
    /// which any team can rename or replace.
    pub fn defaults() -> Vec<Self> {
        [
            ("未着手", "#f1f5f9", Some(0)),
            ("実施中", "#dbeafe", None),
            ("待ち", "#ede9fe", None),
            ("完了", "#dcfce7", Some(100)),
            ("保留", "#fef3c7", None),
        ]
        .into_iter()
        .map(|(name, color, percent)| Self {
            name: name.to_owned(),
            color: color.to_owned(),
            percent,
        })
        .collect()
    }
}

/// 元号: the name a period of years is written under, and the day it began.
#[derive(Debug, Clone, Serialize)]
pub struct Era {
    pub from: String,
    pub name: String,
}

/// A non-working day the project declared.
#[derive(Debug, Clone, Serialize)]
pub struct Holiday {
    pub date: String,
    pub name: String,
}

/// A column the project added for itself.
#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub id: String,
    pub label: String,
    pub kind: String,
    /// The master list, for `select` and `suggest` fields.
    pub options: Vec<Option_>,
    /// Whether any task has written a value here. What is already entered is
    /// what makes changing the kind a bad idea.
    #[serde(default)]
    pub in_use: bool,
}

/// One entry of a master list, with the colours it is drawn in.
///
/// Named with a trailing underscore because `Option` is taken by something far
/// more important.
#[derive(Debug, Clone, Serialize)]
pub struct Option_ {
    pub value: String,
    /// Empty means "whatever the cell would otherwise use".
    pub color: String,
    pub background: String,
}

/// The chart's palette, overridable per project.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Theme {
    pub bar: String,
    pub done: String,
    /// The actual bar. In the same blue as the plan, two identical bands sit one
    /// above the other and neither says which one is the work that happened.
    pub actual: String,
    pub summary: String,
    pub late: String,
    /// Saturdays and Sundays are shaded apart: a Japanese calendar prints
    /// Saturday in blue and Sunday in red, and reading one for the other is
    /// the kind of mistake a schedule should not invite.
    pub saturday: String,
    pub sunday: String,
    pub holiday: String,
    /// The days somebody is away, shaded in their own row only.
    pub leave: String,
    /// The days the work was stopped, waiting on somebody else.
    pub wait: String,
    /// Today's line and the date above it. Its own colour because red is
    /// already Sunday's, and "is this a holiday or is this now" is not a
    /// question a chart should ask.
    pub today: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            holidays: Vec::new(),
            leaves: Vec::new(),
            assignees: Vec::new(),
            statuses: Status::defaults(),
            // April: the business year almost every Japanese organisation uses.
            fiscal_year_start: 4,
            japanese_era: false,
            quarters: true,
            eras: Vec::new(),
            day_width: 26,
            theme: Theme::default(),
            fields: Vec::new(),
            values: HashMap::new(),
            hidden_columns: Vec::new(),
            column_order: Vec::new(),
            tooltip_columns: Vec::new(),
            column_widths: HashMap::new(),
            frozen_columns: 1,
            counting: Counting::default(),
            can_edit: false,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bar: "#dbeafe".to_owned(),
            done: "#2563eb".to_owned(),
            // The actual half of 予実. Drawing it in a colour of its own is the
            // norm, and red is taken by lateness, so green.
            actual: "#059669".to_owned(),
            // Nothing that reads as black: a summary row gathers rows together,
            // it is not the row to shout with.
            summary: "#64748b".to_owned(),
            late: "#dc2626".to_owned(),
            saturday: "#eff6ff".to_owned(),
            sunday: "#fef2f2".to_owned(),
            holiday: "#fef2f2".to_owned(),
            // Amber rather than another red or blue: a person being away is not
            // the same kind of day off as a weekend or a holiday.
            leave: "#fff7ed".to_owned(),
            wait: "#ede9fe".to_owned(),
            today: "#ea580c".to_owned(),
        }
    }
}

impl Theme {
    /// Reads the palette from stored settings, keeping the default for anything
    /// unset or malformed — a bad colour must not blank out the chart.
    pub fn from_settings(settings: &std::collections::HashMap<String, String>) -> Self {
        let mut theme = Self::default();

        for (key, target) in [
            ("color_bar", &mut theme.bar),
            ("color_done", &mut theme.done),
            ("color_actual", &mut theme.actual),
            ("color_summary", &mut theme.summary),
            ("color_late", &mut theme.late),
            ("color_saturday", &mut theme.saturday),
            ("color_sunday", &mut theme.sunday),
            ("color_holiday", &mut theme.holiday),
            ("color_leave", &mut theme.leave),
            ("color_wait", &mut theme.wait),
            ("color_today", &mut theme.today),
        ] {
            if let Some(value) = settings.get(key)
                && is_hex_colour(value)
            {
                *target = value.clone();
            }
        }

        theme
    }
}

/// `#rgb` or `#rrggbb`. Anything else is not going into a stylesheet.
pub fn is_hex_colour(value: &str) -> bool {
    let Some(digits) = value.strip_prefix('#') else {
        return false;
    };

    matches!(digits.len(), 3 | 6) && digits.chars().all(|c| c.is_ascii_hexdigit())
}

/// Flattens the stored rows into the grid's view of them.
///
/// Rows arrive ordered by `sort_key`; this walks them depth-first so a child
/// always follows its parent, and resolves every parent's dates and progress
/// from its subtree on the way back up.
pub fn build(
    project_id: &str,
    revision: i64,
    today: Date,
    rows: Vec<TaskRow>,
    settings: Settings,
) -> GridData {
    let Settings {
        holidays,
        leaves,
        assignees,
        statuses,
        fiscal_year_start,
        japanese_era,
        quarters,
        eras,
        day_width,
        theme,
        fields,
        mut values,
        hidden_columns,
        column_order,
        tooltip_columns,
        column_widths,
        frozen_columns,
        counting,
        can_edit,
    } = settings;

    // Working-day counting needs the set of days that do not count.
    let off_days: std::collections::HashSet<Date> = if counting.holidays {
        holidays
            .iter()
            .filter_map(|holiday| parse_date(&holiday.date))
            .collect()
    } else {
        std::collections::HashSet::new()
    };

    // Leave is per person, so it is indexed by the name in the 担当者 column.
    // 出社 rides in the same list, pointing the other way.
    let mut away: HashMap<String, Vec<(Date, Date)>> = HashMap::new();
    let mut working: HashMap<String, Vec<(Date, Date)>> = HashMap::new();

    for leave in &leaves {
        let (Some(start), Some(end)) = (parse_date(&leave.start), parse_date(&leave.end)) else {
            continue;
        };

        let target = if leave.kind == "on" {
            &mut working
        } else {
            &mut away
        };

        target
            .entry(leave.assignee.trim().to_owned())
            .or_default()
            .push((start.min(end), end.max(start)));
    }

    let calendar = Calendar {
        counting,
        off_days,
        away,
        working,
    };
    let mut children: HashMap<Option<&str>, Vec<&TaskRow>> = HashMap::new();
    for row in &rows {
        children
            .entry(row.parent_id.as_deref())
            .or_default()
            .push(row);
    }
    for siblings in children.values_mut() {
        siblings.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));
    }

    let mut tasks = Vec::with_capacity(rows.len());
    for root in children.get(&None).into_iter().flatten() {
        visit(root, 0, &children, today, &calendar, &mut tasks);
    }

    for task in &mut tasks {
        task.values = values.remove(&task.id).unwrap_or_default();
    }

    // A project with no dates still needs a window, so fall back to the month
    // around today.
    // Everything the chart draws has to fit inside it — the plan, what actually
    // happened, and the wait in between. A date outside the window used to
    // paint a bar straight across the page.
    let (mut range_start, mut range_end) = (today, today);
    for task in &tasks {
        for date in [&task.start, &task.actual_start] {
            if let Some(date) = date.as_deref().and_then(parse_date) {
                range_start = range_start.min(date);
            }
        }
        for date in [&task.end, &task.actual_end] {
            if let Some(date) = date.as_deref().and_then(parse_date) {
                range_end = range_end.max(date);
            }
        }
    }

    GridData {
        // Filled in by the caller: the sets live in the database, and this
        // function is given a table rather than a connection.
        filter_sets: Vec::new(),
        project_id: project_id.to_owned(),
        revision,
        // The caller decides: it is the one that knows who is reading.
        language: "ja".to_owned(),
        today: today.to_string(),
        // A week either side: three days left the first month a sliver wide, and
        // its label printed on top of the next one's.
        range_start: range_start
            .saturating_sub(jiff::Span::new().days(7))
            .to_string(),
        range_end: range_end
            .saturating_add(jiff::Span::new().days(7))
            .to_string(),
        holidays,
        leaves,
        assignees,
        statuses,
        theme,
        fields,
        hidden_columns,
        column_order,
        tooltip_columns,
        column_widths,
        frozen_columns,
        counting,
        fiscal_year_start,
        japanese_era,
        quarters,
        eras,
        day_width,
        can_edit,
        tasks,
    }
}

/// A named set of filter conditions.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FilterSet {
    pub id: String,
    pub name: String,
    /// What the grid puts back: filters and directions, as the island wrote it.
    pub conditions: String,
    /// Whether it belongs to everybody. The alternative is "mine", because a
    /// list of somebody else's private views is noise on a screen.
    pub shared: bool,
}

/// One person's days over a stretch of the calendar.
#[derive(Debug, Clone, Serialize)]
pub struct Load {
    pub assignee: String,
    /// Days this person could work: the calendar's, less their leave.
    /// `None` for the unassigned row — nobody has a capacity.
    pub capacity: Option<i64>,
    /// Days of the window that are already behind us. Counted apart from the
    /// rest, because a month half gone with "12 days free" in it is a lie by
    /// arithmetic: those twelve days include ones nobody can use any more.
    pub gone: Option<i64>,
    /// Days from today on that their unfinished tasks cover, counted once each.
    pub busy: i64,
    pub free: Option<i64>,
    /// Which days those are, as stretches. A number answers "how much"; the
    /// only question anybody asks next is "when", and counting it back off the
    /// chart by hand is the work this page exists to remove.
    pub free_spans: Vec<Span>,
    /// Days where two or more of their tasks overlap. Kept apart from `busy`
    /// on purpose: it is a different question, and adding it in would make a
    /// person with three tasks on one day look three days busier than they are.
    pub overlap: i64,
    pub overlap_spans: Vec<Span>,
    /// How many tasks were counted, so a row of zeroes can be read.
    pub tasks: usize,
}

/// Who has room, over a stretch of months.
///
/// The unit is a day, and a day is either taken or it is not. Anything finer —
/// half a day, 30% of a person — is a number nobody in a 予実 meeting can
/// defend, and it would need a field on every task that nobody would fill in.
///
/// Days are counted once. Three tasks on one Tuesday is one Tuesday: the
/// alternative reports 300% loads that read as nonsense and get ignored, which
/// is worse than a number that is merely coarse. The overlap is reported
/// separately, because "you are booked solid" and "you are booked three times
/// over" are different problems.
pub fn load(data: &GridData, from: Date, to: Date, today: Date) -> Vec<Load> {
    let calendar = calendar_of(data);

    // Everything below is about what is left. Yesterday cannot be booked and it
    // cannot be free either; it is gone, and says so in a column of its own.
    // 稼働可能日数 = 経過済 + 割当済 + 空き日数, which is the only way a row of
    // numbers about a half-finished month adds up.
    let ahead = from.max(today);

    // Everyone the project knows about, plus anyone standing on a task without
    // being on the list, plus the tasks nobody is holding.
    let mut names: Vec<String> = data
        .assignees
        .iter()
        .map(|person| person.name.trim().to_owned())
        .collect();

    for task in &data.tasks {
        let name = task.assignee.trim().to_owned();
        if !names.contains(&name) {
            names.push(name);
        }
    }

    names.sort();
    names.dedup();
    // The unassigned row last: it is work, not a person.
    names.retain(|name| !name.is_empty());
    names.push(String::new());

    names
        .into_iter()
        .map(|name| {
            let mut taken: HashMap<Date, usize> = HashMap::new();
            let mut counted = 0;

            for task in &data.tasks {
                // Summary rows are their children added up; counting both would
                // book every day twice.
                if task.has_children || task.assignee.trim() != name {
                    continue;
                }

                // Finished work is not in anybody's way. The question this page
                // answers is what is left.
                if task.actual_end.is_some() {
                    continue;
                }

                let (Some(start), Some(end)) = (
                    task.start.as_deref().and_then(parse_date),
                    task.end.as_deref().and_then(parse_date),
                ) else {
                    continue;
                };

                counted += 1;

                let mut day = start.max(ahead);
                let last = end.min(to);
                while day <= last {
                    // Days the person could not have worked anyway are not days
                    // their plan is using up.
                    if name.is_empty() || calendar.is_workday(&name, &[], day) {
                        *taken.entry(day).or_default() += 1;
                    }

                    let Ok(next) = day.tomorrow() else { break };
                    day = next;
                }
            }

            let busy = taken.len() as i64;
            let overlap = taken.values().filter(|count| **count > 1).count() as i64;

            let workable = |day: Date| !name.is_empty() && calendar.is_workday(&name, &[], day);

            let count = |first: Date, last: Date| {
                let mut days = 0;
                let mut day = first;
                while day <= last {
                    if workable(day) {
                        days += 1;
                    }

                    let Ok(next) = day.tomorrow() else { break };
                    day = next;
                }

                days
            };

            let capacity = (!name.is_empty()).then(|| count(from, to));
            // Up to yesterday: today is still a day somebody can use.
            let gone = (!name.is_empty()).then(|| match today > from {
                true => count(from, today.yesterday().unwrap_or(from).min(to)),
                false => 0,
            });

            // Runs are joined across days nobody could have worked anyway, so a
            // fortnight free reads as one stretch rather than as three weekday
            // fragments with the weekends punched out of it.
            let free_spans = stretches(
                ahead,
                to,
                |day| workable(day) && !taken.contains_key(&day),
                |day| !workable(day),
            );
            let overlap_spans = stretches(
                ahead,
                to,
                |day| taken.get(&day).is_some_and(|count| *count > 1),
                |day| !workable(day),
            );

            Load {
                assignee: name,
                capacity,
                gone,
                busy,
                // What is actually left: the window, less the part that has
                // gone by, less what is already booked in the rest of it.
                free: capacity
                    .zip(gone)
                    .map(|(capacity, gone)| capacity - gone - busy),
                free_spans,
                overlap,
                overlap_spans,
                tasks: counted,
            }
        })
        .collect()
}

/// Runs of days that answer yes, as `from`/`to` pairs.
///
/// `neutral` days neither start a run nor end one: a Sunday in the middle of a
/// free fortnight is part of the stretch, and a Sunday at either end of it is
/// not worth mentioning.
fn stretches(
    from: Date,
    to: Date,
    yes: impl Fn(Date) -> bool,
    neutral: impl Fn(Date) -> bool,
) -> Vec<Span> {
    let mut spans: Vec<(Date, Date)> = Vec::new();
    let mut open: Option<(Date, Date)> = None;
    let mut day = from;

    while day <= to {
        if yes(day) {
            open = match open {
                Some((start, _)) => Some((start, day)),
                None => Some((day, day)),
            };
        } else if !neutral(day)
            && let Some(span) = open.take()
        {
            spans.push(span);
        }

        let Ok(next) = day.tomorrow() else { break };
        day = next;
    }

    if let Some(span) = open {
        spans.push(span);
    }

    spans
        .into_iter()
        .map(|(start, end)| Span {
            start: start.to_string(),
            end: end.to_string(),
            reason: String::new(),
            open: false,
            days: i64::from(end.since(start).map(|span| span.get_days()).unwrap_or(0)) + 1,
        })
        .collect()
}

/// The project's calendar, rebuilt from what the grid was given.
///
/// `build` makes one from its settings; this makes the same one from the table
/// it produced, so a page that only has the table still counts days the way
/// the project counts them.
fn calendar_of(data: &GridData) -> Calendar {
    let off_days: std::collections::HashSet<Date> = if data.counting.holidays {
        data.holidays
            .iter()
            .filter_map(|holiday| parse_date(&holiday.date))
            .collect()
    } else {
        std::collections::HashSet::new()
    };

    let mut away: HashMap<String, Vec<(Date, Date)>> = HashMap::new();
    let mut working: HashMap<String, Vec<(Date, Date)>> = HashMap::new();

    for leave in &data.leaves {
        let (Some(start), Some(end)) = (parse_date(&leave.start), parse_date(&leave.end)) else {
            continue;
        };

        let target = if leave.kind == "on" {
            &mut working
        } else {
            &mut away
        };

        target
            .entry(leave.assignee.trim().to_owned())
            .or_default()
            .push((start.min(end), end.max(start)));
    }

    Calendar {
        counting: data.counting,
        off_days,
        away,
        working,
    }
}

/// Emits `row` and its subtree, returning the row's resolved schedule.
fn visit<'rows>(
    row: &'rows TaskRow,
    depth: usize,
    children: &HashMap<Option<&'rows str>, Vec<&'rows TaskRow>>,
    today: Date,
    calendar: &Calendar,
    out: &mut Vec<TaskView>,
) -> Resolved {
    let kids = children.get(&Some(row.id.as_str()));
    let has_children = kids.is_some_and(|kids| !kids.is_empty());

    // Reserve this row's slot before descending, so the subtree lands after it.
    let index = out.len();
    out.push(TaskView {
        id: row.id.clone(),
        depth,
        name: row.name.clone(),
        start: None,
        end: None,
        actual_start: None,
        actual_end: None,
        progress: 0,
        days: None,
        actual_days: None,
        start_variance: None,
        end_variance: None,
        overdue: 0,
        waits: Vec::new(),
        wait_days: 0,
        status: row.status.clone(),
        assignee: row.assignee.clone(),
        note: row.note.clone(),
        targets: Vec::new(),
        expected: None,
        delayed: false,
        color: row.color.clone(),
        background: row.background.clone(),
        has_children,
        tags: row.tags.split_whitespace().map(ToOwned::to_owned).collect(),
        values: HashMap::new(),
    });

    // The waits belong to this row, and come out of the day count and the
    // variance alike.
    let waits = parse_waits(&row.waits);
    let spans = wait_spans(&waits, today);

    let resolved = if has_children {
        let mut subtree = Vec::new();
        for kid in kids.into_iter().flatten() {
            let resolved = visit(kid, depth + 1, children, today, calendar, out);
            let weight = match (resolved.start, resolved.end) {
                (Some(start), Some(end)) => calendar.days(
                    &kid.assignee,
                    &wait_spans(&parse_waits(&kid.waits), today),
                    start,
                    end,
                ),
                // A child with no dates still counts, just at the lightest weight.
                _ => 1,
            };

            subtree.push((resolved, weight));
        }
        rollup(&subtree)
    } else {
        let start = row.start_date.as_deref().and_then(parse_date);
        let end = row.end_date.as_deref().and_then(parse_date);
        let actual_start = row.actual_start.as_deref().and_then(parse_date);
        let actual_end = row.actual_end.as_deref().and_then(parse_date);

        Resolved {
            start,
            end,
            actual_start,
            actual_end,
            progress: row.progress.clamp(0, 100),
            start_variance: start.zip(actual_start).map(|(planned, actual)| {
                difference(calendar, &row.assignee, &spans, planned, actual)
            }),
            end_variance: end.zip(actual_end).map(|(planned, actual)| {
                difference(calendar, &row.assignee, &spans, planned, actual)
            }),
            overdue: match (end, actual_end) {
                (Some(end), None) if today > end => {
                    difference(calendar, &row.assignee, &spans, end, today)
                }
                _ => 0,
            },
            // Filled in below, once this row's own checkpoints are read.
            delayed: false,
        }
    };

    // 予定進捗 is read against the progress this row ended up with, so a summary
    // row is judged by the same rule as any other: the number on the screen
    // against the number the plan named.
    let targets = parse_targets(&row.targets, today, resolved.progress);
    let expected = expected_progress(&targets);

    let view = &mut out[index];
    view.start = resolved.start.map(|date| date.to_string());
    view.end = resolved.end.map(|date| date.to_string());
    view.actual_start = resolved.actual_start.map(|date| date.to_string());
    view.actual_end = resolved.actual_end.map(|date| date.to_string());
    view.progress = resolved.progress;
    view.days = resolved
        .start
        .zip(resolved.end)
        .map(|(start, end)| calendar.days(&row.assignee, &spans, start, end));
    // An unfinished actual is counted too. The chart draws it up to today, and a
    // blank beside a drawn bar makes one row contradict itself.
    view.actual_days = resolved.actual_start.map(|start| {
        let end = resolved.actual_end.unwrap_or(today);
        calendar.days(&row.assignee, &spans, start, end)
    });

    // Counted the way this project counts days, and — for a summary row —
    // summed from the children rather than taken from the rolled-up dates.
    view.start_variance = resolved.start_variance;
    view.end_variance = resolved.end_variance;
    view.overdue = resolved.overdue;

    // Days the waits took out: the difference from counting as if they were not
    // there at all.
    view.waits = waits
        .iter()
        .map(|wait| {
            let end = wait.to.unwrap_or(today).max(wait.from);

            // Clipped to the plan: a wait recorded outside the task's own dates
            // took nothing out of it, and the total below must agree with the
            // per-reason parts.
            let days = match resolved.start.zip(resolved.end) {
                Some((start, finish)) if wait.from <= finish && end >= start => {
                    calendar.days(&row.assignee, &[], wait.from.max(start), end.min(finish))
                }
                _ => 0,
            };

            Span {
                start: wait.from.to_string(),
                end: end.to_string(),
                reason: wait.reason.clone(),
                open: wait.to.is_none(),
                days,
            }
        })
        .collect();
    view.wait_days = match resolved.start.zip(resolved.end) {
        Some((start, end)) if !spans.is_empty() => {
            calendar.days(&row.assignee, &[], start, end)
                - calendar.days(&row.assignee, &spans, start, end)
        }
        _ => 0,
    };

    view.delayed = targets.iter().any(|target| target.missed) || resolved.delayed;
    view.targets = targets;
    view.expected = expected;

    Resolved {
        delayed: view.delayed,
        ..resolved
    }
}

#[derive(Debug, Clone, Copy)]
struct Resolved {
    start: Option<Date>,
    end: Option<Date>,
    actual_start: Option<Date>,
    actual_end: Option<Date>,
    progress: i64,
    /// Variance is summed from the children rather than recomputed from dates. A
    /// parent's dates are the earliest and the latest, so subtracting them lets
    /// the slippage inside cancel itself out.
    start_variance: Option<i64>,
    end_variance: Option<i64>,
    overdue: i64,
    /// Behind a checkpoint — its own, or one inside its subtree. A parent that
    /// is collapsed still has to say that something under it is behind.
    delayed: bool,
}

/// A parent spans its children and inherits their progress, weighted by how
/// long each child runs — a one-day task finishing does not move a parent as
/// much as a one-month task does.
fn rollup(children: &[(Resolved, i64)]) -> Resolved {
    let start = children.iter().filter_map(|(child, _)| child.start).min();
    let end = children.iter().filter_map(|(child, _)| child.end).max();
    let actual_start = children
        .iter()
        .filter_map(|(child, _)| child.actual_start)
        .min();

    // A parent has only finished when every child has: one unfinished child
    // leaves the whole thing open, so a missing end swallows the maximum.
    let actual_end = if children.iter().any(|(child, _)| child.actual_end.is_none()) {
        None
    } else {
        children
            .iter()
            .filter_map(|(child, _)| child.actual_end)
            .max()
    };

    let mut total_weight = 0i64;
    let mut weighted = 0i64;

    for (child, weight) in children {
        total_weight += weight;
        weighted += weight * child.progress;
    }

    let progress = if total_weight == 0 {
        0
    } else {
        weighted / total_weight
    };

    // 積算: the subtree's slippage in days. A parent whose earliest start moved
    // by nothing can still hold two children that slipped a week each.
    let sum = |values: &mut dyn Iterator<Item = Option<i64>>| {
        values.fold(None, |total: Option<i64>, value| match (total, value) {
            (total, None) => total,
            (None, Some(value)) => Some(value),
            (Some(total), Some(value)) => Some(total + value),
        })
    };

    Resolved {
        start,
        end,
        actual_start,
        actual_end,
        progress,
        start_variance: sum(&mut children.iter().map(|(child, _)| child.start_variance)),
        end_variance: sum(&mut children.iter().map(|(child, _)| child.end_variance)),
        overdue: children.iter().map(|(child, _)| child.overdue).sum(),
        delayed: children.iter().any(|(child, _)| child.delayed),
    }
}

/// Signed difference between two dates, in the days this project counts.
///
/// Both ends are one day, so the distance between them is one less than the
/// count: 金曜 to 月曜 is one working day apart, not two.
fn difference(
    calendar: &Calendar,
    assignee: &str,
    waits: &[(Date, Date)],
    planned: Date,
    actual: Date,
) -> i64 {
    if actual == planned {
        return 0;
    }

    let (from, to) = if actual > planned {
        (planned, actual)
    } else {
        (actual, planned)
    };

    let distance = calendar.between(assignee, waits, from, to);

    if actual > planned {
        distance
    } else {
        -distance
    }
}

/// The share of the task that should be done today, as a percentage.
///
/// Both endpoints are inclusive: a task that starts and ends today is a full
/// day of work, not a zero-length one.
/// Reads the stored form of 予定進捗: `YYYY-MM-DD/PERCENT` per line.
///
/// The same shape as 待ち, because it is the same kind of thing: a short list a
/// person keeps on one task, entered a line at a time.
fn parse_targets(text: &str, today: Date, progress: i64) -> Vec<Target> {
    let mut targets: Vec<(Date, i64)> = text
        .lines()
        .filter_map(|line| {
            let (date, percent) = line.trim().split_once('/')?;

            Some((
                parse_date(date)?,
                percent.trim().parse::<i64>().ok()?.clamp(0, 100),
            ))
        })
        .collect();

    targets.sort_by_key(|(date, _)| *date);

    targets
        .into_iter()
        .map(|(date, percent)| {
            let due = date <= today;

            Target {
                date: date.to_string(),
                percent,
                due,
                missed: due && progress < percent,
            }
        })
        .collect()
}

/// What the plan says should be done by now.
///
/// The last checkpoint whose date has passed, and nothing between two of them:
/// joining checkpoints with a line would invent a plan again, only at a finer
/// grain. `None` means the plan has not said, and nothing is judged.
fn expected_progress(targets: &[Target]) -> Option<i64> {
    targets
        .iter()
        .filter(|target| target.due)
        .map(|target| target.percent)
        .max()
}

/// How the project counts days.
///
/// Excluding weekends and holidays changes the day count *and* the expected
/// progress: if a task is not meant to advance on a Sunday, it must not be
/// called late for having failed to.
struct Calendar {
    counting: Counting,
    off_days: std::collections::HashSet<Date>,
    /// The days each assignee is away, by the name in the 担当者 column.
    away: HashMap<String, Vec<(Date, Date)>>,
    /// The days they are in regardless — 休日出勤.
    working: HashMap<String, Vec<(Date, Date)>>,
}

impl Calendar {
    /// Inclusive day count between two dates, never less than one.
    ///
    /// Counted for whoever is doing the work: a week the assignee is on leave
    /// is not a week of that person's task.
    fn days(&self, assignee: &str, waits: &[(Date, Date)], start: Date, end: Date) -> i64 {
        let total = i64::from(end.since(start).map(|span| span.get_days()).unwrap_or(0)) + 1;
        let away = self.away.get(assignee.trim());

        let nothing_to_skip = !self.counting.skips_a_weekday()
            && !self.counting.holidays
            && away.is_none()
            && waits.is_empty();

        if nothing_to_skip {
            return total.max(1);
        }

        let mut counted = 0;
        let mut date = start;

        for _ in 0..total.max(0) {
            if self.is_workday(assignee, waits, date) {
                counted += 1;
            }

            let Ok(next) = date.checked_add(jiff::Span::new().days(1)) else {
                break;
            };
            date = next;
        }

        counted.max(1)
    }

    /// Countable days after `from`, up to and including `to`.
    ///
    /// Not the inclusive count minus one: when `from` itself is a day that does
    /// not count — a plan due on a Saturday — that subtraction takes away a day
    /// that was never counted, and the delay comes out one short.
    fn between(&self, assignee: &str, waits: &[(Date, Date)], from: Date, to: Date) -> i64 {
        let total = i64::from(to.since(from).map(|span| span.get_days()).unwrap_or(0));
        let mut counted = 0;
        let mut date = from;

        for _ in 0..total.max(0) {
            let Ok(next) = date.checked_add(jiff::Span::new().days(1)) else {
                break;
            };
            date = next;

            if self.is_workday(assignee, waits, date) {
                counted += 1;
            }
        }

        counted
    }

    fn is_workday(&self, assignee: &str, waits: &[(Date, Date)], date: Date) -> bool {
        // A wait is excluded whatever the calendar says: it is the record that
        // nobody could move that day.
        if waits.iter().any(|(from, to)| date >= *from && date <= *to) {
            return false;
        }

        // 出社 beats the calendar: the person said they were in that day.
        let in_anyway = self
            .working
            .get(assignee.trim())
            .is_some_and(|spans| spans.iter().any(|(from, to)| date >= *from && date <= *to));

        if !in_anyway
            && (self.counting.skips(date.weekday())
                || (self.counting.holidays && self.off_days.contains(&date)))
        {
            return false;
        }

        !(self.counting.leave && self.on_leave(assignee, date))
    }

    fn on_leave(&self, assignee: &str, date: Date) -> bool {
        self.away
            .get(assignee.trim())
            .is_some_and(|spans| spans.iter().any(|(from, to)| date >= *from && date <= *to))
    }
}

/// One wait, as it is stored and as the calendar needs it.
struct Wait {
    from: Date,
    /// `None` while it is still running.
    to: Option<Date>,
    reason: String,
}

/// Reads the stored form: `FROM/TO` or `FROM/` for one that has not ended,
/// each optionally followed by `:reason`, separated by newlines.
///
/// Newline separated rather than space separated, because a reason has spaces
/// in it and a person writing 「他部署 承認待ち」 should not have to think about
/// that.
fn parse_waits(text: &str) -> Vec<Wait> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            let (range, reason) = line.split_once(':').unwrap_or((line, ""));
            let (from, to) = range.trim().split_once('/')?;

            let from = parse_date(from)?;
            let to = parse_date(to);

            Some(Wait {
                // A range written backwards means the same days either way.
                from: to.map_or(from, |to| from.min(to)),
                to: to.map(|to| to.max(from)),
                reason: reason.trim().to_owned(),
            })
        })
        .collect()
}

/// The days a wait covers, up to today while it is still running.
fn wait_spans(waits: &[Wait], today: Date) -> Vec<(Date, Date)> {
    waits
        .iter()
        .map(|wait| (wait.from, wait.to.unwrap_or(today).max(wait.from)))
        .collect()
}

fn parse_date(text: &str) -> Option<Date> {
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Counts every day on the calendar.
    ///
    /// These tests are about the arithmetic — what a parent sums, what a wait
    /// takes out, when a row is late — and a calendar underneath would make
    /// every expected number depend on which weekday the fixture happens to
    /// start on. The tests that are about the calendar say so and set their own.
    fn build_for_test(id: &str, revision: i64, today: Date, rows: Vec<TaskRow>) -> GridData {
        build(
            id,
            revision,
            today,
            rows,
            Settings {
                can_edit: true,
                counting: Counting {
                    leave: true,
                    ..Counting::none()
                },
                ..Settings::default()
            },
        )
    }

    /// A task due Saturday that finished on Tuesday is two working days late:
    /// Monday and Tuesday.
    ///
    /// Counting both ends and subtracting one took away a Saturday that was
    /// never counted in the first place, and answered one.
    #[test]
    fn a_deadline_on_a_saturday_still_counts_the_days_after_it() {
        let mut task = row("t", None, "2026-09-08", "2026-09-12", 0);
        task.actual_start = Some("2026-09-08".to_owned());
        task.actual_end = Some("2026-09-15".to_owned());

        let data = build(
            "p",
            1,
            date("2026-09-20"),
            vec![task],
            Settings {
                can_edit: true,
                counting: Counting {
                    saturday: true,
                    sunday: true,
                    ..Counting::default()
                },
                ..Settings::default()
            },
        );

        // 9/12 Sat → 9/15 Tue: what counts is Monday and Tuesday.
        assert_eq!(data.tasks[0].end_variance, Some(2));
    }

    /// A parent's variance is summed from its children, not recomputed.
    ///
    /// Subtracting dates rounded out to the earliest and the latest lets the
    /// children's slippage cancel. A parent with one child a day early and one a
    /// day late has not "stayed on plan".
    #[test]
    fn a_parent_adds_up_what_its_children_slipped() {
        let mut early = row("a", Some("p"), "2026-08-03", "2026-08-07", 0);
        early.actual_start = Some("2026-08-02".to_owned());
        early.actual_end = Some("2026-08-06".to_owned());

        let mut late = row("b", Some("p"), "2026-08-10", "2026-08-14", 0);
        late.actual_start = Some("2026-08-11".to_owned());
        late.actual_end = Some("2026-08-15".to_owned());

        let data = build_for_test(
            "p",
            1,
            date("2026-08-20"),
            vec![row("p", None, "", "", 0), early, late],
        );

        // Children at -1 and +1. Subtracting the parent's own dates would answer
        // zero; summing keeps both.
        assert_eq!(data.tasks[1].start_variance, Some(-1));
        assert_eq!(data.tasks[2].start_variance, Some(1));
        assert_eq!(data.tasks[0].start_variance, Some(0));
        assert_eq!(data.tasks[0].end_variance, Some(0));

        // Two late children make the parent late by the total.
        let mut later = row("c", Some("q"), "2026-08-10", "2026-08-14", 0);
        later.actual_start = Some("2026-08-12".to_owned());
        later.actual_end = Some("2026-08-17".to_owned());

        let both = build_for_test(
            "q",
            1,
            date("2026-08-20"),
            vec![row("q", None, "", "", 0), later],
        );
        assert_eq!(both.tasks[0].end_variance, Some(3));
    }

    /// Working on a day off. Counted even when weekends are excluded.
    #[test]
    fn working_on_a_day_off_puts_it_back() {
        let mut task = row("t", None, "2026-08-03", "2026-08-09", 0);
        task.assignee = "山田".to_owned();

        let weekends_off = Settings {
            can_edit: true,
            counting: Counting {
                saturday: true,
                sunday: true,
                ..Counting::default()
            },
            ..Settings::default()
        };

        // Monday to Friday: five days.
        let plain = build(
            "p",
            1,
            date("2026-08-03"),
            vec![task.clone()],
            Settings {
                ..weekends_off.clone()
            },
        );
        assert_eq!(plain.tasks[0].days, Some(5));

        // Plus the Saturday worked: six.
        let worked = build(
            "p",
            1,
            date("2026-08-03"),
            vec![task],
            Settings {
                leaves: vec![Leave {
                    id: "w".to_owned(),
                    assignee: "山田".to_owned(),
                    start: "2026-08-08".to_owned(),
                    end: "2026-08-08".to_owned(),
                    note: "休日出勤".to_owned(),
                    kind: "on".to_owned(),
                }],
                ..weekends_off
            },
        );
        assert_eq!(worked.tasks[0].days, Some(6));
    }

    /// Days spent waiting leave both the day count and the delay judgement.
    #[test]
    fn a_wait_takes_its_days_out_of_the_task() {
        let mut task = row("t", None, "2026-08-03", "2026-08-14", 0);
        task.waits = "2026-08-05/2026-08-09".to_owned();

        let data = build_for_test("p", 1, date("2026-08-07"), vec![task]);

        // Five of the twelve days were spent waiting.
        assert_eq!(data.tasks[0].days, Some(7));
        assert_eq!(data.tasks[0].wait_days, 5);
        assert_eq!(data.tasks[0].waits.len(), 1);
    }

    /// Both variances follow the project's own way of counting days.
    ///
    /// 2026-08-05 Wed to 2026-08-10 Mon is five days on the calendar and three
    /// working days apart. Under this setting the picture (calendar) and the
    /// number (working days) disagree on purpose.
    #[test]
    fn variance_counts_the_days_the_project_counts() {
        let mut task = row("t", None, "2026-08-05", "2026-09-01", 0);
        task.actual_start = Some("2026-08-10".to_owned());

        let calendar_days = build_for_test("p", 1, date("2026-08-12"), vec![task.clone()]);
        assert_eq!(calendar_days.tasks[0].start_variance, Some(5));

        let workdays = build(
            "p",
            1,
            date("2026-08-12"),
            vec![task],
            Settings {
                can_edit: true,
                counting: Counting {
                    saturday: true,
                    sunday: true,
                    holidays: true,
                    leave: true,
                    ..Counting::default()
                },
                ..Settings::default()
            },
        );
        assert_eq!(workdays.tasks[0].start_variance, Some(3));
    }

    /// A leave for one person, on a project that counts calendar days.
    fn with_leave(rows: Vec<TaskRow>, today: Date, leave: (&str, &str, &str)) -> GridData {
        build(
            "p",
            1,
            today,
            rows,
            Settings {
                can_edit: true,
                leaves: vec![Leave {
                    id: "l".to_owned(),
                    assignee: leave.0.to_owned(),
                    start: leave.1.to_owned(),
                    end: leave.2.to_owned(),
                    note: "夏季休暇".to_owned(),
                    kind: "off".to_owned(),
                }],
                // Leave alone, so the numbers below are about the leave.
                counting: Counting {
                    leave: true,
                    ..Counting::none()
                },
                ..Settings::default()
            },
        )
    }

    /// A week off is a week the task cannot advance, so it does not count as
    /// days of work either.
    ///
    /// It does not move a checkpoint: a date the plan named is a date, and
    /// somebody has to decide out loud whether the leave changed the promise.
    #[test]
    fn leave_is_not_counted_against_the_person_taking_it() {
        // A plan that runs 8/3–8/14, read on the 10th.
        let mut task = row("t", None, "2026-08-03", "2026-08-14", 50);
        task.assignee = "山田".to_owned();

        let plain = build_for_test("p", 1, date("2026-08-10"), vec![task.clone()]);
        assert_eq!(plain.tasks[0].days, Some(12));

        let away = with_leave(
            vec![task],
            date("2026-08-10"),
            ("山田", "2026-08-05", "2026-08-12"),
        );
        assert_eq!(away.tasks[0].days, Some(4), "休んでいる8日は数えない");
    }

    /// The leave belongs to a person, not to the project.
    #[test]
    fn leave_only_applies_to_its_own_assignee() {
        let mut task = row("t", None, "2026-08-03", "2026-08-14", 0);
        task.assignee = "佐藤".to_owned();

        let data = with_leave(
            vec![task],
            date("2026-08-10"),
            ("山田", "2026-08-05", "2026-08-14"),
        );

        assert_eq!(data.tasks[0].days, Some(12));
    }

    fn date(text: &str) -> Date {
        text.parse().unwrap()
    }

    fn row(id: &str, parent: Option<&str>, start: &str, end: &str, progress: i64) -> TaskRow {
        TaskRow {
            id: id.to_owned(),
            parent_id: parent.map(ToOwned::to_owned),
            sort_key: id.to_owned(),
            name: id.to_owned(),
            start_date: (!start.is_empty()).then(|| start.to_owned()),
            end_date: (!end.is_empty()).then(|| end.to_owned()),
            progress,
            actual_start: None,
            actual_end: None,
            tags: String::new(),
            status: "未着手".to_owned(),
            assignee: String::new(),
            note: String::new(),
            waits: String::new(),
            targets: String::new(),
            color: String::new(),
            background: String::new(),
        }
    }

    /// A parent's own stored dates are ignored; the subtree decides. Otherwise
    /// a stale parent row could hide a child that slipped past it.
    #[test]
    fn a_parent_spans_its_children() {
        let rows = vec![
            row("p", None, "2026-01-01", "2026-01-02", 0),
            row("a", Some("p"), "2026-03-01", "2026-03-10", 100),
            row("b", Some("p"), "2026-02-01", "2026-02-05", 0),
        ];

        let data = build_for_test("proj", 0, date("2026-06-01"), rows);
        let parent = &data.tasks[0];

        assert_eq!(parent.id, "p");
        assert_eq!(parent.start.as_deref(), Some("2026-02-01"));
        assert_eq!(parent.end.as_deref(), Some("2026-03-10"));
    }

    /// Weighting by duration is the whole point: a ten-day task at 100% and a
    /// five-day task at 0% is not a simple 50%.
    #[test]
    fn parent_progress_is_weighted_by_duration() {
        let rows = vec![
            row("p", None, "", "", 0),
            row("a", Some("p"), "2026-03-01", "2026-03-10", 100),
            row("b", Some("p"), "2026-03-11", "2026-03-15", 0),
        ];

        let data = build_for_test("proj", 0, date("2026-01-01"), rows);

        // 10 days done, 5 days not: 1000 / 15.
        assert_eq!(data.tasks[0].progress, 66);
    }

    /// The plan said 60% by the 5th. It is the 6th and the work is at 20%.
    #[test]
    fn a_task_is_late_when_it_misses_a_checkpoint_it_named() {
        let mut task = row("a", None, "2026-03-01", "2026-03-10", 20);
        task.targets = "2026-03-05/60".to_owned();

        let data = build_for_test("proj", 0, date("2026-03-06"), vec![task]);
        let task = &data.tasks[0];

        assert_eq!(task.expected, Some(60));
        assert!(task.delayed);
        assert!(task.targets[0].missed);
    }

    /// A checkpoint that has not come round yet judges nothing. Half a day
    /// before it is due, being at 0% is not being late.
    #[test]
    fn a_checkpoint_judges_nothing_until_its_date() {
        let mut task = row("a", None, "2026-03-01", "2026-03-10", 0);
        task.targets = "2026-03-05/60".to_owned();

        let data = build_for_test("proj", 0, date("2026-03-04"), vec![task]);

        assert_eq!(data.tasks[0].expected, None);
        assert!(!data.tasks[0].delayed);
        assert!(!data.tasks[0].targets[0].due);
    }

    /// The old rule read a plan out of the dates — elapsed days over total —
    /// and called a task late for not being linear. A plan that names no
    /// checkpoint promises nothing, and nothing is judged against it.
    #[test]
    fn a_task_that_names_no_checkpoint_is_never_behind() {
        let rows = vec![row("a", None, "2026-03-01", "2026-03-10", 0)];

        let data = build_for_test("proj", 0, date("2026-03-09"), rows);

        assert_eq!(data.tasks[0].expected, None);
        assert!(!data.tasks[0].delayed);
    }

    /// Only the checkpoints that have passed are read, and the latest of them
    /// wins. Nothing is claimed for the days between two of them.
    #[test]
    fn the_last_checkpoint_that_has_passed_is_the_one_that_counts() {
        let mut task = row("a", None, "2026-03-01", "2026-03-31", 40);
        task.targets = "2026-03-20/80\n2026-03-10/30\n2026-03-31/100".to_owned();

        let data = build_for_test("proj", 0, date("2026-03-15"), vec![task]);
        let task = &data.tasks[0];

        // The 10th asked for 30% and got 40%; the 20th has not come round.
        assert_eq!(task.expected, Some(30));
        assert!(!task.delayed);
        // Kept in date order, whatever order they were typed in.
        let dates: Vec<&str> = task.targets.iter().map(|t| t.date.as_str()).collect();
        assert_eq!(dates, ["2026-03-10", "2026-03-20", "2026-03-31"]);
    }

    /// Two tasks on one day is one day. The alternative reports a 200% load,
    /// which reads as nonsense and gets ignored — and being ignored is worse
    /// than being coarse.
    #[test]
    fn days_are_counted_once_however_many_tasks_land_on_them() {
        let mut first = row("a", None, "2026-03-02", "2026-03-06", 0);
        first.assignee = "山田".to_owned();
        let mut second = row("b", None, "2026-03-04", "2026-03-10", 0);
        second.assignee = "山田".to_owned();

        let data = build_for_test("p", 0, date("2026-03-01"), vec![first, second]);
        let load = load(
            &data,
            date("2026-03-01"),
            date("2026-03-31"),
            date("2026-03-01"),
        );
        let yamada = load.iter().find(|row| row.assignee == "山田").unwrap();

        // 3/2〜3/10 is nine days; 3/4〜3/6 belongs to both.
        assert_eq!(yamada.busy, 9);
        assert_eq!(yamada.overlap, 3);
        assert_eq!(yamada.tasks, 2);
    }

    /// Finished work is not in anybody's way, and a summary row is its children
    /// counted twice.
    #[test]
    fn what_is_done_and_what_is_a_total_are_left_out() {
        let mut done = row("a", None, "2026-03-02", "2026-03-06", 100);
        done.assignee = "山田".to_owned();
        done.actual_start = Some("2026-03-02".to_owned());
        done.actual_end = Some("2026-03-06".to_owned());

        let mut parent = row("p", None, "", "", 0);
        parent.assignee = "山田".to_owned();
        let mut child = row("c", Some("p"), "2026-03-09", "2026-03-10", 0);
        child.assignee = "山田".to_owned();

        let data = build_for_test("p", 0, date("2026-03-01"), vec![done, parent, child]);
        let load = load(
            &data,
            date("2026-03-01"),
            date("2026-03-31"),
            date("2026-03-01"),
        );
        let yamada = load.iter().find(|row| row.assignee == "山田").unwrap();

        // Only the child: two days.
        assert_eq!(yamada.busy, 2, "{yamada:?}");
        assert_eq!(yamada.tasks, 1);
    }

    /// Work nobody is holding is the thing a capacity page is for.
    #[test]
    fn the_unassigned_get_a_row_of_their_own() {
        let rows = vec![row("a", None, "2026-03-02", "2026-03-04", 0)];
        let data = build_for_test("p", 0, date("2026-03-01"), rows);
        let load = load(
            &data,
            date("2026-03-01"),
            date("2026-03-31"),
            date("2026-03-01"),
        );
        let nobody = load.last().unwrap();

        assert_eq!(nobody.assignee, "");
        assert_eq!(nobody.busy, 3);
        // Nobody has a capacity, so there is no free to report either.
        assert_eq!(nobody.capacity, None);
        assert_eq!(nobody.free, None);
    }

    /// Half a month gone with "twelve days free" in it is a lie by arithmetic:
    /// those twelve include days nobody can use any more.
    #[test]
    fn the_days_already_gone_are_not_free() {
        let mut task = row("a", None, "2026-03-16", "2026-03-20", 0);
        task.assignee = "山田".to_owned();

        let data = build_for_test("p", 0, date("2026-03-15"), vec![task]);
        let load = load(
            &data,
            date("2026-03-01"),
            date("2026-03-31"),
            date("2026-03-15"),
        );
        let yamada = load.iter().find(|row| row.assignee == "山田").unwrap();

        // 3/1〜3/14 is behind us; 3/15 is today and still usable.
        assert_eq!(yamada.gone, Some(14));
        assert_eq!(yamada.busy, 5);
        assert_eq!(yamada.free, Some(31 - 14 - 5));
        // The three add up to the whole window. Any other arrangement leaves a
        // reader wondering which number is lying.
        assert_eq!(
            yamada.capacity,
            Some(yamada.gone.unwrap() + yamada.busy + yamada.free.unwrap())
        );
    }

    /// A window that has already been and gone is all "gone", and nothing else.
    #[test]
    fn a_window_in_the_past_offers_nothing() {
        let mut task = row("a", None, "2026-01-05", "2026-01-09", 0);
        task.assignee = "山田".to_owned();

        let data = build_for_test("p", 0, date("2026-03-15"), vec![task]);
        let load = load(
            &data,
            date("2026-01-01"),
            date("2026-01-31"),
            date("2026-03-15"),
        );
        let yamada = load.iter().find(|row| row.assignee == "山田").unwrap();

        assert_eq!(yamada.gone, Some(31));
        assert_eq!(yamada.busy, 0);
        assert_eq!(yamada.free, Some(0));
    }

    /// A day outside the window is not this month's problem.
    #[test]
    fn only_the_days_inside_the_window_are_counted() {
        let mut task = row("a", None, "2026-02-20", "2026-03-05", 0);
        task.assignee = "山田".to_owned();

        let data = build_for_test("p", 0, date("2026-03-01"), vec![task]);
        let load = load(
            &data,
            date("2026-03-01"),
            date("2026-03-31"),
            date("2026-03-01"),
        );
        let yamada = load.iter().find(|row| row.assignee == "山田").unwrap();

        assert_eq!(yamada.busy, 5);
    }

    /// A collapsed parent has to say that something under it is behind.
    #[test]
    fn a_parent_is_behind_when_a_child_is() {
        let mut kid = row("a", Some("p"), "2026-03-01", "2026-03-10", 0);
        kid.targets = "2026-03-05/50".to_owned();

        let rows = vec![row("p", None, "", "", 0), kid];
        let data = build_for_test("proj", 0, date("2026-03-06"), rows);

        assert!(data.tasks[0].delayed);
        // The parent itself named nothing, so it claims no expected progress.
        assert_eq!(data.tasks[0].expected, None);
    }

    /// Depth-first order is what lets the grid draw indentation without
    /// knowing the tree.
    #[test]
    fn children_follow_their_parent_in_order() {
        let rows = vec![
            row("p", None, "", "", 0),
            row("q", None, "", "", 0),
            row("a", Some("p"), "2026-03-01", "2026-03-02", 0),
        ];

        let data = build_for_test("proj", 0, date("2026-01-01"), rows);
        let order: Vec<&str> = data.tasks.iter().map(|task| task.id.as_str()).collect();

        assert_eq!(order, ["p", "a", "q"]);
        assert_eq!(data.tasks[1].depth, 1);
    }
}
