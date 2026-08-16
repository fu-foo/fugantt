-- How one person wants the screen to look.
--
-- Per user, not per installation: a plan is shared and its colours are the
-- project's, but the paper it is drawn on is nobody else's business. Somebody
-- working at night should not have to talk the team into it.
ALTER TABLE users ADD COLUMN theme TEXT NOT NULL DEFAULT '';
ALTER TABLE users ADD COLUMN custom_css TEXT NOT NULL DEFAULT '';
