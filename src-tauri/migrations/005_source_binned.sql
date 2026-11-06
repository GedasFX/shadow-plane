-- A video moved to _Bin stays in the index under the key it had before. The index is a cache, so a
-- row's flag is re-read from the folder the file is found in.
ALTER TABLE source ADD COLUMN binned INTEGER NOT NULL DEFAULT 0 CHECK (binned IN (0, 1));
