-- Holidays are per project: teams in different countries keep different
-- calendars, and a plan is the thing that has a calendar.
CREATE TABLE project_holidays (
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- 'YYYY-MM-DD'
    date       TEXT NOT NULL,
    name       TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (project_id, date)
);
