-- The statuses a new project starts with.
--
-- Copied at creation rather than followed for life: a shared list that every
-- project reads live would mean one edit in the settings changing the colours —
-- and, where progress is linked, the numbers — on somebody else's chart while
-- they were looking at it. What a team agreed on for one plan stays with it.
CREATE TABLE app_statuses (
  position INTEGER NOT NULL,
  name     TEXT PRIMARY KEY,
  color    TEXT NOT NULL DEFAULT '',
  -- What this state implies about progress. Empty means it says nothing.
  percent  INTEGER
);
