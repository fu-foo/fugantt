# fugantt

**Plan against actual, counted in working days.**

A Gantt chart you edit like a spreadsheet, for teams who have to answer not just
"when is it due" but "how far off the plan did it run, and why". Rust server,
SQLite, one binary. The grid is plain TypeScript; everything else is HTML.

[日本語の README](README.md)

![The schedule](docs/images/schedule.png)

## The problem it solves

Most schedule tools hold one set of dates. Real projects hold two: what was
planned, and what happened. The gap between them is the thing anyone actually
gets asked about, and it is the thing spreadsheets end up carrying because no
tool made room for it.

fugantt keeps both on one row and works out the rest.

- **Variance is never stored.** The start and end differences are subtracted on
  every read, so they cannot drift away from the dates they came from.
- **Days are counted the way your workplace counts them.** Weekends, public
  holidays, each person's leave, and the periods the work sat waiting are all
  excluded — or not, per project.
- **The delay is split.** "Twelve days late" is not a finding. "Nine days of
  work, three days waiting on another team" is. The statistics page separates
  the part that was work from the part that was somebody else.
- **Waiting is a first-class thing.** A task can record the ranges it was
  blocked, with reasons. Those days count towards neither the duration nor the
  lateness, and the chart hatches them.

| | |
| --- | --- |
| Statistics | ![Statistics](docs/images/stats.png) |
| History | ![History](docs/images/history.png) |

## Editing

The grid is keyboard-first, because the people replacing a spreadsheet are fast
in a spreadsheet and will not accept being slowed down.

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
cargo-topcoat dev            # http://127.0.0.1:3000
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

A token for another project is refused, a read-only token cannot write, and a
change made with a token carries no person's name in the history — nobody did
that work.

## Running it safely

- Passwords are stored as **Argon2** hashes. The raw value is never written
  anywhere, including the log.
- Sessions are 32 bytes of randomness; the database holds only the **SHA-256**.
  The cookie is `__Host-` prefixed, `Secure`, `HttpOnly`, `SameSite=Lax`.
- Every change is a POST, which together with `SameSite=Lax` closes the
  cross-site path without a token dance.
- Every query is parameterised. The island never uses `innerHTML`.
- Changing a password ends that person's other sessions.
- The password rule — minimum length, required kinds of character, a list of
  refused words — is set per installation.

Two environment variables trade a defence for reach and say so loudly at
startup: `FUGANTT_NO_AUTH` (no sign-in at all, refused if the host is public)
and `FUGANTT_ALLOW_HTTP` (drops `Secure` from the cookie). Neither belongs on a
machine reachable from outside.

**Base roles apply to every project.** Set someone to "no access" and add them
to the projects they should see, if a plan needs to stay private within the
company.

## How it is built

- `src/domain.rs` owns every derived value: durations, variances, expected
  progress, roll-ups. The browser draws; it does not decide.
- The grid is an island. It fetches its own data and owns that subtree; the rest
  of the app is server-rendered HTML with no client framework.
- Live updates over SSE. Somebody else's edit arrives on your screen.
- Tests: `cargo test` for the domain, and a browser suite driving a real Chrome
  against a running server for the grid (`web/test/grid.test.mjs`). The second
  one exists because "the element is there" is not a test — computed style and
  geometry are.

## Licence

**Apache License 2.0.** Commercial use, modification and redistribution are all
permitted, with no obligation to publish your changes, and the patent grant is
explicit.

Copyright 2026 Kazunari Fukagawa
