-- 全体の設定。
--
-- Everything else is per project, but an app name and the list of Japanese eras
-- belong to the installation: a new era arrives for everybody at once.
CREATE TABLE app_settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
