-- One encode job per saved clip; the worker takes pending jobs in position order. A clip's removal
-- takes its job with it.
CREATE TABLE job (
  id INTEGER PRIMARY KEY,
  clip_id INTEGER NOT NULL UNIQUE REFERENCES clip (id) ON DELETE CASCADE,
  position INTEGER NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'done', 'failed', 'cancelled')),
  output TEXT, -- the finished file, relative to the library folder, '/' between parts
  error TEXT,
  finished_at TEXT
) STRICT;
