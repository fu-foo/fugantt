# Guide

[README](../README.en.md) · **Guide** · [Reference](reference.en.md) · [Design](design.en.md) · [日本語](guide.md)

- [Editing the table](#editing-the-table)
- [The chart](#the-chart)
- [Due dates](#due-dates)
- [Plan and actual](#plan-and-actual)
- [Checkpoints and lateness](#checkpoints-and-lateness)
- [Availability](#availability)
- [Statistics and task history](#statistics-and-task-history)
- [Filters](#filters)
- [Settings](#settings)
- [Export and import](#export-and-import)
- [How it looks](#how-it-looks)
- [Language](#language)

## Editing the table

Arrows move; Enter (or F2, a double-click, or just typing) opens a cell; Escape puts it back.
Enter confirms and moves down — or right, in a menu or a date. Tab moves right. The computed columns (day counts,
variances) are stepped over.

- **⌘Enter / Ctrl+Enter** adds a row below, ready to type its name
- **⌥→ / Alt+→** makes a row a child, **⌥← / Alt+←** brings it back. **⌥↑↓ / Alt+↑↓** reorders it
- **Right-click** gives the same moves by name, and the row's own **colours** (background and text)

The full list is in the [reference](reference.en.md#keys). `⌘` and `Ctrl` both work on either platform.

### Typing dates

A date is read **by its number of digits**.

| Type | Get |
| --- | --- |
| `5` | the 5th of this month |
| `305` | March 5th this year (three digits get a leading 0: `115` is January 15th) |
| `1225` | December 25th this year |
| `20261225` | as written |
| `8/5`, `2026-08-05` | as written; full-width digits too |

**It never rolls forward.** Typing `3` on 12/28 gives 12/03, in the past.

### Columns

Drag **the right edge of a heading** to resize a column (kept in this browser; double-click to reset). The border
between table and chart moves the same way. Computed columns have a pale heading.

### Undo

**⌘Z / Ctrl+Z** undoes, **⌘Y / ⌘⇧Z / Ctrl+Y** redoes. Only changes made **in this tab**.

- Values, **an added row** (removed again) and **a move** (back to its parent and place)
- If somebody else has touched the same cell or row in between, it stops and says so
- A row with anything in it is not removed by an undo
- **Deleting cannot be undone.** It asks first
- Reloading clears the history

## The chart

- **Today**, top left, brings you back to today
- **Both the table and the chart can be grabbed and pulled sideways.** Until it moves a few pixels a press is still a click
- **Bars drag.** The ends (7px) stretch only the start or the end. Actual bars too. A row in progress (no actual end) does not gain an end date by being moved
- **The handle inside the plan bar** sets the progress
- **Pointing at a bar** shows its dates and whatever the settings chose — including columns off the table, and your own fields
- **Summary rows draw no bar by default.** A folded summary row draws its bar, and the leave of the people folded under it.
  **Display** at the top left of the chart shows them always (for your screen only)

## Due dates

A task that just has to be **done by a date** gets only a **due date**.

The chart shows **a downward mark at the top of the row**. No bar is drawn, and in Availability it **uses none of
that person's days**.

Due date and plan **can both be set**. "The customer needs it by 11/30; we plan 11/10–11/25" is one row, and the
gap between the bar's end and the mark is your slack.

A summary row's due date is **the latest of its children**. Lateness is drawn up from the children (a child that
misses its date makes the parent late).

## Plan and actual

Five dates: **due, planned start, planned end, actual start, actual end**. The **start and end variances** are
subtracted on every read. On the chart the plan is the outline, the actual is the solid line beneath it, and the
due date is the mark at the top.

### Progress

A new project starts with progress **from the status**: changing the status sets the percentage that status
stands for (完了 → 100%, 未着手 → 0%; a status with no percentage leaves the number to be typed). To type every
number yourself, set "How progress is set" to "Typed in" in the project settings.

Either way, setting 100% fills in today as the actual end if it is empty. Progress is painted inside the plan bar.

### Waiting

Record a stop with **its dates and a reason**. Leave the end open and it is still waiting, up to today. Waiting
days are left out of the day count, the lateness and Availability. On the chart they are hatched.
**Enter them on child tasks** — not on summary rows.

### Comments

Written in a dialog. Enter is a new line; **⌘Enter / Ctrl+Enter saves**. Up to 500 characters. The table shows
one line; pointing at it shows the rest.

### Leave and working days

From the buttons on the schedule. Leave belongs to the person, so it counts in every project they appear in.
"A Saturday, but this one we work" is a working day.

## Checkpoints and lateness

### Checkpoints

**N% by this date**, entered in the same dialog as waiting. Once the date passes with the actual progress short of
it, the row turns red.

- Only the dates you entered are checked. 30% on 8/20 and 100% on 8/28 means those two days, nothing between
- On the chart, the shortfall is drawn as a band continuing the progress fill, with `8/5 50%` at its end. The band
  ends at 50% of the bar, not at the date
- Reached, the band disappears. Checkpoints still ahead show as a thin mark. **Display** hides them
- **With none entered, a row is never behind on progress**

### The lateness columns

**Two columns**, right of the task name. A late row gets a red mark.

| Column | Red when |
| --- | --- |
| **Behind plan** | short of a checkpoint, or past the planned end with no actual end (overdue) |
| **Past due** | past the due date and not finished |

A row with no due date has an empty Past due; a row with no plan has an empty Behind plan.

**Finished late is "was late".** A row finished after its due date shows a quiet "was late" mark and nothing turns
red. Filtering for "behind" gives rows late now; pick "was late" when looking back.

Why two columns, and why finished lateness is not red: see [Design](design.en.md#lateness).

## Availability

How much room each person has, per month, over any range.

| Person | Available days | Elapsed | Committed | Free days | Overlapping | |
| --- | --- | --- | --- | --- | --- | --- |
| 佐藤 | 21d | 10d | 11d | **0d** | 5d | Overlapping: 8/24–8/28 |
| 山田 | 21d | 10d | 0d | **11d** | — | Free days: 8/17–8/31 |
| (unassigned) | — | — | 6d | — | — | |

| Column | Meaning |
| --- | --- |
| Available days | Days in the range less days off and leave. **Elapsed + Committed + Free days** |
| Elapsed | Days before today |
| Committed | Days from today on with at least one plan bar on them |
| Free days | Days from today on with nothing on them, with the dates |
| Overlapping | Days with two or more bars, with the dates |

- Three tasks on one day count as one day
- Not counted: finished tasks, summary rows, rows with only a due date, waiting days
- **Unassigned** work (no assignee) gets a row of its own
- **Split work that comes and goes into children.** One bar holds every day from end to end. Make the parent "the
  task" and each child one stretch of work, and only the children's days are held. If it is only stopped, record waiting instead

**Availability everywhere** (in the menu on the left) is the same table across every project you can open. Days off
are the shared calendar (weekends and the global holidays) and the person's own leave.

## Statistics and task history

**Statistics** (in the drawer) are the project's numbers. Summary rows are not counted.

| | |
| --- | --- |
| Tasks, average progress | How many, and their mean progress |
| Late, on time | Rows late now (behind plan, or past due and unfinished), and the rest |
| Work behind | Days of lateness, waiting excluded |
| Where the slippage went | Work behind + waiting = the slip. Waiting is split by reason |
| Status, person | Counts of each; per person, the status mix and average progress |

The same numbers over the API: [`/api/summary`](reference.en.md#api).

**Task history** shows who changed which value on which row, and when. Changes made with an API token show as
`API <what the token is for>`.

## Filters

One box per column, above the headings. Filled boxes are ANDed.

- Text columns match rows containing the text
- Dates and numbers compare: pick **at least / at most / equals / more than / less than** from the button beside the box (typing `<=100` or `>=3` works too)
- The two lateness columns are picked from **behind / on track**; Past due also has **was late**

## Settings

What the company has decided goes in the **global** settings (administrators); what belongs to one plan goes in the **project**.

| Global (administrator) | Project |
| --- | --- |
| Holidays and closures | This site's own days off / days it works when everyone else is off |
| People's colours | The people in this plan, and their order |
| People's leave and working days | Columns, own fields, display, notes, members |
| Default statuses (copied on creation) | This project's statuses |
| Password rule, app name, eras, language | API tokens (for this project) |
| Backup and restore, API tokens for all projects | |

- **Japanese public holidays fill themselves in**: last year, this year and next, at start and when the date
  moves on. Each year is filled once, so a holiday you delete stays deleted. A year you entered holidays for yourself is left alone
- A project's holidays are "global + days added here − days removed here"
- Statuses are copied from the global list when a project is created. Changing the global list does not change existing projects

### Project settings

| | |
| --- | --- |
| Display | Days left out of counts, business-year start, era years, quarters, day width |
| Columns | Shown, width, order. Built-in columns and your own fields together. The task name stays first |
| What a bar shows on hover | The items shown when pointing at a bar. Columns off the table can be chosen |
| Statuses | Name, colour, the progress it stands for (empty: typed in), order |
| Your own fields | Free, choice, free + choice, date, number. Renamable later. The type can change only while the column is empty |
| Bar colours | Plan, done, actual, summary, late, today, Saturday, Sunday, holiday, leave, waiting |
| Members | Owner, editor, viewer |
| API tokens | A token that opens this project only ([reference](reference.en.md#api)) |

**A new plan starts with eleven columns**: task, behind plan, past due, assignee, status, due date, planned start
and end, actual start and end, comment. Day counts, variances, progress, waits and checkpoints are hidden and come
back under Columns.

## Export and import

From "Import and export" in the drawer.

| | |
| --- | --- |
| Export to Excel | The screen's columns, with the chart drawn cell by cell to the right. For reading |
| Export as JSON (tasks + settings) | The whole project: settings, statuses, people, calendar, own fields, tasks |
| Export as JSON (tasks only) | The tasks alone |
| Import JSON (replaces everything) | Replaces the project with the file |

> [!WARNING]
> **Importing replaces every task with the file's. Rows not in the file are deleted.**
>
> - Rows are matched by `id`. A file row whose `id` this project knows updates that row. A row with no `id`, or
>   an `id` from elsewhere, is added as new
> - So **importing a file without ids into an existing project deletes every current row and adds the file's rows
>   as new ones** (their history starts over)
> - A file with an empty or missing `tasks` deletes all tasks
> - The screen asks first; writing back through the [API](reference.en.md#api) does the same thing without asking
>
> Export a copy as JSON before importing.

**Common uses**

- **Export, edit, import.** An exported file carries the ids, so importing it back updates the same rows. A
  tasks-only file is fine (settings stay as they are)
- **Start from a hand-written file.** Import it into an empty project. Leave the ids out
- **Move to another server.** Export tasks + settings, import into an empty project there

Everything other than tasks (settings, statuses, people, holidays, own fields) is **replaced only for the sections
the file has; sections it lacks are left as they are**. Leave is company-wide, so the file's leave is added and no
other leave is removed.

The format is in the [reference](reference.en.md#the-json-format).

## How it looks

**Theme** — automatic (follows the OS), light or dark — and **your own CSS**, both for your screen only. The plan's
colours (bars, statuses, people) belong to the project and do not change here.

Your CSS is loaded last, so it wins. 20,000 characters, `@import` is disabled, and outside images and fonts
(`url(https://…)`) are not loaded.

```css
.fg-bar { border-radius: 0 }        /* square bars */
.fg-row.fg-data { font-size: 13px } /* a tighter table */
```

## Language

Japanese by default. The order is: the person's own setting, then the installation's, then the browser's
`Accept-Language`.

What the users named — statuses, their own fields, people, projects — is data and is never translated.
