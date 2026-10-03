# Reference

[README](../README.en.md) · [Guide](guide.en.md) · **Reference** · [Design](design.en.md) · [日本語](reference.md)

- [Keys](#keys)
- [Settings](#settings)
- [Commands](#commands)
- [The JSON format](#the-json-format)
- [API](#api)
- [Roles and sign-in](#roles-and-sign-in)
- [Defences](#defences)
- [What 1.0 promises](#what-10-promises)

## Keys

| Key | |
| --- | --- |
| Arrows | Move between cells |
| Tab / Shift+Tab | Next / previous cell, wrapping to the next row. Computed columns (day counts, variances) are skipped |
| Enter / F2 / double-click | Open the cell |
| Typing | Opens the cell with that character (Japanese input included) |
| Escape | Cancel the edit |
| Enter (editing) | Confirm and move down. Menus and dates move right |
| Tab (editing) | Confirm and move right |
| Delete / Backspace | Empty the cell |
| ⌘Enter / Ctrl+Enter | Add a row below, ready to type its name |
| ⌘Delete / Ctrl+Delete | Delete the row (children too) |
| Home / End | Start / end of the row. With ⌘ / Ctrl, top / bottom of the table |
| ⌥→ ← / Alt+→ ← | Make a child / bring back a level |
| ⌥↑ ↓ / Alt+↑ ↓ | Reorder among siblings |
| ⌘← → / Ctrl+← → | Fold / unfold |
| ⌘Z / Ctrl+Z | Undo |
| ⌘Y / ⌘⇧Z / Ctrl+Y | Redo |

`⌘` and `Ctrl` both work on either platform; only the label on the screen changes. Right-click gives the same moves as a menu.

## Settings

Environment variables, or the same names in `fugantt.ini`. The environment wins.

`fugantt.ini` is looked for beside the executable, in the working directory, and in the platform's place for user
data (`%LOCALAPPDATA%\fugantt`, `~/Library/Application Support/fugantt`, `~/.local/share/fugantt`).
`fugantt.conf` and `.env` are read too.

| Name | Default | |
| --- | --- | --- |
| `HOST` | `127.0.0.1` | `0.0.0.0` to serve the LAN |
| `PORT` | `1861` | |
| `FUGANTT_DB` | see below | The SQLite file. Migrated on start |
| `FUGANTT_OPEN` | automatic | `window` (application window) / `tab` / `0` (don't open) |
| `FUGANTT_ALLOW_HTTP` | — | `1` drops `Secure` from the cookie so sign-in works over plain HTTP |
| `FUGANTT_NO_AUTH` | — | [Running without sign-in](#running-without-sign-in) |

**Without `FUGANTT_OPEN`**: the page opens only when started on `127.0.0.1`, `localhost` or `::1` — an Edge
application window (no address bar) on Windows, a tab in the default browser elsewhere. On `0.0.0.0` nothing opens.

**The database**: `FUGANTT_DB`, then a `fugantt.db` already in the working directory, then the platform's place for
user data. The absolute path is printed at startup. If you already keep a `fugantt.db` somewhere, replacing the
executable does not move it.

## Commands

| | |
| --- | --- |
| `fugantt` | Run in the foreground. Close it or press Ctrl+C to stop |
| `fugantt start` | Start in the background and return (the page still opens) |
| `fugantt status` | Whether it is running, and where: URL, data, log |
| `fugantt log` | The end of its log |
| `fugantt stop` | Stop it |
| `fugantt --config` | Where each value came from |
| `fugantt --help` | What can be set |
| `fugantt --make-admin <user name>` | Make that account an administrator |

In the background, the log and process number are kept in the data's default place, named by port (`fugantt-1861.log`).

## The JSON format

```json
{
  "version": 1,
  "name": "Release plan",
  "tasks": [
    { "id": "c0ffee…", "name": "Requirements", "depth": 0,
      "start": "2026-08-03", "end": "2026-08-14", "due": "2026-08-20",
      "actual_start": "2026-08-03", "actual_end": "2026-08-18",
      "progress": 100, "status": "完了", "assignee": "山田",
      "waits": ["2026-08-17/2026-08-21"], "targets": ["2026-08-10/50"],
      "fields": { "Product": "A" } }
  ]
}
```

| | |
| --- | --- |
| `id` | Identifies the row. An `id` this project knows updates that row; none, or an unknown one, adds a new row. **Rows not in the file are deleted** (see below) |
| `depth` | From 0. Deeper than the row above means a child of it |
| `start` `end` `due` | Planned start, planned end, due date. `YYYY-MM-DD` |
| `waits` | `"from/to"`. Omit the end and it is still waiting |
| `targets` | `"date/percent"`. Checkpoints |
| references | Statuses, people and fields are named, never referenced by id |
| summary rows | Their dates and progress are not written out: they come from the children |

> [!WARNING]
> Importing **replaces all tasks**. A file without ids imported into an existing project deletes every current row
> and adds the file's rows as new ones. An empty `tasks` deletes everything.
> See the [guide](guide.en.md#export-and-import).

There are also `settings`, `statuses`, `assignees`, `holidays`, `leaves` and `fields` sections (written by "tasks +
settings"). **Only the sections in the file are replaced; the rest are left as they are.** `leaves` only adds: no
other leave is removed.

No derived value is in the file — no day counts, no variance, no lateness. To read those,
`GET /api/projects/{id}/grid` returns the table already computed.

> Read from the grid, write to the document.

## API

### API tokens

| | Issued by | Opens |
| --- | --- | --- |
| Project settings → API tokens | the owner | that project only. "Read" or "read and write" |
| Global settings → API tokens | an administrator | every project |

Shown once when issued (only a hash is kept).

```sh
curl -H "Authorization: Bearer fug_…" \
  https://example.com/api/projects/release-plan/document          # read

curl -X POST -H "Authorization: Bearer fug_…" \
  -H "Content-Type: application/json" --data @plan.json \
  https://example.com/api/projects/release-plan/document          # write back (replaces all; no confirmation)
```

```sh
curl -H "Authorization: Bearer fug_…" https://example.com/api/projects   # the plans
curl -H "Authorization: Bearer fug_…" https://example.com/api/summary    # numbers per project
```

| | |
| --- | --- |
| `GET /api/projects/{id}/document` | Export as JSON. `?settings=0` for tasks only |
| `POST /api/projects/{id}/document` | Import JSON ([replaces all](guide.en.md#export-and-import)) |
| `GET /api/projects/{id}/grid` | The table with day counts, variances and lateness computed (not covered by the 1.0 promise) |
| `GET /api/projects` | The plans |
| `GET /api/summary` | The statistics page's arithmetic, one row per project (`late_days + wait_days = slipped`) |

- A token for another project, a write with a read-only token, and a request with neither token nor cookie all get **403**
- A change made with a token is recorded in the task history as **`API <what the token is for>`**
- `/api/projects` and `/api/summary` also work with a signed-in session, returning only what that person may see
- `delayed` in `/api/summary` counts rows late now (behind plan, or past due and unfinished)

## Roles and sign-in

Whoever registers first is the administrator ([README](../README.en.md#the-first-account)). After that, only
administrators create accounts.

| Base role | |
| --- | --- |
| Administrator | Everything: global settings, users, backups |
| Editor | Can edit every project |
| Viewer | Can read every project |
| No access | Only the projects they are a member of |

To keep a plan to a few people, set the base role to "no access" and add them as project members (owner, editor, viewer).

- The password rule (minimum length, required kinds of character, refused words) is set globally. Eight characters by default
- Changing a password ends that person's other sessions
- An administrator sets the first password and hands it over, and sets it again if it is forgotten. No mail is sent
- **Accounts added, removed and changed**, and **leave and working days added or removed**, are recorded with who did it and from which plan (at the foot of the users page)
- Somebody who leaves can simply be deleted: assignees and task history keep names as text, so the record survives

**When no administrator can get in**, run `fugantt --make-admin yamada@example.com` on the server.

There is no LDAP, SAML or OIDC. Put a reverse proxy in front and authenticate there.

### Running without sign-in

`FUGANTT_NO_AUTH=yes-everyone-on-this-network-can-edit`.
**Everyone who can reach that URL can read and edit every project.** A banner stays on screen while it is on.

## Defences

- **Changes are accepted only from fugantt's own pages.** A change sent from another port on the same machine, or a
  sibling subdomain, gets a 403 — told apart by the browser's `Sec-Fetch-Site` (or `Origin` on older browsers).
  API tokens are not affected
- **A reverse proxy must pass `Host` or `X-Forwarded-Host` as the browser sent it**, or your own changes are refused too
- **Requests have a size limit**: 1 MB normally, 64 MB for a JSON plan, 1 GB for restoring a backup. Larger gets a 413
- **Sign-in failures: 8 per user name per 15 minutes.** Counting by address as well happens only on fly.io
- **Pages refuse to be framed and load no outside scripts, images or fonts** (`Content-Security-Policy`). Colours
  are `#rrggbb` only, imported files included
- **The cookie is `Secure` and `__Host-` by default**

## What 1.0 promises

From 1.0, these are the things that will not be broken:

- **The database.** An older file opens in a newer build, migrated on start
- **The API.** `/api/projects/{id}/document` (read and write), `/api/projects`, `/api/summary`
- **The settings.** The `fugantt.ini` format and the environment variable names
- **The commands.** `fugantt`, `fugantt start` / `stop` / `status` / `log`, `--config`, `--make-admin`
- **Port 1861**, as the default

**The endpoints the grid itself uses are not covered.** They change with it.

**Decided against** — which is also a promise, of a kind:

- **No Windows code signing.** "Windows protected your PC" → More info → Run anyway
- **The Docker image stays amd64.** On a Mac, use the binary — it is native
- **No cross-project statistics screen.** `/api/summary` has the numbers for every project
- **No LDAP, SAML or OIDC.** Put a reverse proxy in front
- **Deleting a row cannot be undone.** It asks first. Undo covers values, added rows and reordering
- **Opening a plan sends all of it.** 170ms at ten thousand rows, once

**Measured, not guaranteed** — here to save you the time:

- Committing one cell is a **14ms** round trip at a hundred rows and at ten thousand (release build)
- A plan of **1,000–3,000 rows** is the working range. Ten thousand still types at the same speed
- The browser tests drive a real Chrome (384 of them)
- One process, one SQLite file. No limit is set on how many people use it
