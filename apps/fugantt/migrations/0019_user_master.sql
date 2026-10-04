-- ユーザーマスター。
--
-- 招待リンクを配って本人に登録してもらう仕組みは、社内で使うには回りくどい。
-- 管理者が「名前・ユーザー名・パスワード・ベース権限」を直接作れるようにする。
--
-- ベース権限は、プロジェクトに明示のメンバー指定が無いときに使われる既定値。
-- `none` は「指定されたプロジェクトしか見えない人」で、社外の人や、1件だけ
-- 参照させたい相手のための形。
CREATE TABLE users_new (
  id            TEXT PRIMARY KEY,
  email         TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  created_at    INTEGER NOT NULL,
  display_name  TEXT NOT NULL DEFAULT '',
  -- admin / editor / viewer / none
  base_role     TEXT NOT NULL DEFAULT 'none'
);

-- 既存のユーザーは今までどおり: メンバーに入っているプロジェクトだけが見える。
-- 管理者だけが admin になる。
INSERT INTO users_new (id, email, password_hash, created_at, display_name, base_role)
  SELECT id, email, password_hash, created_at, display_name,
         CASE WHEN is_admin = 1 THEN 'admin' ELSE 'none' END
    FROM users;

DROP TABLE users;
ALTER TABLE users_new RENAME TO users;

DROP TABLE IF EXISTS invites;
