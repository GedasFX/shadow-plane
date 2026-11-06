use crate::clip_name;
use crate::library;
use crate::review::{Error, file_of};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub struct Job {
    pub id: i64,
    pub status: String,
    // The video the clip is cut from.
    pub key: String,
    pub title: String,
    pub game: String,
    pub file_name: String,
    // The pieces' total length.
    pub kept_s: f64,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Queue {
    pub running: Option<Job>,
    pub pending: Vec<Job>,
    pub done: Vec<Job>,
}

const DONE_SHOWN: i64 = 5;

// Last in line.
pub fn queue_clip(conn: &Connection, clip_id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO job (clip_id, position) VALUES (?1, (SELECT coalesce(max(position), -1) + 1 FROM job))",
        [clip_id],
    )?;
    Ok(())
}

const JOBS: &str = "SELECT j.id, j.status, c.decision_source, c.title, c.game, c.great, c.best,
        s.modified_ms,
        (SELECT coalesce(sum(end_s - start_s), 0) FROM segment WHERE clip_id = c.id),
        (SELECT start_s FROM segment WHERE clip_id = c.id ORDER BY position LIMIT 1),
        j.output, j.error
     FROM job j JOIN clip c ON c.id = j.clip_id JOIN source s ON s.path = c.decision_source";

fn job(row: &rusqlite::Row) -> rusqlite::Result<Job> {
    let key: String = row.get(2)?;
    let title: Option<String> = row.get(3)?;
    let game: Option<String> = row.get(4)?;
    let title = title.unwrap_or_default();
    let game = game.unwrap_or_else(|| library::game(&key));
    let start_s: Option<f64> = row.get(9)?;

    Ok(Job {
        id: row.get(0)?,
        status: row.get(1)?,
        file_name: clip_name::file_name(
            &game,
            &title,
            row.get(5)?,
            row.get(6)?,
            file_of(&key),
            row.get(7)?,
            start_s.unwrap_or(0.0),
        ),
        kept_s: row.get(8)?,
        output: row.get(10)?,
        error: row.get(11)?,
        key,
        title,
        game,
    })
}

fn jobs(conn: &Connection, filter: &str) -> rusqlite::Result<Vec<Job>> {
    let mut rows = conn.prepare(&format!("{JOBS} {filter}"))?;
    let rows = rows.query_map([], job)?;

    rows.collect()
}

pub fn queue(conn: &Connection) -> rusqlite::Result<Queue> {
    let running = jobs(
        conn,
        "WHERE j.status = 'running' ORDER BY j.position LIMIT 1",
    )?;
    let pending = jobs(conn, "WHERE j.status = 'pending' ORDER BY j.position")?;
    let done = jobs(
        conn,
        &format!(
            "WHERE j.status IN ('done', 'failed') ORDER BY j.finished_at DESC, j.id DESC LIMIT {DONE_SHOWN}"
        ),
    )?;

    Ok(Queue {
        running: running.into_iter().next(),
        pending,
        done,
    })
}

fn pending_ids(conn: &Connection) -> rusqlite::Result<Vec<i64>> {
    let mut rows = conn.prepare("SELECT id FROM job WHERE status = 'pending' ORDER BY position")?;
    let ids = rows.query_map([], |row| row.get(0))?;

    ids.collect()
}

// The ids must be exactly the pending jobs.
pub fn reorder(conn: &mut Connection, ids: &[i64]) -> Result<(), Error> {
    let tx = conn.transaction()?;
    let mut now = pending_ids(&tx)?;
    let mut wanted = ids.to_vec();
    now.sort_unstable();
    wanted.sort_unstable();
    if now != wanted {
        return Err(Error::Invalid("job.order.mismatch"));
    }

    // Spread over the pending jobs' own positions, so the running and finished ones keep theirs.
    let mut slots: Vec<i64> = {
        let mut rows = tx.prepare("SELECT position FROM job WHERE status = 'pending'")?;
        let slots = rows.query_map([], |row| row.get(0))?;
        slots.collect::<rusqlite::Result<_>>()?
    };
    slots.sort_unstable();
    for (id, position) in ids.iter().zip(slots) {
        tx.execute("UPDATE job SET position = ?2 WHERE id = ?1", (id, position))?;
    }

    Ok(tx.commit()?)
}

