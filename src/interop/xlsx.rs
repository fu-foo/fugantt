//! Excel export: the chart redrawn with filled cells.
//!
//! One column per day, so the result is a spreadsheet someone can keep editing
//! rather than a picture of one. The bar is a run of filled cells, dark for the
//! part reported done and light for the rest.

use jiff::civil::Date;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, XlsxError};

use crate::domain::GridData;

const MONTH_ROW: u32 = 0;
const DAY_ROW: u32 = 1;
const FIRST_TASK_ROW: u32 = 2;

/// The table beside the chart, in the grid's own order.
///
/// The export used to carry four columns while the grid grew to sixteen, which
/// made the spreadsheet a worse copy of the plan than the screen it came from.
const COLUMNS: [(&str, &str, f64); 16] = [
    ("name", "タスク", 34.0),
    ("late", "遅延", 6.0),
    ("assignee", "担当者", 10.0),
    ("status", "ステータス", 10.0),
    ("start", "予定開始", 12.0),
    ("end", "予定終了", 12.0),
    ("days", "予定日数", 8.0),
    ("targets", "予定進捗", 14.0),
    ("actual_start", "実施開始", 12.0),
    ("actual_end", "実施終了", 12.0),
    ("actual_days", "実作業日数", 8.0),
    ("progress", "実進捗", 7.0),
    ("start_variance", "開始差異", 9.0),
    ("end_variance", "終了差異", 9.0),
    ("waits", "待ち", 16.0),
    ("note", "コメント", 24.0),
];

/// The columns this project shows, built-in ones first.
///
/// Hidden columns are hidden here too: the export is meant to be the same plan,
/// not a different one.
fn visible_columns(data: &GridData, lang: crate::i18n::Lang) -> Vec<(&str, String, f64)> {
    let mut columns: Vec<(&str, String, f64)> = COLUMNS
        .iter()
        .filter(|(key, ..)| *key == "name" || !data.hidden_columns.iter().any(|c| c == key))
        // Headings in the reader's language. A project's own field names are the
        // users' words: not in the dictionary, and written as they are.
        .map(|(key, label, width)| (*key, lang.t(label).to_owned(), *width))
        .chain(
            data.fields
                .iter()
                .map(|field| (field.id.as_str(), field.label.clone(), 14.0)),
        )
        .collect();

    // Same order as the screen: the export is meant to be the same plan, and a
    // column that moved on screen has moved for a reason.
    let rank = |key: &str| {
        data.column_order
            .iter()
            .position(|column| column == key)
            .unwrap_or(usize::MAX)
    };
    columns.sort_by_key(|(key, ..)| rank(key));

    columns
}

/// What one cell of the table says, as the grid would say it.
fn cell_text(task: &crate::domain::TaskView, key: &str) -> String {
    let date = |value: &Option<String>| value.clone().unwrap_or_default();

    match key {
        // Excel has no cell indent that survives a plain string, so the outline
        // is spelled out with spaces.
        "name" => format!("{}{}", "    ".repeat(task.depth), task.name),
        "start" => date(&task.start),
        "end" => date(&task.end),
        "actual_start" => date(&task.actual_start),
        "actual_end" => date(&task.actual_end),
        "days" => task.days.map(|days| days.to_string()).unwrap_or_default(),
        "start_variance" => variance(task.start_variance),
        "end_variance" => variance(task.end_variance),
        "actual_days" => task
            .actual_days
            .map(|days| days.to_string())
            .unwrap_or_default(),
        "targets" => task
            .targets
            .iter()
            .map(|target| format!("{} {}%", target.date, target.percent))
            .collect::<Vec<_>>()
            .join(", "),
        "late" => match task.delayed || task.overdue > 0 {
            true => "遅延".to_owned(),
            false => String::new(),
        },
        "progress" => format!("{}%", task.progress),
        "status" => task.status.clone(),
        "assignee" => task.assignee.clone(),
        "note" => task.note.clone(),
        "waits" => task
            .waits
            .iter()
            .map(|span| format!("{}〜{}", span.start, span.end))
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    }
}

