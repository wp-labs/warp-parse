CREATE TABLE IF NOT EXISTS {table} (
  id      INTEGER PRIMARY KEY,
  name    TEXT NOT NULL,
  pinying TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_{table}_name ON {table}(name);
