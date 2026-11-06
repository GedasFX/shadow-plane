use crate::bin;
use crate::ffmpeg::Probe;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::UNIX_EPOCH;

// Over SMB, 12 probes took 72 ms each one at a time and 158 ms in all at once.
const PROBES_AT_ONCE: usize = 8;

#[derive(Debug, PartialEq, Serialize)]
pub struct Scan {
    pub sources: usize,
    pub unreadable: usize,
}

struct File {
    path: PathBuf,
    key: String,
    size: i64,
    modified_ms: i64,
    binned: bool,
}

// Brings the source index in line with the library folder: new and changed files are probed, gone ones dropped.
pub fn scan<F>(db: &Mutex<Connection>, root: &Path, probe: F) -> Result<Scan, Box<dyn Error>>
where
    F: Fn(&Path) -> io::Result<Option<Probe>> + Sync,
{
    let files = mp4s(root)?;
    let indexed = indexed(&db.lock().unwrap())?;

    let on_disk: HashSet<&str> = files.iter().map(|f| f.key.as_str()).collect();
    let todo: Vec<&File> = files
        .iter()
        .filter(|f| indexed.get(&f.key) != Some(&(f.size, f.modified_ms, f.binned)))
        .collect();

    let mut conn = db.lock().unwrap();
    let tx = conn.transaction()?;
    for key in indexed.keys().filter(|key| !on_disk.contains(key.as_str())) {
        tx.execute("DELETE FROM source WHERE path = ?1", [key])?;
    }
    tx.commit()?;
    drop(conn);

    let unreadable = probe_all(db, &todo, &probe)?;
    Ok(Scan {
        sources: files.len() - unreadable,
        unreadable,
    })
}

// Folder by folder, A to Z, as the Library lists them. _Bin's files come under the key they had
// before they moved; a key found in place too keeps the one in place.
fn mp4s(root: &Path) -> io::Result<Vec<File>> {
    let mut files = Vec::new();
    walk(root, root, false, &mut files)?;

    let bin = root.join(bin::FOLDER);
    if bin.is_dir() {
        let mut binned = Vec::new();
        walk(&bin, &bin, true, &mut binned)?;
        let in_place: HashSet<_> = files.iter().map(|f| f.key.clone()).collect();
        files.extend(binned.into_iter().filter(|f| !in_place.contains(&f.key)));
    }

    files.sort_by_cached_key(|f| f.key.to_lowercase());
    Ok(files)
}

// Every .mp4 below dir; folders starting with _ hold the app's own output, so they are skipped.
fn walk(root: &Path, dir: &Path, binned: bool, files: &mut Vec<File>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;

        if kind.is_dir() && !entry.file_name().to_string_lossy().starts_with('_') {
            walk(root, &path, binned, files)?;
        } else if kind.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
        {
            let meta = entry.metadata()?;
            files.push(File {
                key: key(root, &path),
                size: meta.len() as i64,
                modified_ms: modified_ms(&meta)?,
                binned,
                path,
            });
        }
    }

    Ok(())
}

pub fn modified_ms(meta: &fs::Metadata) -> io::Result<i64> {
    let modified = meta.modified()?.duration_since(UNIX_EPOCH);

    Ok(modified.unwrap_or_default().as_millis() as i64)
}

// The path below the library folder with / between parts, the same on every system.
fn key(root: &Path, path: &Path) -> String {
    let below = path.strip_prefix(root).expect("walked from the root");
    let parts: Vec<_> = below
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect();

    parts.join("/")
}

fn indexed(conn: &Connection) -> rusqlite::Result<HashMap<String, (i64, i64, bool)>> {
    let mut rows = conn.prepare("SELECT path, size, modified_ms, binned FROM source")?;
    let rows = rows.query_map([], |row| {
        Ok((row.get(0)?, (row.get(1)?, row.get(2)?, row.get(3)?)))
    })?;

    rows.collect()
}

// The threads take the files in order and store each as it lands, so the page lists them while the
// scan runs, a folder finished before the next starts; returns how many were unreadable.
fn probe_all<F>(db: &Mutex<Connection>, files: &[&File], probe: &F) -> io::Result<usize>
where
    F: Fn(&Path) -> io::Result<Option<Probe>> + Sync,
{
    let next = AtomicUsize::new(0);

    thread::scope(|s| {
        let threads: Vec<_> = (0..PROBES_AT_ONCE.min(files.len()))
            .map(|_| {
                s.spawn(|| {
                    let mut unreadable = 0;
                    while let Some(file) = files.get(next.fetch_add(1, Ordering::Relaxed)) {
                        let video = probe(&file.path)?;
                        unreadable += usize::from(video.is_none());
                        store(db, file, video.as_ref()).map_err(io::Error::other)?;
                    }
                    io::Result::Ok(unreadable)
                })
            })
            .collect();

        let mut unreadable = 0;
        for thread in threads {
            unreadable += thread.join().expect("a probe panicked")?;
        }

        Ok(unreadable)
    })
}

