-- The library's videos as last scanned. A row stands while its file keeps the same size and modified time,
-- so a scan probes only new and changed files.
CREATE TABLE source (
  path TEXT PRIMARY KEY, -- relative to the library folder, '/' between parts, as in decision.source
  size INTEGER NOT NULL,
  modified_ms INTEGER NOT NULL,
  duration_s REAL NOT NULL,
  height INTEGER NOT NULL
) STRICT;