/// Signed day counts read as differences, not quantities: `+3日`, `-2日`, `±0`.
fn variance(days: Option<i64>) -> String {
    match days {
        None => String::new(),
        Some(0) => "±0".to_owned(),
        Some(days) if days > 0 => format!("+{days}日"),
        Some(days) => format!("{days}日"),
    }
}

/// Renders the project as an `.xlsx` file.
pub fn write(
    project_name: &str,
    data: &GridData,
    lang: crate::i18n::Lang,
) -> Result<Vec<u8>, XlsxError> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name(sheet_name(project_name))?;

    let heading = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x00F1_F5F9))
        .set_border(FormatBorder::Thin)
        .set_border_color(Color::RGB(0x00CB_D5E1));

    let day_heading = heading.clone().set_align(FormatAlign::Center);
    let today_heading = day_heading.clone().set_font_color(Color::RGB(0x00DC_2626));

    let plain = Format::new();
    let summary = Format::new().set_bold();
    let late = Format::new()
        .set_font_color(Color::RGB(0x00DC_2626))
        .set_bold();
    let date_cell = Format::new().set_align(FormatAlign::Center);
    let percent_cell = Format::new().set_align(FormatAlign::Right);

    let palette = |hex: &str| Color::RGB(hex_to_rgb(hex));

    let saturday = Format::new().set_background_color(palette(&data.theme.saturday));
    let sunday = Format::new().set_background_color(palette(&data.theme.sunday));
    let holiday = Format::new().set_background_color(palette(&data.theme.holiday));
    let holiday_heading = day_heading
        .clone()
        .set_background_color(palette(&data.theme.holiday));
    let saturday_heading = day_heading
        .clone()
        .set_background_color(palette(&data.theme.saturday));
    let sunday_heading = day_heading
        .clone()
        .set_background_color(palette(&data.theme.sunday));
    let done = Format::new().set_background_color(palette(&data.theme.done));
    let planned = Format::new().set_background_color(palette(&data.theme.bar));
    let done_late = Format::new().set_background_color(palette(&data.theme.late));
    let planned_late = Format::new().set_background_color(Color::RGB(lighten(&data.theme.late)));
    let done_summary = Format::new().set_background_color(palette(&data.theme.summary));
    let planned_summary =
        Format::new().set_background_color(Color::RGB(lighten(&data.theme.summary)));

    // Waiting: the work stopped. Violet, as on screen.
    let waiting = Format::new()
        .set_background_color(Color::RGB(0x00ED_E9FE))
        .set_pattern(rust_xlsxwriter::FormatPattern::LightUp)
        .set_foreground_color(Color::RGB(0x00A7_8BFA));

    // Leave reads as "nobody was on this", the same as a wait but greyer: the
    // work did not stall, the person was away.
    let on_leave = Format::new()
        .set_background_color(Color::RGB(0x00E2_E8F0))
        .set_pattern(rust_xlsxwriter::FormatPattern::LightUp)
        .set_foreground_color(Color::RGB(0x0094_A3B8));

    // --- header ------------------------------------------------------------

    // The columns the project chose to hide are hidden here too: the export is
    // meant to be the same plan, not a different one.
    let columns = visible_columns(data, lang);

    for (index, (_, label, width)) in columns.iter().enumerate() {
        let column = u16::try_from(index).unwrap_or(0);
        sheet.write_with_format(DAY_ROW, column, label.as_str(), &heading)?;
        sheet.set_column_width(column, *width)?;
    }

    let first_day_column = u16::try_from(columns.len()).unwrap_or(0);

    // A project with no usable range still produces a valid, if empty, sheet.
    let Some(origin) = parse(&data.range_start) else {
        return workbook.save_to_buffer();
    };

    let days = span(origin, parse(&data.range_end).unwrap_or(origin));
    let today = parse(&data.today);
    let holidays: std::collections::HashSet<&str> = data
        .holidays
        .iter()
        .map(|holiday| holiday.date.as_str())
        .collect();
    let is_off = |date: Date| is_weekend(date) || holidays.contains(date.to_string().as_str());

    let mut month_start = 0;

    for offset in 0..=days {
        let column = first_day_column + u16::try_from(offset).unwrap_or(u16::MAX);
        let date = origin.checked_add(jiff::Span::new().days(offset)).ok();

        // Close the running month at each boundary and at the end of the range.
        let boundary = offset == days || date.is_some_and(|date| date.day() == 1);

        if boundary && offset > month_start {
            let first = origin
                .checked_add(jiff::Span::new().days(month_start))
                .unwrap_or(origin);
            let from = first_day_column + u16::try_from(month_start).unwrap_or(0);
            let to = column - 1;
            let label = format!("{}年{}月", first.year(), first.month());

            if from == to {
                sheet.write_with_format(MONTH_ROW, from, label, &heading)?;
            } else {
                sheet.merge_range(MONTH_ROW, from, MONTH_ROW, to, &label, &heading)?;
            }

            month_start = offset;
        }

        let Some(date) = date else { continue };
        if offset == days {
            break;
        }

        let format = if today == Some(date) {
            &today_heading
        } else if holidays.contains(date.to_string().as_str()) {
            &holiday_heading
        } else if date.weekday() == jiff::civil::Weekday::Saturday {
            &saturday_heading
        } else if date.weekday() == jiff::civil::Weekday::Sunday {
            &sunday_heading
        } else {
            &day_heading
        };

        sheet.write_with_format(DAY_ROW, column, i64::from(date.day()), format)?;
        sheet.set_column_width(column, 3)?;
    }

    // --- rows --------------------------------------------------------------

    for (index, task) in data.tasks.iter().enumerate() {
        let row = FIRST_TASK_ROW + u32::try_from(index).unwrap_or(0);

        let name_format = if task.delayed {
            &late
        } else if task.has_children {
            &summary
        } else {
            &plain
        };

        for (position, (key, ..)) in columns.iter().enumerate() {
            let column = u16::try_from(position).unwrap_or(0);
            let text = match *key {
                key if COLUMNS.iter().any(|(known, ..)| *known == key) => cell_text(task, key),
                // Anything else is one of the project's own fields.
                field => task.values.get(field).cloned().unwrap_or_default(),
            };

            let format = match *key {
                "name" => name_format,
                "progress" | "days" => &percent_cell,
                "start" | "end" | "actual_start" | "actual_end" | "wait_start" | "wait_until" => {
                    &date_cell
                }
                "start_variance" | "end_variance" => &percent_cell,
                _ => &plain,
            };

            sheet.write_with_format(row, column, text, format)?;
        }

        let range = |from: &Option<String>, to: &Option<String>| {
            from.as_deref()
                .and_then(parse)
                .zip(to.as_deref().and_then(parse))
        };

        // The days this row's assignee is away.
        let away: Vec<(Date, Date)> = data
            .leaves
            .iter()
            .filter(|leave| {
                !task.assignee.trim().is_empty() && leave.assignee.trim() == task.assignee.trim()
            })
            .filter_map(|leave| parse(&leave.start).zip(parse(&leave.end)))
            .collect();

        let bar = range(&task.start, &task.end);
        // An unfinished task is drawn up to today, the same as on screen.
        let actual = task.actual_start.as_deref().and_then(parse).map(|start| {
            let end = task
                .actual_end
                .as_deref()
                .and_then(parse)
                .unwrap_or_else(|| parse(&data.today).unwrap_or(start).max(start));
            (start, end)
        });

        let within = |span: Option<(Date, Date)>, date: Date| {
            span.is_some_and(|(start, end)| date >= start && date <= end)
        };

        for offset in 0..days {
            let column = first_day_column + u16::try_from(offset).unwrap_or(u16::MAX);
            let Ok(date) = origin.checked_add(jiff::Span::new().days(offset)) else {
                continue;
            };

            // The plan and what happened are layered the way the chart layers
            // them: the plan is the ground, the work sits on top of it, and a
            // wait is cut back out of the work.
            let waits: Vec<(Date, Date)> = task
                .waits
                .iter()
                .filter_map(|span| parse(&span.start).zip(parse(&span.end)))
                .collect();

            let format = if within(bar, date)
                && waits.iter().any(|(from, to)| date >= *from && date <= *to)
            {
                &waiting
            } else if within(bar, date)
                && away.iter().any(|(from, to)| date >= *from && date <= *to)
            {
                &on_leave
            } else if within(actual, date) {
                if task.has_children {
                    &done_summary
                } else {
                    &done
                }
            } else if let Some((start, end)) = bar.filter(|_| within(bar, date)) {
                if actual.is_some() {
                    // With an actual bar in the row, the plan is the outline
                    // behind it rather than a progress gauge.
                    if task.delayed {
                        &planned_late
                    } else {
                        &planned
                    }
                } else {
                    let length = span(start, end);
                    // The done part is the leading share of the bar's length.
                    let filled = (length * task.progress + 99) / 100;
                    let is_done = span(start, date) <= filled;

                    match (task.delayed, task.has_children, is_done) {
                        (true, _, true) => &done_late,
                        (true, _, false) => &planned_late,
                        (_, true, true) => &done_summary,
                        (_, true, false) => &planned_summary,
                        (_, _, true) => &done,
                        (_, _, false) => &planned,
                    }
                }
            } else if holidays.contains(date.to_string().as_str()) {
                &holiday
            } else if date.weekday() == jiff::civil::Weekday::Saturday {
                &saturday
            } else if date.weekday() == jiff::civil::Weekday::Sunday {
                &sunday
            } else if is_off(date) {
                &saturday
            } else {
                continue;
            };

            sheet.write_blank(row, column, format)?;
        }
    }

    // Keep the names and the dates on screen while scrolling through the year.
    sheet.set_freeze_panes(FIRST_TASK_ROW, first_day_column)?;

    workbook.save_to_buffer()
}

