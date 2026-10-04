-- The last of the one-wait-per-task model.
--
-- A task used to have a single wait: a reason, who it was waiting on, and two
-- dates. It has had a list of them in `waits` since, and nothing has read these
-- four columns for a long time — the reads name their columns, so they cost
-- nothing either. They are dropped because a schema is also something people
-- read, and four columns that answer nothing are four questions.
ALTER TABLE tasks DROP COLUMN wait_reason;
ALTER TABLE tasks DROP COLUMN wait_target;
ALTER TABLE tasks DROP COLUMN wait_start;
ALTER TABLE tasks DROP COLUMN wait_until;
