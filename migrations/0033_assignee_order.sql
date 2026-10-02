-- The order this plan lists its people in.
--
-- Alphabetical was the only order there was, and a team is not read that way:
-- the lead first, or the people in the order the work passes through them.
-- The order is the plan's own — two projects can run the same people
-- differently — so it is kept per project.
--
-- Its own table rather than a column on `project_assignees`: that table holds
-- the names somebody added by hand, and the list being ordered also carries
-- members and whoever stands on a task. Putting those into it to give them a
-- position would make them stay on the list after they had left the plan.
-- A name with no row here comes after the ordered ones, alphabetically.
CREATE TABLE project_assignee_order (
  project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
  name       TEXT NOT NULL,
  position   INTEGER NOT NULL,
  PRIMARY KEY (project_id, name)
);