pub fn remove(conn: &Connection, id: i64) -> Result<(), Error> {
    let changed = conn.execute(
        "UPDATE job SET status = 'cancelled' WHERE id = ?1 AND status = 'pending'",
        [id],
    )?;
    if changed == 0 {
        return Err(Error::Invalid("job.status.not_pending"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::review::{self, ClipInput, Piece};

    const KEY: &str = "PEAK/PEAK 2024.02.20 - 23.46.01.02.DVR.mp4";

    fn library() -> Connection {
        let conn = db::memory();
        conn.execute_batch(&format!(
            "INSERT INTO source (path, size, modified_ms, duration_s, width, height, fps) VALUES ('{KEY}', 500, 1, 60, 2560, 1440, 59.94);"
        ))
        .unwrap();
        conn
    }

    fn save(conn: &mut Connection, title: &str, pieces: &[(f64, f64)]) {
        let clip = ClipInput {
            title: title.into(),
            game: "PEAK".into(),
            great: false,
            best: false,
            pieces: pieces
                .iter()
                .map(|&(start_s, end_s)| Piece { start_s, end_s })
                .collect(),
        };
        review::save(conn, KEY, &clip, "2026-11-06T10:00:00Z").unwrap();
    }

    fn titles(jobs: &[Job]) -> Vec<&str> {
        jobs.iter().map(|j| j.title.as_str()).collect()
    }

    fn ids(conn: &Connection) -> Vec<i64> {
        pending_ids(conn).unwrap()
    }

    #[test]
    fn saving_a_clip_queues_a_job_last_in_line() {
        let mut conn = library();
        save(&mut conn, "one", &[(1.0, 3.0), (10.0, 14.0)]);
        save(&mut conn, "two", &[(5.0, 6.0)]);

        let q = queue(&conn).unwrap();

        assert_eq!(titles(&q.pending), ["one", "two"]);
        let first = &q.pending[0];
        assert_eq!((first.status.as_str(), first.kept_s), ("pending", 6.0));
        assert_eq!((first.key.as_str(), first.game.as_str()), (KEY, "PEAK"));
        assert!(first.file_name.contains("one"));
        assert_eq!((&first.output, &first.error), (&None, &None));
        assert!(q.running.is_none() && q.done.is_empty());
    }

    #[test]
    fn reorder_sets_the_pending_order() {
        let mut conn = library();
        save(&mut conn, "a", &[(1.0, 2.0)]);
        save(&mut conn, "b", &[(1.0, 2.0)]);
        save(&mut conn, "c", &[(1.0, 2.0)]);
        let [a, b, c] = ids(&conn)[..] else { panic!() };

        reorder(&mut conn, &[c, a, b]).unwrap();

        assert_eq!(titles(&queue(&conn).unwrap().pending), ["c", "a", "b"]);
    }

    #[test]
    fn reorder_refuses_a_list_that_is_not_the_pending_set() {
        let mut conn = library();
        save(&mut conn, "a", &[(1.0, 2.0)]);
        save(&mut conn, "b", &[(1.0, 2.0)]);
        let [a, b] = ids(&conn)[..] else { panic!() };
        conn.execute("UPDATE job SET status = 'running' WHERE id = ?1", [a])
            .unwrap();

        for bad in [vec![a, b], vec![], vec![b, b], vec![b, 99]] {
            match reorder(&mut conn, &bad) {
                Err(Error::Invalid(code)) => assert_eq!(code, "job.order.mismatch"),
                other => panic!("expected a refusal, got {other:?}"),
            }
        }
        reorder(&mut conn, &[b]).unwrap();
    }

    #[test]
    fn remove_cancels_a_pending_job_only() {
        let mut conn = library();
        save(&mut conn, "a", &[(1.0, 2.0)]);
        save(&mut conn, "b", &[(1.0, 2.0)]);
        let [a, b] = ids(&conn)[..] else { panic!() };
        conn.execute("UPDATE job SET status = 'running' WHERE id = ?1", [a])
            .unwrap();

        remove(&conn, b).unwrap();

        let q = queue(&conn).unwrap();
        assert!(q.pending.is_empty() && q.done.is_empty());
        assert_eq!(q.running.unwrap().id, a);
        for id in [a, b] {
            match remove(&conn, id) {
                Err(Error::Invalid(code)) => assert_eq!(code, "job.status.not_pending"),
                other => panic!("expected a refusal, got {other:?}"),
            }
        }
    }

    #[test]
    fn done_lists_the_last_five_finished_newest_first() {
        let mut conn = library();
        for i in 0..7 {
            save(&mut conn, &format!("c{i}"), &[(1.0, 2.0)]);
        }
        conn.execute_batch(
            "UPDATE job SET status = 'done', output = 'out/x.mp4', finished_at = '2026-11-06T1' || id WHERE id <= 5;
             UPDATE job SET status = 'failed', error = 'boom', finished_at = '2026-11-06T1' || id WHERE id = 6;
             UPDATE job SET status = 'cancelled' WHERE id = 7;",
        )
        .unwrap();

        let q = queue(&conn).unwrap();

        assert_eq!(titles(&q.done), ["c5", "c4", "c3", "c2", "c1"]);
        assert_eq!(q.done[0].error.as_deref(), Some("boom"));
        assert_eq!(q.done[1].output.as_deref(), Some("out/x.mp4"));
    }

    #[test]
    fn discarding_the_video_deletes_its_jobs() {
        let mut conn = library();
        save(&mut conn, "a", &[(1.0, 2.0)]);

        review::discard(&conn, KEY, "2026-11-06T11:00:00Z").unwrap();

        let left: i64 = conn
            .query_row("SELECT count(*) FROM job", [], |row| row.get(0))
            .unwrap();
        assert_eq!(left, 0);
    }
}
