use rusqlite::{Connection, OptionalExtension};

pub const LIBRARY: &str = "library";

pub fn get(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT value FROM setting WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .optional()
}

pub fn set(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO setting (key, value) VALUES (?1, ?2) ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn an_unset_setting_is_none() {
        assert_eq!(get(&db::memory(), LIBRARY).unwrap(), None);
    }

    #[test]
    fn setting_again_replaces_the_value() {
        let conn = db::memory();
        set(&conn, LIBRARY, r"Z:\Old").unwrap();
        set(&conn, LIBRARY, r"Z:\Shadowplays").unwrap();
        assert_eq!(
            get(&conn, LIBRARY).unwrap().as_deref(),
            Some(r"Z:\Shadowplays")
        );
    }
}
