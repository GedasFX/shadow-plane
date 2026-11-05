use rusqlite::Connection;
use std::path::Path;
use std::time::Duration;

// In order. Each runs once, in a transaction; PRAGMA user_version counts how many have run.
const MIGRATIONS: &[&str] = &[include_str!(
    "../migrations/001_settings_quality_decisions.sql"
)];

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    migrate(&mut conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> rusqlite::Result<()> {
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.busy_timeout(Duration::from_secs(5))
}

fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    let done: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(done as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        configure(&conn).unwrap();
        migrate(&mut conn).unwrap();
        conn
    }

    fn version(conn: &Connection) -> i64 {
        conn.pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn migrating_again_changes_nothing() {
        let mut conn = memory();
        migrate(&mut conn).unwrap();
        assert_eq!(version(&conn), MIGRATIONS.len() as i64);
    }

    #[test]
    fn a_profile_is_one_per_encoder_game_tier_and_band() {
        let conn = memory();
        let add = |encoder: &str| {
            conn.execute(
                "INSERT INTO quality_profile (encoder, game, tier, bpp_band) VALUES (?1, 'PEAK', 1440, 'lean')",
                [encoder],
            )
        };
        add("svt-av1").unwrap();
        assert!(add("svt-av1").is_err());
        add("nvenc-av1").unwrap();
    }

    #[test]
    fn deleting_a_decision_deletes_its_clips_and_segments() {
        let conn = memory();
        conn.execute_batch(
            "INSERT INTO decision (source, action, decided_at) VALUES ('PEAK/a.mp4', 'queue', '2026-11-05');
             INSERT INTO clip (id, decision_source, position, blend) VALUES (1, 'PEAK/a.mp4', 0, 'hard');
             INSERT INTO segment (clip_id, position, source, start_s, end_s) VALUES (1, 0, 'PEAK/a.mp4', 1, 2);
             DELETE FROM decision;",
        )
        .unwrap();
        let left: i64 = conn
            .query_row(
                "SELECT (SELECT count(*) FROM clip) + (SELECT count(*) FROM segment)",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(left, 0);
    }

    #[test]
    fn a_decision_is_queue_skip_or_reject() {
        let conn = memory();
        let decide = |action: &str| {
            conn.execute(
                "INSERT OR REPLACE INTO decision (source, action, decided_at) VALUES ('a.mp4', ?1, '2026-11-05')",
                [action],
            )
        };
        decide("reject").unwrap();
        assert!(decide("maybe").is_err());
    }
}
