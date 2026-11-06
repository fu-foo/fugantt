-- The order the project list is read in.
--
-- The list was newest-touched first, so a plan moved every time somebody typed
-- into it, and the one being looked for was never where it was the last time.
-- One order for everyone, the way a team names its plans to each other; an
-- administrator moves them.
--
-- What is here now starts in name order, which is what the API already listed
-- them in. A project made later goes to the end.
ALTER TABLE projects ADD COLUMN position INTEGER NOT NULL DEFAULT 0;

UPDATE projects
   SET position = (SELECT COUNT(*) FROM projects AS earlier WHERE earlier.name < projects.name);
