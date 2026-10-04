-- アカウントの表示名。
--
-- The screen used to show an email address wherever a person appeared, which
-- is neither what anyone is called nor short enough to sit in a cell.
ALTER TABLE users ADD COLUMN display_name TEXT NOT NULL DEFAULT '';
