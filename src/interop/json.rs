//! JSON in and out: the whole plan as one file.
//!
//! Excel is for reading and JSON is for moving — between installations, into a
//! backup, or through a script somebody writes in an afternoon. The shape is
//! deliberately flat: a list of tasks carrying their own depth, which is how
//! the grid draws them anyway and what a person can edit by hand without
//! tracking brackets.

use serde::{Deserialize, Serialize};

use crate::domain::GridData;

/// What the grid's data does not carry, read from the database by the caller.
pub struct Extras {
    pub settings: std::collections::BTreeMap<String, String>,
    pub assignees: Vec<Assignee>,
}

/// Everything a project is, in one file.
///
/// The theme is "pull it all out and you can take it all with you": settings,
/// the master lists, the calendar, and the plan. Every section is optional on
/// the way back in — a file without one leaves that part of the project alone,
/// so a hand-written file with just tasks still works.
/// The format this file is written in.
///
/// Absent means the first shape, which had no version at all. A file from a
/// newer fugantt is refused rather than half-understood.
pub const VERSION: u32 = 1;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Document {
    #[serde(default)]
    pub version: u32,
    pub name: String,
    /// The project's settings, as stored: colours, counting, columns, the memo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statuses: Option<Vec<Status>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignees: Option<Vec<Assignee>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holidays: Option<Vec<Holiday>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leaves: Option<Vec<Leave>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<Field>>,
    #[serde(default)]
    pub tasks: Vec<Task>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Status {
    pub name: String,
    #[serde(default)]
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent: Option<i64>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Assignee {
    pub name: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub background: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Holiday {
    pub date: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Leave {
    pub assignee: String,
    pub start: String,
    pub end: String,
    #[serde(default)]
    pub note: String,
    /// `off` for a day away, `on` for a day worked regardless. Away by default.
    #[serde(default = "leave_off")]
    pub kind: String,
}

fn leave_off() -> String {
    "off".to_owned()
}

/// A project's own column, with the master list behind it.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Field {
    pub label: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<Choice>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Choice {
    pub value: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub background: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Task {
    /// The row's id, so a file can be edited and handed back without the rows
    /// being taken for new ones.
    ///
    /// A task carrying an id that this project knows is updated in place;
    /// anything else is added; a row the project has and the file does not is
    /// removed. Written out on export, and safe to leave off in a file somebody
    /// wrote by hand.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    pub name: String,
    /// Indent level, as in the grid. A row deeper than the one above it is its
    /// child; anything deeper than one step is pulled back to one step.
    #[serde(default)]
    pub depth: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_end: Option<String>,
    #[serde(default)]
    pub progress: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub status: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub assignee: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// Waiting periods: `["2026-08-17/2026-08-21"]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waits: Vec<String>,
    /// 予定進捗: `["2026-08-20/30", "2026-08-28/100"]` — by this date, this
    /// much. The one plan a derived value could never guess.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<String>,
    /// The colours somebody gave this row, `#rrggbb`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub background: String,
    /// The project's own columns, by their label rather than their id — an id
    /// means nothing in another installation.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub fields: std::collections::BTreeMap<String, String>,
}

/// The project as a document, pretty-printed: these files get read by people.
///
/// `extras` carries what the grid does not: the stored settings and the master
/// lists, which the caller reads from the database. `None` writes the plan on
/// its own — for anyone taking the tasks somewhere else, where a page of
/// colours and holiday dates is noise to read past.
pub fn write(project_name: &str, data: &GridData, extras: Option<Extras>) -> String {
    let labels: std::collections::HashMap<&str, &str> = data
        .fields
        .iter()
        .map(|field| (field.id.as_str(), field.label.as_str()))
        .collect();

    let with_settings = extras.is_some();
    let (settings, assignees) = match extras {
        Some(extras) => (Some(extras.settings), Some(extras.assignees)),
        None => (None, None),
    };
    // Every section is optional on the way back in, so leaving them out writes a
    // file that still imports — it just says nothing about the parts it omits.
    fn kept<T>(keep: bool, section: T) -> Option<T> {
        keep.then_some(section)
    }

    let document = Document {
        version: VERSION,
        name: project_name.to_owned(),
        settings,
        statuses: kept(
            with_settings,
            data.statuses
                .iter()
                .map(|status| Status {
                    name: status.name.clone(),
                    color: status.color.clone(),
                    percent: status.percent,
                })
                .collect(),
        ),
        assignees,
        holidays: kept(
            with_settings,
            data.holidays
                .iter()
                .map(|holiday| Holiday {
                    date: holiday.date.clone(),
                    name: holiday.name.clone(),
                })
                .collect(),
        ),
        leaves: kept(
            with_settings,
            data.leaves
                .iter()
                .map(|leave| Leave {
                    assignee: leave.assignee.clone(),
                    start: leave.start.clone(),
                    end: leave.end.clone(),
                    note: leave.note.clone(),
                    kind: leave.kind.clone(),
                })
                .collect(),
        ),
        fields: kept(
            with_settings,
            data.fields
                .iter()
                .map(|field| Field {
                    label: field.label.clone(),
                    kind: field.kind.clone(),
                    options: field
                        .options
                        .iter()
                        .map(|option| Choice {
                            value: option.value.clone(),
                            color: option.color.clone(),
                            background: option.background.clone(),
                        })
                        .collect(),
                })
                .collect(),
        ),
        tasks: data
            .tasks
            .iter()
            .map(|task| Task {
                id: task.id.clone(),
                name: task.name.clone(),
                depth: task.depth,
                // A summary row's dates come from its children, so writing them
                // out would make the file disagree with itself once anything
                // moves. They are rebuilt on the way back in.
                start: (!task.has_children).then(|| task.start.clone()).flatten(),
                end: (!task.has_children).then(|| task.end.clone()).flatten(),
                actual_start: (!task.has_children)
                    .then(|| task.actual_start.clone())
                    .flatten(),
                actual_end: (!task.has_children)
                    .then(|| task.actual_end.clone())
                    .flatten(),
                progress: if task.has_children { 0 } else { task.progress },
                status: task.status.clone(),
                assignee: task.assignee.clone(),
                note: task.note.clone(),
                waits: task
                    .waits
                    .iter()
                    .map(|span| format!("{}/{}", span.start, span.end))
                    .collect(),
                targets: task
                    .targets
                    .iter()
                    .map(|target| format!("{}/{}", target.date, target.percent))
                    .collect(),
                color: task.color.clone(),
                background: task.background.clone(),
                fields: task
                    .values
                    .iter()
                    .filter_map(|(id, value)| {
                        labels
                            .get(id.as_str())
                            .map(|label| ((*label).to_owned(), value.clone()))
                    })
                    .collect(),
            })
            .collect(),
    };

    serde_json::to_string_pretty(&document).unwrap_or_else(|_| "{}".to_owned())
}

/// Reads a document, refusing anything that is not one.
pub fn read(text: &str) -> Result<Document, String> {
    let mut document: Document =
        serde_json::from_str(text).map_err(|error| format!("JSON として読めません: {error}"))?;

    if document.version > VERSION {
        return Err(format!(
            "このファイルは新しい形式（version {}）です。fugantt を新しくしてください。",
            document.version
        ));
    }

    // A depth that jumps two levels has no parent to attach to, so it is pulled
    // back to one step deeper than the row above it.
    let mut previous = 0;
    for task in &mut document.tasks {
        task.depth = task.depth.min(previous + 1);
        previous = task.depth;
        task.progress = task.progress.clamp(0, 100);
    }

    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{GridData, TaskView};

    fn view(name: &str, depth: usize, has_children: bool) -> TaskView {
        TaskView {
            name: name.to_owned(),
            depth,
            has_children,
            start: Some("2026-08-03".to_owned()),
            end: Some("2026-08-14".to_owned()),
            progress: 40,
            status: "実施中".to_owned(),
            ..TaskView::default()
        }
    }

    fn extras() -> Extras {
        Extras {
            settings: std::collections::BTreeMap::from([(
                "memo".to_owned(),
                "覚え書き".to_owned(),
            )]),
            assignees: vec![Assignee {
                name: "山田".to_owned(),
                color: "#1e3a8a".to_owned(),
                background: "#dbeafe".to_owned(),
            }],
        }
    }

    fn data(tasks: Vec<TaskView>) -> GridData {
        GridData {
            tasks,
            ..GridData::empty("p", 1)
        }
    }

    #[test]
    fn a_document_survives_the_round_trip() {
        let text = write(
            "リリース計画",
            &data(vec![view("要件定義", 0, false)]),
            Some(extras()),
        );
        let document = read(&text).unwrap();

        assert_eq!(document.name, "リリース計画");
        assert_eq!(document.tasks.len(), 1);
        assert_eq!(document.tasks[0].name, "要件定義");
        assert_eq!(document.tasks[0].start.as_deref(), Some("2026-08-03"));
        assert_eq!(document.tasks[0].progress, 40);
    }

    /// A summary row's schedule is its children's; writing it out would make
    /// the file disagree with itself the moment a child moves.
    #[test]
    fn summary_rows_carry_no_schedule() {
        let text = write(
            "p",
            &data(vec![view("開発", 0, true), view("設計", 1, false)]),
            Some(extras()),
        );
        let document = read(&text).unwrap();

        assert_eq!(document.tasks[0].start, None);
        assert_eq!(document.tasks[1].start.as_deref(), Some("2026-08-03"));
    }

    /// Pull it all out and you can take it all with you: the settings and the
    /// lists travel in the same file.
    /// The plan on its own, for whoever is taking the tasks somewhere else.
    ///
    /// Every section is optional on the way back in, so a file written this way
    /// still imports — it simply says nothing about the parts it left out, and
    /// those parts of the project stay as they are.
    #[test]
    fn the_settings_can_be_left_out() {
        let text = write("p", &data(vec![view("要件定義", 0, false)]), None);
        let document: serde_json::Value = serde_json::from_str(&text).unwrap();

        for section in [
            "settings",
            "statuses",
            "assignees",
            "holidays",
            "leaves",
            "fields",
        ] {
            assert!(
                document.get(section).is_none(),
                "{section} が残っている: {text}"
            );
        }

        assert_eq!(document["name"], "p");
        assert_eq!(document["tasks"].as_array().unwrap().len(), 1);

        // And it still reads back as a document.
        let read = read(&text).unwrap();
        assert_eq!(read.tasks.len(), 1);
        assert!(read.settings.is_none());
    }

    #[test]
    fn the_file_carries_the_whole_project() {
        let text = write("p", &data(vec![view("要件定義", 0, false)]), Some(extras()));
        let document = read(&text).unwrap();

        assert_eq!(
            document.settings.as_ref().and_then(|map| map.get("memo")),
            Some(&"覚え書き".to_owned())
        );
        assert_eq!(
            document
                .assignees
                .as_ref()
                .map(|list| list[0].background.clone()),
            Some("#dbeafe".to_owned())
        );
        // Statuses are written out even when they are the shipped defaults.
        assert!(document.statuses.is_some_and(|list| !list.is_empty()));
    }

    /// A hand-written file of nothing but tasks works, and leaves the settings
    /// as they are.
    #[test]
    fn a_file_of_only_tasks_still_works() {
        let document = read(r#"{"name":"p","tasks":[{"name":"a"}]}"#).unwrap();

        assert!(document.settings.is_none());
        assert!(document.statuses.is_none());
        assert_eq!(document.tasks.len(), 1);
    }

    #[test]
    fn an_impossible_indent_is_pulled_back() {
        let document =
            read(r#"{"name":"p","tasks":[{"name":"a"},{"name":"b","depth":5}]}"#).unwrap();

        assert_eq!(document.tasks[1].depth, 1);
    }

    /// A file that came out of fugantt carries the ids, so the same rows can be
    /// found again when it goes back in.
    #[test]
    fn the_rows_keep_their_names_on_the_way_out() {
        let mut task = view("要件定義", 0, false);
        task.id = "t-req".to_owned();

        let text = write("p", &data(vec![task]), Some(extras()));
        let document = read(&text).unwrap();

        assert_eq!(document.tasks[0].id, "t-req");
        assert_eq!(document.version, VERSION);
    }

    /// A hand-written file without ids is still a file. Every row in it is new.
    #[test]
    fn a_file_without_ids_still_reads() {
        let document = read(r#"{"name":"p","tasks":[{"name":"a"}]}"#).unwrap();

        assert!(document.tasks[0].id.is_empty());
        assert_eq!(document.version, 0);
    }

    /// A newer file is refused rather than half-understood.
    #[test]
    fn a_newer_format_is_refused() {
        let error = read(r#"{"version":99,"name":"p","tasks":[]}"#).unwrap_err();

        assert!(error.contains("99"), "{error}");
    }

    #[test]
    fn nonsense_is_refused() {
        assert!(read("これは JSON ではない").is_err());
    }
}
