use crate::ffprobe::Probe;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;
use std::time::UNIX_EPOCH;

// Over SMB, 12 probes took 72 ms each one at a time and 158 ms in all at once.
const PROBES_AT_ONCE: usize = 8;

#[derive(Debug, PartialEq, Serialize)]
pub struct Scan {
    pub sources: usize,
    pub unreadable: usize,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Game {
    pub name: String,
    pub sources: Vec<Source>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Source {
    pub path: String,
    pub name: String,
    pub duration_s: f64,
    pub height: i64,
    pub decision: Option<String>,
}

struct File {
    path: PathBuf,
    key: String,
    size: i64,
    modified_ms: i64,
}

// Brings the source index in line with the library folder: new and changed files are probed, gone ones dropped.
pub fn scan<F>(db: &Mutex<Connection>, root: &Path, probe: F) -> Result<Scan, Box<dyn Error>>
where
    F: Fn(&Path) -> io::Result<Option<Probe>> + Sync,
{
    let mut files = Vec::new();
    walk(root, root, &mut files)?;
    let indexed = indexed(&db.lock().unwrap())?;

    let unchanged = |file: &&File| indexed.get(&file.key) == Some(&(file.size, file.modified_ms));
    let kept: HashSet<&str> = files
        .iter()
        .filter(unchanged)
        .map(|f| f.key.as_str())
        .collect();
    let todo: Vec<&File> = files
        .iter()
        .filter(|f| !kept.contains(f.key.as_str()))
        .collect();
    let probes = probe_all(&todo, &probe)?;

    // Write it all at once, and only once every probe has run.
    let mut conn = db.lock().unwrap();
    let tx = conn.transaction()?;
    for key in indexed.keys().filter(|key| !kept.contains(key.as_str())) {
        tx.execute("DELETE FROM source WHERE path = ?1", [key])?;
    }
    for (file, video) in todo.iter().zip(&probes) {
        let Some(video) = video else { continue };
        tx.execute(
            "INSERT INTO source (path, size, modified_ms, duration_s, width, height, fps)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                &file.key,
                file.size,
                file.modified_ms,
                video.duration_s,
                video.width,
                video.height,
                video.fps,
            ),
        )?;
    }
    tx.commit()?;

    let unreadable = probes.iter().filter(|video| video.is_none()).count();
    Ok(Scan {
        sources: files.len() - unreadable,
        unreadable,
    })
}

// Every .mp4 below dir; folders starting with _ hold the app's own output, so they are skipped.
fn walk(root: &Path, dir: &Path, files: &mut Vec<File>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;

        if kind.is_dir() && !entry.file_name().to_string_lossy().starts_with('_') {
            walk(root, &path, files)?;
        } else if kind.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
        {
            let meta = entry.metadata()?;
            let modified = meta
                .modified()?
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            files.push(File {
                key: key(root, &path),
                size: meta.len() as i64,
                modified_ms: modified.as_millis() as i64,
                path,
            });
        }
    }

    Ok(())
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

fn indexed(conn: &Connection) -> rusqlite::Result<HashMap<String, (i64, i64)>> {
    let mut rows = conn.prepare("SELECT path, size, modified_ms FROM source")?;
    let rows = rows.query_map([], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?))))?;

    rows.collect()
}

// Each thread probes its own share of the files; the results come back in the files' order.
fn probe_all<F>(files: &[&File], probe: &F) -> io::Result<Vec<Option<Probe>>>
where
    F: Fn(&Path) -> io::Result<Option<Probe>> + Sync,
{
    let share = files.len().div_ceil(PROBES_AT_ONCE).max(1);

    thread::scope(|s| {
        let threads: Vec<_> = files
            .chunks(share)
            .map(|part| {
                s.spawn(move || {
                    part.iter()
                        .map(|file| probe(&file.path))
                        .collect::<io::Result<Vec<_>>>()
                })
            })
            .collect();

        let mut probes = Vec::with_capacity(files.len());
        for thread in threads {
            probes.extend(thread.join().expect("a probe panicked")?);
        }

        Ok(probes)
    })
}

// The indexed sources by game folder, each with its decision; "" holds those outside any folder.
pub fn games(conn: &Connection) -> rusqlite::Result<Vec<Game>> {
    let mut rows = conn.prepare(
        "SELECT s.path, s.duration_s, s.height, d.action
         FROM source s LEFT JOIN decision d ON d.source = s.path
         ORDER BY s.path",
    )?;
    let rows = rows.query_map([], |row| {
        Ok(Source {
            path: row.get(0)?,
            name: String::new(),
            duration_s: row.get(1)?,
            height: row.get(2)?,
            decision: row.get(3)?,
        })
    })?;

    let mut games: BTreeMap<String, Vec<Source>> = BTreeMap::new();
    for source in rows {
        let mut source = source?;
        let (game, name) = source.path.split_once('/').unwrap_or(("", &source.path));
        let game = game.to_owned();
        source.name = name.to_owned();
        games.entry(game).or_default().push(source);
    }

    let games = games
        .into_iter()
        .map(|(name, sources)| Game { name, sources });
    Ok(games.collect())
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
    fn a_scan_that_cannot_probe_fails_and_keeps_the_index() {
        let root = library(&["PEAK/a.mp4"]);
        let db = Mutex::new(db::memory());
        scan(&db, root.path(), probe).unwrap();
        write(&root, "PEAK/b.mp4", b"x");

        let missing = |_: &Path| {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "FFmpeg isn't installed",
            ))
        };
        let failed = scan(&db, root.path(), missing).unwrap_err();

        assert_eq!(failed.to_string(), "FFmpeg isn't installed");
        assert_eq!(paths(&db), ["PEAK/a.mp4"]);
    }

    #[test]
    fn the_list_groups_sources_by_game_folder_with_their_decisions() {
        let conn = db::memory();
        conn.execute_batch(
            "INSERT INTO source VALUES ('PEAK/b.mp4', 1, 1, 90.5, 2560, 1440, 60);
             INSERT INTO source VALUES ('PEAK/a.mp4', 1, 1, 60, 1920, 1080, 60);
             INSERT INTO source VALUES ('Minecraft/2024/c.mp4', 1, 1, 30, 2560, 1440, 60);
             INSERT INTO source VALUES ('d.mp4', 1, 1, 10, 1280, 720, 60);
             INSERT INTO decision (source, action, decided_at) VALUES ('PEAK/b.mp4', 'skip', '2026-11-05');
             INSERT INTO decision (source, action, decided_at) VALUES ('Gone/e.mp4', 'queue', '2026-11-05');",
        )
        .unwrap();

        let source = |path: &str, name: &str, duration_s, height, decision: Option<&str>| Source {
            path: path.into(),
            name: name.into(),
            duration_s,
            height,
            decision: decision.map(String::from),
        };
        let game = |name: &str, sources| Game {
            name: name.into(),
            sources,
        };
        assert_eq!(
            games(&conn).unwrap(),
            [
                game("", vec![source("d.mp4", "d.mp4", 10.0, 720, None)]),
                game(
                    "Minecraft",
                    vec![source(
                        "Minecraft/2024/c.mp4",
                        "2024/c.mp4",
                        30.0,
                        1440,
                        None
                    )]
                ),
                game(
                    "PEAK",
                    vec![
                        source("PEAK/a.mp4", "a.mp4", 60.0, 1080, None),
                        source("PEAK/b.mp4", "b.mp4", 90.5, 1440, Some("skip")),
                    ]
                ),
            ]
        );
    }
}
