-- The index is a cache: dropping it only costs a rescan. The new columns come from ffprobe, so
-- every file is probed again.
DROP TABLE source;

CREATE TABLE source (
  path TEXT PRIMARY KEY, -- relative to the library folder, '/' between parts, as in decision.source
  size INTEGER NOT NULL,
  modified_ms INTEGER NOT NULL,
  duration_s REAL NOT NULL,
  width INTEGER NOT NULL,
  height INTEGER NOT NULL,
  fps REAL NOT NULL
) STRICT;

-- The two labels a clip can carry; they end its file name as emoji.
ALTER TABLE clip ADD COLUMN great INTEGER NOT NULL DEFAULT 0 CHECK (great IN (0, 1));
ALTER TABLE clip ADD COLUMN best INTEGER NOT NULL DEFAULT 0 CHECK (best IN (0, 1));