// A changed file keeps its old row until its new probe lands, so it never drops out mid-scan.
fn store(db: &Mutex<Connection>, file: &File, video: Option<&Probe>) -> rusqlite::Result<()> {
    let conn = db.lock().unwrap();
    let Some(video) = video else {
        conn.execute("DELETE FROM source WHERE path = ?1", [&file.key])?;
        return Ok(());
    };
    conn.execute(
        "INSERT OR REPLACE INTO source (path, size, modified_ms, duration_s, width, height, fps, binned)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            &file.key,
            file.size,
            file.modified_ms,
            video.duration_s,
            video.width,
            video.height,
            video.fps,
            file.binned,
        ),
    )?;
    Ok(())
}

// The game folder a video sits in; "Misc" for one outside any folder, as the legacy app had it.
pub fn game(key: &str) -> String {
    key.split_once('/')
        .map_or("Misc", |(game, _)| game)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::fs;
    use std::time::{Duration, SystemTime};
    use tempfile::TempDir;

    fn library(files: &[&str]) -> TempDir {
        let root = tempfile::tempdir().unwrap();
        for file in files {
            write(&root, file, b"x");
        }
        root
    }

    fn write(root: &TempDir, file: &str, bytes: &[u8]) {
        let path = root.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    // Every file reads as a one-minute 1440p video, except those named broken.mp4.
    fn probe(file: &Path) -> io::Result<Option<Probe>> {
        let video = Probe {
            duration_s: 60.0,
            width: 2560,
            height: 1440,
            fps: 60.0,
        };
        Ok((!file.ends_with("broken.mp4")).then_some(video))
    }

    fn index(db: &Mutex<Connection>) -> Vec<(String, f64, i64)> {
        let conn = db.lock().unwrap();
        let mut rows = conn
            .prepare("SELECT path, duration_s, height FROM source ORDER BY path")
            .unwrap();
        let rows = rows
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap();
        rows.map(Result::unwrap).collect()
    }

    fn paths(db: &Mutex<Connection>) -> Vec<String> {
        index(db).into_iter().map(|(path, _, _)| path).collect()
    }

    #[test]
    fn the_scan_indexes_each_readable_mp4_outside_underscore_folders() {
        let root = library(&[
            "PEAK/a.mp4",
            "PEAK/b.MP4",
            "PEAK/notes.txt",
            "PEAK/broken.mp4",
            "_Processed/PEAK/a.mp4",
            "Minecraft/2024/c.mp4",
            "d.mp4",
        ]);
        let db = Mutex::new(db::memory());

        let found = scan(&db, root.path(), probe).unwrap();

        assert_eq!(
            found,
            Scan {
                sources: 4,
                unreadable: 1
            }
        );
        assert_eq!(
            index(&db),
            [
                ("Minecraft/2024/c.mp4".into(), 60.0, 1440),
                ("PEAK/a.mp4".into(), 60.0, 1440),
                ("PEAK/b.MP4".into(), 60.0, 1440),
                ("d.mp4".into(), 60.0, 1440),
            ]
        );
    }

    #[test]
    fn the_scan_indexes_bin_under_the_key_before_the_move_and_flags_it() {
        let root = library(&[
            "PEAK/a.mp4",
            "_Bin/PEAK/b.mp4",
            "_Bin/PEAK/a.mp4",
            "_Bin/_Old/c.mp4",
        ]);
        let db = Mutex::new(db::memory());

        scan(&db, root.path(), probe).unwrap();

        let conn = db.lock().unwrap();
        let mut rows = conn
            .prepare("SELECT path, binned FROM source ORDER BY path")
            .unwrap();
        let rows: Vec<(String, bool)> = rows
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            rows,
            [("PEAK/a.mp4".into(), false), ("PEAK/b.mp4".into(), true)]
        );
    }

    #[test]
    fn a_video_moved_to_bin_is_probed_again_and_flagged() {
        let root = library(&["PEAK/a.mp4"]);
        let db = Mutex::new(db::memory());
        scan(&db, root.path(), probe).unwrap();
        fs::create_dir_all(root.path().join("_Bin/PEAK")).unwrap();
        fs::rename(
            root.path().join("PEAK/a.mp4"),
            root.path().join("_Bin/PEAK/a.mp4"),
        )
        .unwrap();

        scan(&db, root.path(), probe).unwrap();

        let binned: bool = db
            .lock()
            .unwrap()
            .query_row("SELECT binned FROM source", [], |row| row.get(0))
            .unwrap();
        assert!(binned);
    }

    #[test]
    fn the_files_come_folder_by_folder_a_to_z() {
        let root = library(&[
            "PEAK/b.mp4",
            "make way/z.mp4",
            "PEAK/a.mp4",
            "Minecraft/d.mp4",
            "Minecraft/2024/c.mp4",
            "e.mp4",
        ]);

        let keys: Vec<_> = mp4s(root.path())
            .unwrap()
            .into_iter()
            .map(|f| f.key)
            .collect();

        assert_eq!(
            keys,
            [
                "e.mp4",
                "make way/z.mp4",
                "Minecraft/2024/c.mp4",
                "Minecraft/d.mp4",
                "PEAK/a.mp4",
                "PEAK/b.mp4"
            ]
        );
    }

    #[test]
    fn the_scan_keeps_each_videos_width_and_frame_rate() {
        let root = library(&["PEAK/a.mp4"]);
        let db = Mutex::new(db::memory());

        scan(&db, root.path(), probe).unwrap();

        let stored: (i64, f64) = db
            .lock()
            .unwrap()
            .query_row("SELECT width, fps FROM source", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(stored, (2560, 60.0));
    }

    #[test]
    fn a_rescan_probes_only_new_and_changed_files_and_drops_gone_ones() {
        let root = library(&[
            "PEAK/same.mp4",
            "PEAK/grown.mp4",
            "PEAK/touched.mp4",
            "PEAK/gone.mp4",
        ]);
        let db = Mutex::new(db::memory());
        scan(&db, root.path(), probe).unwrap();

        // Change one file's size, another's modified time only.
        write(&root, "PEAK/grown.mp4", b"xy");
        let touched = fs::File::options()
            .write(true)
            .open(root.path().join("PEAK/touched.mp4"))
            .unwrap();
        touched
            .set_modified(SystemTime::now() + Duration::from_secs(3600))
            .unwrap();
        fs::remove_file(root.path().join("PEAK/gone.mp4")).unwrap();
        write(&root, "PEAK/new.mp4", b"x");

        let probed = Mutex::new(Vec::new());
        let counting = |file: &Path| {
            probed
                .lock()
                .unwrap()
                .push(file.file_name().unwrap().to_owned());
            probe(file)
        };
        scan(&db, root.path(), counting).unwrap();

        let mut probed = probed.into_inner().unwrap();
        probed.sort();
        assert_eq!(probed, ["grown.mp4", "new.mp4", "touched.mp4"]);
        assert_eq!(
            paths(&db),
            [
                "PEAK/grown.mp4",
                "PEAK/new.mp4",
                "PEAK/same.mp4",
                "PEAK/touched.mp4"
            ]
        );
    }

    #[test]
    fn a_scan_that_stops_midway_keeps_what_it_probed() {
        let root = library(&["PEAK/a.mp4", "PEAK/b.mp4", "PEAK/c.mp4"]);
        let db = Mutex::new(db::memory());

        let crashing_on_b = |file: &Path| {
            if file.ends_with("b.mp4") {
                return Err(io::Error::other("FFmpeg crashed"));
            }
            probe(file)
        };
        scan(&db, root.path(), crashing_on_b).unwrap_err();

        assert_eq!(paths(&db), ["PEAK/a.mp4", "PEAK/c.mp4"]);
    }

    #[test]
    fn a_changed_file_that_no_longer_reads_drops_out() {
        let root = library(&["PEAK/a.mp4"]);
        let db = Mutex::new(db::memory());
        scan(&db, root.path(), probe).unwrap();
        write(&root, "PEAK/a.mp4", b"xy");

        let unreadable = |_: &Path| Ok(None);
        scan(&db, root.path(), unreadable).unwrap();

        assert!(paths(&db).is_empty());
    }

    #[test]
    fn a_scan_that_cannot_probe_fails_and_keeps_the_index() {
        let root = library(&["PEAK/a.mp4"]);
        let db = Mutex::new(db::memory());
        scan(&db, root.path(), probe).unwrap();
        write(&root, "PEAK/b.mp4", b"x");

        let missing = |_: &Path| {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "FFmpeg isn't installed or isn't on PATH",
            ))
        };
        let failed = scan(&db, root.path(), missing).unwrap_err();

        assert_eq!(
            failed.to_string(),
            "FFmpeg isn't installed or isn't on PATH"
        );
        assert_eq!(paths(&db), ["PEAK/a.mp4"]);
    }
}
