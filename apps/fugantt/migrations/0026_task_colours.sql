-- A colour a person chose, on the row they chose it for.
--
-- Rows already carry meaning nobody entered: red for late, grey for a summary,
-- a status pill in the project's own colours. What none of that can say is
-- "this one, the one we keep talking about" — which is the note people were
-- already leaving in the task name with ★ and 【】.
ALTER TABLE tasks ADD COLUMN color TEXT NOT NULL DEFAULT '';
ALTER TABLE tasks ADD COLUMN background TEXT NOT NULL DEFAULT '';
