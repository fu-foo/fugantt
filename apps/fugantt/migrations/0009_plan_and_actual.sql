-- The existing dates become the plan; the actual ones are new.
-- Renaming would have touched every query for no gain, so `start_date` and
-- `end_date` keep their names and mean 予定.
ALTER TABLE tasks ADD COLUMN actual_start TEXT;
ALTER TABLE tasks ADD COLUMN actual_end TEXT;