/// Excel rejects sheet names over 31 characters or containing `[]:*?/\`.
fn sheet_name(project_name: &str) -> String {
    let cleaned: String = project_name
        .chars()
        .filter(|c| !"[]:*?/\\".contains(*c))
        .take(31)
        .collect();

    if cleaned.trim().is_empty() {
        "スケジュール".to_owned()
    } else {
        cleaned
    }
}

/// `#rgb` or `#rrggbb` to the 0x00RRGGBB Excel wants. Anything else is black,
/// which the settings form already prevents from being stored.
fn hex_to_rgb(hex: &str) -> u32 {
    let digits = hex.trim_start_matches('#');

    let expanded = if digits.len() == 3 {
        digits.chars().flat_map(|c| [c, c]).collect()
    } else {
        digits.to_owned()
    };

    u32::from_str_radix(&expanded, 16).unwrap_or(0)
}

/// The unfinished part of a bar, as a wash of the same hue.
fn lighten(hex: &str) -> u32 {
    let rgb = hex_to_rgb(hex);
    let mix = |channel: u32| (channel + (255 - channel) * 4 / 5) & 0xFF;

    (mix((rgb >> 16) & 0xFF) << 16) | (mix((rgb >> 8) & 0xFF) << 8) | mix(rgb & 0xFF)
}

