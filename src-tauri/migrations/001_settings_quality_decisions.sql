CREATE TABLE setting (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
) STRICT;

-- What makes a clip expensive to encode: its game, the output's height tier and the source's bits per
-- pixel. The encoder is part of the key because each one has its own quality scale.
CREATE TABLE quality_profile (
  id INTEGER PRIMARY KEY,
  encoder TEXT NOT NULL,
  game TEXT NOT NULL,
  tier INTEGER NOT NULL,
  bpp_band TEXT NOT NULL CHECK (bpp_band IN ('lean', 'mid', 'rich', 'extreme')),
  UNIQUE (encoder, game, tier, bpp_band)
) STRICT;

CREATE TABLE quality_measurement (
  id INTEGER PRIMARY KEY,
  profile_id INTEGER NOT NULL REFERENCES quality_profile (id),
  setting REAL NOT NULL, -- the encoder's quality setting: CRF for SVT-AV1, CQ for NVENC
  mbps REAL NOT NULL,
  vmaf_mean REAL NOT NULL,
  vmaf_p1 REAL NOT NULL,
  measured_at TEXT NOT NULL,
  note TEXT
) STRICT;

-- The quality band an encode must land in; game '' is the default for every game.
CREATE TABLE quality_target (
  game TEXT PRIMARY KEY,
  band_lo REAL NOT NULL,
  band_hi REAL NOT NULL,
  p1_floor REAL NOT NULL,
  edited_at TEXT NOT NULL
) STRICT;

-- What the user decided for a recording, keyed by its path in the library folder. A null max_height
-- keeps the source's resolution; a null band means the game's quality target.
CREATE TABLE decision (
  source TEXT PRIMARY KEY,
  action TEXT NOT NULL CHECK (action IN ('queue', 'skip', 'reject')),
  title TEXT,
  note TEXT,
  max_height INTEGER,
  band_lo REAL,
  band_hi REAL,
  p1_floor REAL,
  decided_at TEXT NOT NULL,
  CHECK ((band_lo IS NULL) = (band_hi IS NULL) AND (band_lo IS NULL) = (p1_floor IS NULL))
) STRICT;

CREATE TABLE clip (
  id INTEGER PRIMARY KEY,
  decision_source TEXT NOT NULL REFERENCES decision (source) ON DELETE CASCADE,
  position INTEGER NOT NULL,
  title TEXT,
  game TEXT,
  blend TEXT NOT NULL,
  UNIQUE (decision_source, position)
) STRICT;

-- Each segment names its own recording, so one clip can join several.
CREATE TABLE segment (
  clip_id INTEGER NOT NULL REFERENCES clip (id) ON DELETE CASCADE,
  position INTEGER NOT NULL,
  source TEXT NOT NULL,
  start_s REAL NOT NULL,
  end_s REAL NOT NULL CHECK (end_s > start_s),
  PRIMARY KEY (clip_id, position)
) STRICT;
