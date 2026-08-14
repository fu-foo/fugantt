-- Status is kept separate from progress on purpose: "実施中 0%" and
-- "完了 100%" are both things a plan legitimately says, and deriving one from
-- the other would overwrite what someone typed.
ALTER TABLE tasks ADD COLUMN status TEXT NOT NULL DEFAULT '未着手'
    CHECK (status IN ('未着手', '実施中', '完了', '保留'));

-- Free text rather than a user reference: the person doing the work often has
-- no account here.
ALTER TABLE tasks ADD COLUMN assignee TEXT NOT NULL DEFAULT '';

ALTER TABLE tasks ADD COLUMN note TEXT NOT NULL DEFAULT '';
