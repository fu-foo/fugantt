-- 読む人の言語。
--
-- 空なら「決めていない」＝全体の設定に従い、それも決まっていなければブラウザが
-- 送ってくる言語（＝その人の OS の設定）を見る。
ALTER TABLE users ADD COLUMN language TEXT NOT NULL DEFAULT '';
