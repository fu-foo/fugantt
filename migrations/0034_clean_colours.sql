-- Colours that are not colours.
--
-- A colour is written into a `style` attribute when a page is drawn. Typed in
-- the settings, it was always checked; arriving in an imported file, it was
-- stored as it came — and `#fff;position:fixed;inset:0` covered a whole page,
-- `url(…)` made every viewer fetch an address the file chose. The import now
-- keeps only `#rgb` / `#rrggbb`. This clears whatever got in before that.
--
-- Cleared rather than repaired: there is no telling what colour was meant.

UPDATE project_statuses SET color = '' WHERE NOT (color = '' OR color GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]' OR color GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]');
UPDATE project_field_options SET color = '' WHERE NOT (color = '' OR color GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]' OR color GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]');
UPDATE project_field_options SET background = '' WHERE NOT (background = '' OR background GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]' OR background GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]');
UPDATE assignees SET color = '' WHERE NOT (color = '' OR color GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]' OR color GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]');
UPDATE assignees SET background = '' WHERE NOT (background = '' OR background GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]' OR background GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]');
UPDATE app_statuses SET color = '' WHERE NOT (color = '' OR color GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]' OR color GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]');
UPDATE tasks SET color = '' WHERE NOT (color = '' OR color GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]' OR color GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]');
UPDATE tasks SET background = '' WHERE NOT (background = '' OR background GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]' OR background GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]');
