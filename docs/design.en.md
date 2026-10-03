# Design

[README](../README.en.md) · [Guide](guide.en.md) · [Reference](reference.en.md) · **Design** · [日本語](design.md)

Why it behaves the way it does. To use it, the [guide](guide.en.md) is enough.

## Derived values are not stored

Variances, day counts and lateness are computed on every read, and left out of the JSON. Nothing written back can
contradict itself.

## Due dates and plans answer different questions

They are not competing end dates. Which number is measured against which was settled once:

| Number | Measured against |
| --- | --- |
| Will it meet the due date | the due date only |
| Is it behind the plan | checkpoints and the planned end |
| Days in capacity and Availability | the plan bar only. A row with only a due date is 0 days |

A task that just has to be done by a date gets no span, because a span would book the person's days.

A summary row's due date is the latest of its children, but its lateness is drawn up from them. Recomputed from the
parent's date, a child that missed 10/5 would stay hidden until the parent's 11/30 came round.

## Progress is not calculated

Progress derived from the calendar advances as the days do, and then nothing is ever behind. So progress is
entered — from the status, or typed.

## Checkpoints

**Nothing between the points is judged.** 30% on 8/20 and 100% on 8/28 means those two days. Drawing a straight
line between them would be inventing a plan nobody made.

**The band ends at 50% of the bar, not at the date.** Grab that bar and it sets a percentage, so its axis is read as
one — two things that are compared have to share a ruler.

**Enter nothing and nothing is claimed.** Lateness is measured against a plan somebody wrote, not guessed from the
dates. Overdue (past the planned end with no actual end) is a fact of the dates, so it turns red with or without checkpoints.

## Lateness

**Late is a column, not a colour.** "Show me only the late ones" is a question a colour cannot answer.

**Two columns, because they use different rulers.** Folded into one, you could ask for "what will miss its due
date" or for "what is behind the plan", not both. A row that promised nothing has broken nothing.

**Finished late is not red.** It stays on record, but nothing can be done about it now. Red is kept for the rows
you can still act on; the statistics page's "late" and `/api/summary`'s `delayed` count those rows only.

**Waiting is kept out of lateness.** A stopped day is not work running late, and it is a day the person can spend on
something else. The statistics split work from waiting because a plan late on another team and a plan late because
the work took longer need different conversations.

## Availability

- **Counted from today.** Half a month gone with "twelve days free" in it is a lie by arithmetic. Days gone are their
  own column, and available = elapsed + committed + free, so the row adds up
- **A day is a day.** Three tasks on one Tuesday counted three times produces the 300% loads that make a report
  unreadable and then unread. How deep the stacking goes is its own column
- **The dates are printed.** "Nine days free" is half an answer
- **Unassigned work is shown.** Work nobody holds is not a plan
- **Availability everywhere exists** because somebody on three plans looks three times as free on each project's
  page, and adding that up is not a person's job. One project's own calendar does not speak for the others, so it is not used there
- **No effort percentages.** A finer unit needs a number on every task that nobody would keep up to date

## Editing

- **Keyboard first.** Whatever people keep their plans in now, they type into it without reaching for the mouse.
  Anything slower is abandoned within a week
- **Enter moves right in menus and dates**, because those are filled in along the row
- **Dates never roll forward.** An actual start is usually in the past; rolling a day number into next month would make "yesterday" untypeable
- **Undo never undoes somebody else's work.** That is why it stops at a cell someone else touched. Deleting cannot
  be undone because putting a row back means putting its subtree back with the ids it had — a different piece of work
- **Row colours** exist because people were writing ★ into task names, where it sorts, exports and stays for ever
- **Summary bars are hidden by default.** Open children already show that span
- **Hover items are chosen.** A column worth a glance does not have to sit on the screen all day
- **A new plan starts with eleven columns.** Day counts, variances, waits and checkpoints are on the chart already, or entered through a dialog

## Global and project

The company's facts (holidays, people's colours, leave) are global; the plan's facts (columns, statuses, the site's
own days off) belong to the project.

- **Statuses are copied.** Editing the global list must not change what a running plan means
- **Leave is one list for the company**, editable by an editor of any plan the person appears on. Assignee names are
  free text, so it is not fenced by permissions but kept traceable
- **Plan colours belong to the project.** Two people reading the same plan in different colours are reading two different plans

## The Japanese calendar

- **Holidays are computed, not pasted** — substitute holidays and the citizens' holiday between two others
  included. Last year, this year and next are filled in by themselves. Each year is filled once, so a deleted holiday stays deleted
- **The business year sits above the months**, starting in the month you say (April by default)
- **Saturday blue, Sunday red.** Same-grey weekends are misread
- **The weekday is printed under every date.** Counting to a Friday off a month grid is not a plan
- **Era years are data.** A new era is one line in the settings, not a new build

None of it is fixed to one country: holidays, the business year, the days skipped and the language are all settings.

## Distribution

- **Port 1861** — the year Henry Gantt was born
- **The Docker image is amd64 only.** An arm64 image would be built under QEMU in CI, and a Rust release build there
  takes long enough to make cutting a release something nobody does
- **No Windows code signing.** The first run's warning is "More info" → "Run anyway"
- **Authentication is left to a reverse proxy.** The hard parts of LDAP, SAML and OIDC stay where they are already solved
- **`--make-admin` grants nothing new.** Whoever can run it can already read the database file
