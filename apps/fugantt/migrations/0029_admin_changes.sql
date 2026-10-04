-- Who did what to the accounts.
--
-- Task changes hang from a project, and these do not: adding a person, taking
-- one away, and moving somebody's role are about the installation. Kept apart
-- rather than bent into `changes`, whose project_id is a real reference to a
-- real project.
CREATE TABLE admin_changes (
    id     INTEGER PRIMARY KEY AUTOINCREMENT,
    action TEXT NOT NULL,
    -- The account it was about, as text: a deleted person still has to be
    -- readable in the record of their own removal.
    about  TEXT NOT NULL DEFAULT '',
    before TEXT NOT NULL DEFAULT '',
    after  TEXT NOT NULL DEFAULT '',
    actor  TEXT NOT NULL DEFAULT '',
    at     INTEGER NOT NULL
);

CREATE INDEX admin_changes_at ON admin_changes (id DESC);
