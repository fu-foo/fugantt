-- Registration is by invitation. The first account to exist becomes the
-- administrator, which is how the very first person gets in.
ALTER TABLE users ADD COLUMN is_admin INTEGER NOT NULL DEFAULT 0;

CREATE TABLE invites (
    token      TEXT PRIMARY KEY,
    -- When set, only this address may use the invite.
    email      TEXT NOT NULL DEFAULT '',
    note       TEXT NOT NULL DEFAULT '',
    created_by TEXT REFERENCES users (id) ON DELETE SET NULL,
    created_at INTEGER NOT NULL,
    used_at    INTEGER,
    used_by    TEXT REFERENCES users (id) ON DELETE SET NULL
);

CREATE INDEX invites_unused ON invites (used_at);
