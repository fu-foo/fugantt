# fugantt

**Plan against actual, counted in working days.**

A Gantt chart you edit like a spreadsheet, for teams who have to answer not just
"when is it due" but "how far off the plan did it run, and why". Rust server,
SQLite, one binary. The grid is plain TypeScript; everything else is HTML.

[日本語の README](README.md)

![The schedule](docs/images/schedule.png)

## The problem it solves

Most schedule tools hold one set of dates. Real projects hold two: what was
planned, and what happened.

- **Planned and actual on one row.** The start and end variances are not stored;
  they are subtracted on every read.
- **Days counted the way your workplace counts them.** Weekends, public
  holidays, leave and waiting periods are excluded — or not, per project.
- **The delay is split.** Not "twelve days late", but "nine days of work, three
  days waiting on another team".
- **Waiting is recorded**, with dates and a reason. Those days count towards
  neither the duration nor the lateness.

| | |
| --- | --- |
| Statistics | ![Statistics](docs/images/stats.png) |
| History | ![History](docs/images/history.png) |

## Editing

The grid is keyboard-first: nobody leaves a spreadsheet for something slower.

Arrows move, Enter opens a cell, Tab goes right, Escape puts it back.
`⌘Enter` / `Ctrl+Enter` adds a row, `⌥→` / `Alt+→` makes it a child, `⌥↑` / `Alt+↑`
moves it within its siblings. Either modifier works on either platform; only the
label on the screen changes.
Bars drag: the body moves the dates, the ends stretch them, and the handle
inside the plan bar sets the progress.

Dates take whatever you type — `20260805`, `8/5`, `2026-08-05` — including
full-width digits, so a Japanese keyboard never has to switch modes.

Filters sit above the columns, one per column, ANDed together. Dates and numbers
compare rather than match: pick **at least / at most / equals / more than / less
than** from the button beside the box. Progress adds two more that need no
number at all — **behind** and **on track**, measured against where today says
the work should be.

## Where it came from

fugantt was built for 予実管理 — the Japanese practice of managing a plan
against its actuals — and that shows in the defaults rather than in the
architecture.

- **Public holidays are computed, not pasted.** `src/holidays.rs` implements the
  rules, including substitute holidays and the "citizens' holiday" that appears
  between two others.
- **The business year sits above the months**, starting in whichever month you
  say (April by default, which is the Japanese norm; October and January are a
  setting away).
- **Saturday is blue and Sunday is red**, the way a Japanese calendar prints
  them. Same-grey weekends are misread.
- **Japanese era years** (`令和8年度 Q2`) are available, and stored as data — a
  new era means adding one line in the settings, not shipping a build.
- The weekday is printed under every date. Counting to a Friday off a month grid
  is not a plan.

None of this is hard-coded to one country: the holiday list, the business year,
the weekdays you skip, and the language are all settings.

## Language

Japanese by default, English when the reader's browser asks for it. The order
is: the person's own setting, then the installation's, then `Accept-Language`
— which browsers fill in from the operating system.

What the users named — statuses, their own fields, people, projects — is data
and is never translated. Two people should not read the same plan in different
words.

## Running it

```sh
brew install fu-foo/tap/fugantt
docker run -p 3000:3000 -v fugantt:/data ghcr.io/fu-foo/fugantt

cargo-topcoat dev            # from source. http://127.0.0.1:3000
```

The database is created at `FUGANTT_DB` (default `fugantt.db`) and migrated on
first start. For a release build, `cargo build --release` produces a single
executable with the static files embedded — deploying is that file and a
database, and nothing else.

SQLite means **one machine**. The same database opened by two servers is two
different plans.

## Getting data in and out

Excel (`.xlsx`) for reading: the same columns as the screen, with the chart
drawn cell by cell to its right. JSON for moving: the whole project — settings,
statuses, people, calendar and tasks — in one file.

The JSON is meant to be read, edited and handed back, including by a program:

```json
{
  "version": 1,
  "name": "Release plan",
  "tasks": [
    {
      "id": "c0ffee…", "name": "Requirements", "depth": 0,
      "start": "2026-08-03", "end": "2026-08-14",
      "actual_start": "2026-08-03", "actual_end": "2026-08-18",
      "progress": 100, "status": "完了", "assignee": "山田",
      "waits": ["2026-08-17/2026-08-21"],
      "fields": { "Product": "A" }
    }
  ]
}
```

| | |
| --- | --- |
| `depth` | From 0. Deeper than the row above means a child of it. |
| `waits` | `"from/to"`. Omit the end (`"from/"`) and it is still waiting. |
| `id` | Present and known: updated. Absent: added. Missing from the file: removed. Leave it out when writing by hand. |
| references | Statuses, people and fields are named, never referenced by id. |
| summary rows | Their dates and progress are not written out: they come from the children. |

**No derived value is in the file** — no day counts, no variance, no expected
progress. Nothing written back can contradict itself. To *read* those, ask
`GET /api/projects/{id}/grid`, which returns the table already computed.

> Read from the grid, write to the document.

## API tokens

A token opens one project, with one role, and is shown once.

```sh
curl -H "Authorization: Bearer fug_…" \
  https://example.com/api/projects/release-plan/document

curl -X POST -H "Authorization: Bearer fug_…" \
  -H "Content-Type: application/json" --data @plan.json \
  https://example.com/api/projects/release-plan/document
```

A token for another project is refused, and a read-only token cannot write. A
change made with a token is recorded as **`API <what the token is for>`** — no
person's name, because nobody did that work, but never anonymous either.

## Roles and sign-in

- **The base roles "editor" and "viewer" apply to every project.** To keep a
  plan to a few people, set the base to "no access" and add them as members.
- The password rule — minimum length, required kinds of character, refused
  words — is set per installation.
- Changing a password ends that person's other sessions.

`FUGANTT_NO_AUTH=yes-everyone-on-this-network-can-edit` runs it without sign-in
at all. **Everyone who can reach that URL can read and edit every project.** A
banner stays on screen while it is on.

## Licence

Apache License 2.0

Copyright 2026 Kazunari Fukagawa
