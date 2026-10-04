-- 実施中 → 進行中.
--
-- 実施中 reads as a report on what is being done to the task; 進行中 is the
-- state the task is in, which is what a status column is for, and it is the
-- word people reach for out loud.
--
-- Renamed in the data rather than only in the default, because this name is
-- data nobody chose — it arrived with the app, and leaving it behind would mean
-- every plan made before today keeps a word the app no longer uses.
--
-- A project that already has a 進行中 of its own is left alone: two rows would
-- collide on the primary key, and a team that named one itself has said what it
-- wants to call things.
UPDATE app_statuses
   SET name = '進行中'
 WHERE name = '実施中'
   AND NOT EXISTS (SELECT 1 FROM app_statuses WHERE name = '進行中');

UPDATE project_statuses
   SET name = '進行中'
 WHERE name = '実施中'
   AND NOT EXISTS (
     SELECT 1
       FROM project_statuses AS other
      WHERE other.project_id = project_statuses.project_id
        AND other.name = '進行中'
   );

-- Only where the project's list no longer offers 実施中 — which, after the
-- statement above, means every project except the ones deliberately left alone.
-- A project with no list of its own follows the defaults, and those have moved.
UPDATE tasks
   SET status = '進行中'
 WHERE status = '実施中'
   AND NOT EXISTS (
     SELECT 1
       FROM project_statuses AS s
      WHERE s.project_id = tasks.project_id
        AND s.name = '実施中'
   );