fn parse(text: &str) -> Option<Date> {
    text.parse().ok()
}

fn is_weekend(date: Date) -> bool {
    matches!(
        date.weekday(),
        jiff::civil::Weekday::Saturday | jiff::civil::Weekday::Sunday
    )
}

/// Inclusive day count, never less than one.
fn span(start: Date, end: Date) -> i64 {
    (i64::from(end.since(start).map(|span| span.get_days()).unwrap_or(0)) + 1).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TaskView;

    fn data() -> GridData {
        GridData {
            language: "ja".to_owned(),
            project_id: "p".to_owned(),
            revision: 1,
            today: "2026-08-11".to_owned(),
            range_start: "2026-08-01".to_owned(),
            range_end: "2026-08-31".to_owned(),
            leaves: Vec::new(),
            assignees: Vec::new(),
            statuses: crate::domain::Status::defaults(),
            holidays: vec![crate::domain::Holiday {
                date: "2026-08-11".to_owned(),
                name: "山の日".to_owned(),
            }],
            theme: crate::domain::Theme::default(),
            fields: Vec::new(),
            hidden_columns: Vec::new(),
            column_order: Vec::new(),
            column_widths: std::collections::HashMap::new(),
            frozen_columns: 1,
            counting: crate::domain::Counting::default(),
            fiscal_year_start: 4,
            japanese_era: false,
            quarters: true,
            eras: Vec::new(),
            day_width: 26,
            can_edit: true,
            tasks: vec![TaskView {
                id: "t".to_owned(),
                depth: 0,
                name: "設計".to_owned(),
                start: Some("2026-08-03".to_owned()),
                end: Some("2026-08-14".to_owned()),
                progress: 50,
                days: Some(12),
                actual_start: None,
                actual_end: None,
                actual_days: None,
                start_variance: None,
                end_variance: None,
                overdue: 0,
                waits: Vec::new(),
                wait_days: 0,
                status: "実施中".to_owned(),
                assignee: String::new(),
                note: String::new(),
                targets: Vec::new(),
                color: String::new(),
                background: String::new(),
                expected: Some(60),
                delayed: true,
                has_children: false,
                tags: Vec::new(),
                values: std::collections::HashMap::new(),
            }],
        }
    }

    #[test]
    fn it_produces_a_readable_workbook() {
        let bytes = write("リリース計画", &data(), crate::i18n::Lang::Ja).unwrap();

        // An xlsx is a zip; the magic number is the cheapest proof it is one.
        assert_eq!(&bytes[..2], b"PK");
        assert!(bytes.len() > 1000, "{} バイトしかない", bytes.len());
    }

    /// The export carried four columns while the grid grew to sixteen. Naming
    /// them here keeps the two from drifting apart again unnoticed.
    #[test]
    fn the_table_matches_the_grid() {
        let mut data = data();
        let labels = |data: &GridData| {
            visible_columns(data, crate::i18n::Lang::Ja)
                .into_iter()
                .map(|(_, label, _)| label)
                .collect::<Vec<_>>()
                .join(",")
        };

        assert_eq!(
            labels(&data),
            "タスク,遅延,担当者,ステータス,予定開始,予定終了,予定日数,予定進捗,\
             実施開始,実施終了,実作業日数,実進捗,開始差異,終了差異,待ち,コメント"
        );

        data.hidden_columns = vec!["status".to_owned(), "note".to_owned()];
        let shown = labels(&data);
        assert!(!shown.contains("ステータス"), "{shown}");
        assert!(!shown.contains("コメント"), "{shown}");
    }

    #[test]
    fn differences_read_as_differences() {
        assert_eq!(variance(Some(3)), "+3日");
        assert_eq!(variance(Some(-2)), "-2日");
        assert_eq!(variance(Some(0)), "±0");
        assert_eq!(variance(None), "");
    }

    /// Excel silently refuses to open a file whose sheet name breaks its rules.
    #[test]
    fn sheet_names_are_made_legal() {
        assert_eq!(sheet_name("a/b:c"), "abc");
        assert_eq!(sheet_name("   "), "スケジュール");
        assert_eq!(sheet_name(&"あ".repeat(50)).chars().count(), 31);
    }
}
