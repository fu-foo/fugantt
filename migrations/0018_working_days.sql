-- 「休みだけど出社」。
--
-- 休みの逆。土日や祝日に出るのは日本の現場では普通にあり、そこを数えられないと
-- 日数が実態と合わない。休みと同じ表に、向きだけ足す。
ALTER TABLE assignee_leaves ADD COLUMN kind TEXT NOT NULL DEFAULT 'off';
