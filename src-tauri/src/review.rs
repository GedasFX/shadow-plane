use crate::clip_name;
use crate::jobs;
use crate::library;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug)]
pub enum Error {
    // A house code the page can show or test for: <entity>.<property>.<rule>.
    Invalid(&'static str),
    Db(rusqlite::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Invalid(code) => f.write_str(code),
            Error::Db(e) => e.fmt(f),
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Db(e)
    }
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Review,
    Saved,
    Discarded,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Video {
    pub key: String,
    pub game: String,
    // Local time, "2024-02-20T23:46:01".
    pub date: String,
    pub duration_s: f64,
    pub size: i64,
    pub status: Status,
    pub saved_clips: i64,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct VideoDetail {
    #[serde(flatten)]
    pub video: Video,
    pub width: i64,
    pub height: i64,
    pub fps: f64,
    pub clips: Vec<SavedClip>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct SavedClip {
    pub title: String,
    pub game: String,
    pub great: bool,
    pub best: bool,
    pub pieces: Vec<Piece>,
    pub file_name: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Piece {
    pub start_s: f64,
    pub end_s: f64,
}

#[derive(Debug, Deserialize)]
pub struct ClipInput {
    pub title: String,
    pub game: String,
    pub great: bool,
    pub best: bool,
    pub pieces: Vec<Piece>,
}

const VIDEOS: &str = "SELECT s.path, s.size, s.modified_ms, s.duration_s, s.width, s.height, s.fps,
        d.action, (SELECT count(*) FROM clip c WHERE c.decision_source = s.path)
     FROM source s LEFT JOIN decision d ON d.source = s.path";

pub fn file_of(key: &str) -> &str {
    key.rsplit_once('/').map_or(key, |(_, file)| file)
}

// Only a queue or a reject decision settles a video; a legacy skip is still to review.
fn video(row: &rusqlite::Row) -> rusqlite::Result<(Video, i64, i64, f64)> {
    let key: String = row.get(0)?;
    let modified_ms = row.get(2)?;
    let action: Option<String> = row.get(7)?;
    let status = match action.as_deref() {
        Some("queue") => Status::Saved,
        Some("reject") => Status::Discarded,
        _ => Status::Review,
    };
    let (at, _) = clip_name::recorded(file_of(&key), modified_ms);

    let video = Video {
        game: library::game(&key),
        date: at.format("%Y-%m-%dT%H:%M:%S").to_string(),
        key,
        size: row.get(1)?,
        duration_s: row.get(3)?,
        status,
        saved_clips: row.get(8)?,
    };
    Ok((video, row.get(4)?, row.get(5)?, row.get(6)?))
}

pub fn videos(conn: &Connection) -> rusqlite::Result<Vec<Video>> {
    let mut rows = conn.prepare(&format!("{VIDEOS} ORDER BY s.path"))?;
    let rows = rows.query_map([], |row| Ok(video(row)?.0))?;

    rows.collect()
}

// None when the key is not in the index.
pub fn detail(conn: &Connection, key: &str) -> rusqlite::Result<Option<VideoDetail>> {
    let found = conn
        .query_row(&format!("{VIDEOS} WHERE s.path = ?1"), [key], video)
        .optional()?;
    let Some((video, width, height, fps)) = found else {
        return Ok(None);
    };
    let clips = saved_clips(conn, &video)?;

    Ok(Some(VideoDetail {
        video,
        width,
        height,
        fps,
        clips,
    }))
}

fn saved_clips(conn: &Connection, video: &Video) -> rusqlite::Result<Vec<SavedClip>> {
    let modified_ms: i64 = conn.query_row(
        "SELECT modified_ms FROM source WHERE path = ?1",
        [&video.key],
        |row| row.get(0),
    )?;
    let mut clips = conn.prepare(
        "SELECT id, title, game, great, best FROM clip WHERE decision_source = ?1 ORDER BY position",
    )?;
    let mut pieces =
        conn.prepare("SELECT start_s, end_s FROM segment WHERE clip_id = ?1 ORDER BY position")?;

    let rows = clips.query_map([&video.key], |row| {
        let id: i64 = row.get(0)?;
        let game: Option<String> = row.get(2)?;
        Ok((
            id,
            row.get::<_, Option<String>>(1)?,
            game,
            row.get(3)?,
            row.get(4)?,
        ))
    })?;

    let mut saved = Vec::new();
    for row in rows {
        let (id, title, game, great, best): (i64, Option<String>, Option<String>, bool, bool) =
            row?;
        let pieces: Vec<Piece> = pieces
            .query_map([id], |row| {
                Ok(Piece {
                    start_s: row.get(0)?,
                    end_s: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;

        let title = title.unwrap_or_default();
        let game = game.unwrap_or_else(|| video.game.clone());
        let start_s = pieces.first().map_or(0.0, |piece| piece.start_s);
        saved.push(SavedClip {
            file_name: clip_name::file_name(
                &game,
                &title,
                great,
                best,
                file_of(&video.key),
                modified_ms,
                start_s,
            ),
            title,
            game,
            great,
            best,
            pieces,
        });
    }

    Ok(saved)
}

// The name a clip will be saved under, for the page to show while the clip is edited.
pub fn clip_file_name(conn: &Connection, key: &str, clip: &ClipInput) -> Result<String, Error> {
    let modified_ms: Option<i64> = conn
        .query_row(
            "SELECT modified_ms FROM source WHERE path = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()?;
    let modified_ms = modified_ms.ok_or(Error::Invalid("video.key.unknown"))?;
    let start_s = clip.pieces.first().map_or(0.0, |piece| piece.start_s);

    Ok(clip_name::file_name(
        &clip.game,
        &clip.title,
        clip.great,
        clip.best,
        file_of(key),
        modified_ms,
        start_s,
    ))
}

// Appends the clip to the video's queue decision: the first Save makes the decision, each later one adds a clip.
pub fn save(conn: &mut Connection, key: &str, clip: &ClipInput, now: &str) -> Result<(), Error> {
    let duration_s: Option<f64> = conn
        .query_row(
            "SELECT duration_s FROM source WHERE path = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()?;
    let duration_s = duration_s.ok_or(Error::Invalid("video.key.unknown"))?;
    if clip_name::sanitize(&clip.title).is_empty() {
        return Err(Error::Invalid("clip.title.empty"));
    }
    if clip_name::sanitize(&clip.game).is_empty() {
        return Err(Error::Invalid("clip.game.empty"));
    }
    if clip.pieces.is_empty() {
        return Err(Error::Invalid("clip.pieces.empty"));
    }
    if clip
        .pieces
        .iter()
        .any(|p| p.start_s < 0.0 || p.end_s <= p.start_s || p.end_s > duration_s)
    {
        return Err(Error::Invalid("clip.pieces.range"));
    }

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO decision (source, action, decided_at) VALUES (?1, 'queue', ?2)
         ON CONFLICT (source) DO UPDATE SET action = 'queue', decided_at = excluded.decided_at",
        (key, now),
    )?;
    tx.execute(
        "INSERT INTO clip (decision_source, position, title, game, blend, great, best)
         VALUES (?1, (SELECT count(*) FROM clip WHERE decision_source = ?1), ?2, ?3, 'hard', ?4, ?5)",
        (key, clip.title.trim(), clip.game.trim(), clip.great, clip.best),
    )?;
    let clip_id = tx.last_insert_rowid();
    for (position, piece) in clip.pieces.iter().enumerate() {
        tx.execute(
            "INSERT INTO segment (clip_id, position, source, start_s, end_s) VALUES (?1, ?2, ?3, ?4, ?5)",
            (clip_id, position as i64, key, piece.start_s, piece.end_s),
        )?;
    }

    jobs::queue_clip(&tx, clip_id)?;

    Ok(tx.commit()?)
}

// Replaces any decision, so the clips saved before go with it. The file move is stage 4's.
pub fn discard(conn: &Connection, key: &str, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO decision (source, action, decided_at) VALUES (?1, 'reject', ?2)",
        (key, now),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const NOW: &str = "2026-11-06T10:00:00Z";

    // Two 60 s videos; the first is named the ShadowPlay way.
    fn library() -> Connection {
        let conn = db::memory();
        conn.execute_batch(
            "INSERT INTO source VALUES ('PEAK/PEAK 2024.02.20 - 23.46.01.02.DVR.mp4', 500, 1, 60, 2560, 1440, 59.94);
             INSERT INTO source VALUES ('loose.mp4', 700, 1, 30, 1920, 1080, 30);",
        )
        .unwrap();
        conn
    }

    const PEAK: &str = "PEAK/PEAK 2024.02.20 - 23.46.01.02.DVR.mp4";

    fn clip(title: &str, pieces: &[(f64, f64)]) -> ClipInput {
        ClipInput {
            title: title.into(),
            game: "PEAK".into(),
            great: false,
            best: false,
            pieces: pieces
                .iter()
                .map(|&(start_s, end_s)| Piece { start_s, end_s })
                .collect(),
        }
    }

    fn invalid(result: Result<(), Error>) -> &'static str {
        match result {
            Err(Error::Invalid(code)) => code,
            other => panic!("expected an invalid clip, got {other:?}"),
        }
    }

    #[test]
    fn the_list_gives_each_video_its_game_date_and_status() {
        let conn = library();
        conn.execute_batch(
            "INSERT INTO decision (source, action, decided_at) VALUES ('loose.mp4', 'reject', '2026-11-06');",
        )
        .unwrap();

        let list = videos(&conn).unwrap();

        assert_eq!(list[0].key, PEAK);
        assert_eq!(list[0].game, "PEAK");
        assert_eq!(list[0].date, "2024-02-20T23:46:01");
        assert_eq!((list[0].size, list[0].duration_s), (500, 60.0));
        assert_eq!(list[0].status, Status::Review);
        assert_eq!(
            (list[1].game.as_str(), &list[1].status),
            ("Misc", &Status::Discarded)
        );
    }

    #[test]
    fn a_legacy_skip_is_still_to_review() {
        let conn = library();
        conn.execute_batch(
            "INSERT INTO decision (source, action, decided_at) VALUES ('loose.mp4', 'skip', '2026-11-06');",
        )
        .unwrap();

        assert_eq!(videos(&conn).unwrap()[1].status, Status::Review);
    }

    #[test]
    fn saving_twice_adds_a_second_clip_with_its_pieces_in_order() {
        let mut conn = library();
        let mut first = clip("Big jump", &[(20.0, 30.0), (5.0, 8.0)]);
        first.great = true;

        save(&mut conn, PEAK, &first, NOW).unwrap();
        save(&mut conn, PEAK, &clip("Fall", &[(40.0, 50.5)]), NOW).unwrap();

        let video = detail(&conn, PEAK).unwrap().unwrap();
        assert_eq!(
            (&video.video.status, video.video.saved_clips),
            (&Status::Saved, 2)
        );
        assert_eq!((video.width, video.height, video.fps), (2560, 1440, 59.94));
        assert_eq!(video.clips[0].title, "Big jump");
        assert!(video.clips[0].great && !video.clips[0].best);
        assert_eq!(
            video.clips[0].pieces,
            [
                Piece {
                    start_s: 20.0,
                    end_s: 30.0
                },
                Piece {
                    start_s: 5.0,
                    end_s: 8.0
                }
            ]
        );
        assert_eq!(video.clips[1].title, "Fall");
        assert_eq!(
            video.clips[0].file_name,
            "PEAK - 2024.02.20 - 23.46.21.02 - Big jump \u{2b50}.mp4"
        );
    }

    #[test]
    fn a_clip_is_refused_without_a_title_game_or_sound_pieces() {
        let mut conn = library();
        let mut save_it = |clip: ClipInput| invalid(save(&mut conn, PEAK, &clip, NOW));

        assert_eq!(save_it(clip(" .. ", &[(0.0, 1.0)])), "clip.title.empty");
        let mut no_game = clip("A", &[(0.0, 1.0)]);
        no_game.game = " ".into();
        assert_eq!(save_it(no_game), "clip.game.empty");
        assert_eq!(save_it(clip("A", &[])), "clip.pieces.empty");
        assert_eq!(save_it(clip("A", &[(5.0, 5.0)])), "clip.pieces.range");
        assert_eq!(save_it(clip("A", &[(-1.0, 5.0)])), "clip.pieces.range");
        assert_eq!(save_it(clip("A", &[(50.0, 61.0)])), "clip.pieces.range");
        assert_eq!(videos(&conn).unwrap()[0].saved_clips, 0);
    }

    #[test]
    fn a_clip_of_an_unknown_video_is_refused() {
        let mut conn = library();
        let result = save(&mut conn, "gone.mp4", &clip("A", &[(0.0, 1.0)]), NOW);
        assert_eq!(invalid(result), "video.key.unknown");
    }

    #[test]
    fn discarding_replaces_the_decision_and_its_saved_clips() {
        let mut conn = library();
        save(&mut conn, PEAK, &clip("A", &[(0.0, 1.0)]), NOW).unwrap();

        discard(&conn, PEAK, NOW).unwrap();

        let video = detail(&conn, PEAK).unwrap().unwrap();
        assert_eq!(
            (&video.video.status, video.video.saved_clips),
            (&Status::Discarded, 0)
        );
        assert!(video.clips.is_empty());
    }

    #[test]
    fn saving_a_discarded_video_makes_it_saved_again() {
        let mut conn = library();
        discard(&conn, PEAK, NOW).unwrap();

        save(&mut conn, PEAK, &clip("A", &[(0.0, 1.0)]), NOW).unwrap();

        assert_eq!(videos(&conn).unwrap()[0].status, Status::Saved);
    }

    #[test]
    fn the_file_name_is_shown_before_the_clip_is_saved() {
        let conn = library();
        let mut best = clip("Jump", &[(10.0, 20.0)]);
        best.best = true;

        let name = clip_file_name(&conn, PEAK, &best).unwrap();

        assert_eq!(name, "PEAK - 2024.02.20 - 23.46.11.02 - Jump \u{1f308}.mp4");
    }
}
