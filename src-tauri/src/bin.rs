use crate::files;
use crate::media;
use rusqlite::{Connection, OptionalExtension};
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Mutex;

pub const FOLDER: &str = "_Bin";

// Moves the video at <library>/<key> to <library>/_Bin/<key> and flags its row; one already binned
// stays where it is. The database is locked only for the row, not through the move.
pub fn bin(db: &Mutex<Connection>, root: &Path, key: &str) -> io::Result<()> {
    let binned = binned(&db.lock().unwrap(), key).map_err(io::Error::other)?;
    if binned.ok_or_else(|| io::Error::other(format!("{key} is not in the library")))? {
        return Ok(());
    }

    let from = media::below(root, key).ok_or_else(|| io::Error::other("Not a library path"))?;
    let to = media::below(&root.join(FOLDER), key).expect("the same key passed above");
    fs::create_dir_all(to.parent().expect("a key names a file"))?;
    files::move_file(&from, &to)?;

    let conn = db.lock().unwrap();
    conn.execute("UPDATE source SET binned = 1 WHERE path = ?1", [key])
        .map_err(io::Error::other)?;

    Ok(())
}

fn binned(conn: &Connection, key: &str) -> rusqlite::Result<Option<bool>> {
    let row = "SELECT binned FROM source WHERE path = ?1";

    conn.query_row(row, [key], |row| row.get(0)).optional()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn library(files: &[&str]) -> (tempfile::TempDir, Mutex<Connection>) {
        let root = tempfile::tempdir().unwrap();
        let conn = db::memory();
        for file in files {
            let path = root.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, file).unwrap();
            conn.execute(
                "INSERT INTO source (path, size, modified_ms, duration_s, width, height, fps) VALUES (?1, 1, 1, 60, 2560, 1440, 60)",
                [file],
            )
            .unwrap();
        }
        (root, Mutex::new(conn))
    }

    fn flag(db: &Mutex<Connection>, key: &str) -> bool {
        binned(&db.lock().unwrap(), key).unwrap().unwrap()
    }

    #[test]
    fn binning_moves_the_file_below_bin_and_flags_its_row() {
        let (root, db) = library(&["PEAK/a.mp4"]);

        bin(&db, root.path(), "PEAK/a.mp4").unwrap();

        assert!(!root.path().join("PEAK/a.mp4").exists());
        assert_eq!(
            fs::read(root.path().join("_Bin/PEAK/a.mp4")).unwrap(),
            b"PEAK/a.mp4"
        );
        assert!(flag(&db, "PEAK/a.mp4"));
    }

    #[test]
    fn a_video_already_binned_is_not_moved_again() {
        let (root, db) = library(&["PEAK/a.mp4"]);
        bin(&db, root.path(), "PEAK/a.mp4").unwrap();

        bin(&db, root.path(), "PEAK/a.mp4").unwrap();

        assert!(root.path().join("_Bin/PEAK/a.mp4").exists());
    }

    #[test]
    fn a_taken_place_in_bin_fails_and_changes_nothing() {
        let (root, db) = library(&["PEAK/a.mp4", "_Bin/PEAK/a.mp4"]);

        let failed = bin(&db, root.path(), "PEAK/a.mp4").unwrap_err();

        assert_eq!(failed.kind(), io::ErrorKind::AlreadyExists);
        assert!(root.path().join("PEAK/a.mp4").exists());
        assert!(!flag(&db, "PEAK/a.mp4"));
    }

    #[test]
    fn a_key_outside_the_index_or_the_library_is_refused() {
        let (root, db) = library(&["PEAK/a.mp4"]);

        assert!(bin(&db, root.path(), "PEAK/other.mp4").is_err());
        assert!(bin(&db, root.path(), "../a.mp4").is_err());
        assert!(root.path().join("PEAK/a.mp4").exists());
    }
}
